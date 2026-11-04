use glam::Vec3;

pub const BOX_UNITS: u8 = 16;
pub const MAX_BOXES: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct VoxelBox {
    pub min: [u8; 3],
    pub max: [u8; 3],
}

impl VoxelBox {
    pub const FULL: VoxelBox = VoxelBox::new([0, 0, 0], [BOX_UNITS; 3]);

    pub const fn new(min: [u8; 3], max: [u8; 3]) -> Self {
        Self { min, max }
    }

    /// Поворот по часовой стрелке сверху вокруг центра блока, как у `FACING_*`.
    pub fn turned(self, turns: u8) -> Self {
        let mut out = self;
        for _ in 0..turns % 4 {
            out = VoxelBox::new(
                [BOX_UNITS - out.max[2], out.min[1], out.min[0]],
                [BOX_UNITS - out.min[2], out.max[1], out.max[0]],
            );
        }
        out
    }

    pub fn flipped_y(self) -> Self {
        VoxelBox::new(
            [self.min[0], BOX_UNITS - self.max[1], self.min[2]],
            [self.max[0], BOX_UNITS - self.min[1], self.max[2]],
        )
    }

    pub fn bounds(self) -> (Vec3, Vec3) {
        let unit = 1.0 / f32::from(BOX_UNITS);
        (
            Vec3::from(self.min.map(f32::from)) * unit,
            Vec3::from(self.max.map(f32::from)) * unit,
        )
    }

    pub fn is_flat(self) -> bool {
        (0..3).any(|axis| self.min[axis] >= self.max[axis])
    }
}

/// Список коробок блока в 1/16 блока; не больше `MAX_BOXES`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct BoxList {
    items: [VoxelBox; MAX_BOXES],
    len: u8,
}

impl BoxList {
    pub const EMPTY: BoxList = BoxList {
        items: [VoxelBox::new([0; 3], [0; 3]); MAX_BOXES],
        len: 0,
    };

    pub fn of(boxes: &[VoxelBox]) -> Self {
        let mut list = Self::EMPTY;
        for &b in boxes {
            list.push(b);
        }
        list
    }

    pub fn push(&mut self, b: VoxelBox) {
        assert!(usize::from(self.len) < MAX_BOXES, "too many boxes");
        self.items[usize::from(self.len)] = b;
        self.len += 1;
    }

    pub fn as_slice(&self) -> &[VoxelBox] {
        &self.items[..usize::from(self.len)]
    }

    pub fn iter(&self) -> impl Iterator<Item = &VoxelBox> {
        self.as_slice().iter()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    pub fn map(&self, f: impl Fn(VoxelBox) -> VoxelBox) -> Self {
        let mut out = Self::EMPTY;
        for &b in self.as_slice() {
            out.push(f(b));
        }
        out
    }

    pub fn top(&self) -> f32 {
        let units = self.iter().map(|b| b.max[1]).max().unwrap_or(0);
        f32::from(units) / f32::from(BOX_UNITS)
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        let (mut min, mut max) = (Vec3::ONE, Vec3::ZERO);
        for b in self.iter() {
            let (lo, hi) = b.bounds();
            min = min.min(lo);
            max = max.max(hi);
        }
        (min.min(max), max)
    }

    pub fn face_mask(&self, axis: usize, positive: bool) -> FaceMask {
        let (u, v) = face_axes(axis);
        let mut mask = FaceMask::EMPTY;
        for b in self.iter() {
            let on_plane = if positive {
                b.max[axis] == BOX_UNITS
            } else {
                b.min[axis] == 0
            };
            if on_plane && !b.is_flat() {
                mask.fill(b.min[u], b.max[u], b.min[v], b.max[v]);
            }
        }
        mask
    }
}

/// Оси грани по нормали: плоскость `u` x `v`, как в упаковке квада.
pub const fn face_axes(axis: usize) -> (usize, usize) {
    match axis {
        0 => (2, 1),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// Покрытие грани блока сеткой 16x16: бит `u` в строке `v`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FaceMask(pub [u16; BOX_UNITS as usize]);

impl FaceMask {
    pub const EMPTY: FaceMask = FaceMask([0; BOX_UNITS as usize]);
    pub const FULL: FaceMask = FaceMask([u16::MAX; BOX_UNITS as usize]);

    fn row_bits(u0: u8, u1: u8) -> u16 {
        let width = u32::from(u1.saturating_sub(u0));
        if width == 0 {
            return 0;
        }
        (((1u32 << width) - 1) << u0) as u16
    }

    pub fn rect(u0: u8, u1: u8, v0: u8, v1: u8) -> Self {
        let mut mask = Self::EMPTY;
        mask.fill(u0, u1, v0, v1);
        mask
    }

    pub fn fill(&mut self, u0: u8, u1: u8, v0: u8, v1: u8) {
        let bits = Self::row_bits(u0, u1);
        for row in &mut self.0[usize::from(v0)..usize::from(v1)] {
            *row |= bits;
        }
    }

    pub fn subtract(&mut self, other: &FaceMask) {
        for (row, cover) in self.0.iter_mut().zip(other.0) {
            *row &= !cover;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&row| row == 0)
    }

    pub fn contains(&self, other: &FaceMask) -> bool {
        self.0
            .iter()
            .zip(other.0)
            .all(|(row, need)| need & !row == 0)
    }

    /// Разбивка на прямоугольники `(u0, u1, v0, v1)`, строка за строкой.
    pub fn rects(&self) -> Vec<(u8, u8, u8, u8)> {
        let mut rows = self.0;
        let mut out = Vec::new();
        for v in 0..BOX_UNITS {
            let mut row = rows[usize::from(v)];
            while row != 0 {
                let u0 = row.trailing_zeros() as u8;
                let run = (row >> u0).trailing_ones() as u8;
                let u1 = u0 + run;
                let bits = Self::row_bits(u0, u1);
                row &= !bits;
                rows[usize::from(v)] &= !bits;
                let mut v1 = v + 1;
                while v1 < BOX_UNITS && rows[usize::from(v1)] & bits == bits {
                    rows[usize::from(v1)] &= !bits;
                    v1 += 1;
                }
                out.push((u0, u1, v, v1));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;

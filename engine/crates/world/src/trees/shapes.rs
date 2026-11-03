use glam::IVec3;

use super::{Rng, SIDES};
use crate::block::{AXIS_Y, VINE_NEG_X, VINE_NEG_Z, VINE_POS_X, VINE_POS_Z};

pub(super) type Log = (IVec3, u8);

pub(super) struct Vine {
    pub cell: IVec3,
    pub side: u8,
    pub drop: i32,
}

#[derive(Default)]
pub(super) struct Parts {
    pub logs: Vec<Log>,
    pub leaves: Vec<IVec3>,
    pub vines: Vec<Vine>,
}

const HANGING_VINE_DROP: i32 = 4;
const MEGA_BRANCH_STEPS: i32 = 5;
const CANOPY_TRUNK_TIERS: i32 = 3;

const HORIZONTAL_SIDES: [(i32, i32, u8); 4] = [
    (1, 0, VINE_NEG_X),
    (-1, 0, VINE_POS_X),
    (0, 1, VINE_NEG_Z),
    (0, -1, VINE_POS_Z),
];

fn column(base: IVec3, height: i32) -> Vec<Log> {
    (0..height)
        .map(|dy| (base + IVec3::Y * dy, AXIS_Y))
        .collect()
}

fn log(pos: IVec3) -> Log {
    (pos, AXIS_Y)
}

pub(super) fn trunk_vines(rng: &mut Rng, cells: &[IVec3], from_y: i32, to_y: i32) -> Vec<Vine> {
    let mut vines = Vec::new();
    for y in from_y..to_y {
        for &trunk in cells {
            for (dx, dz, side) in HORIZONTAL_SIDES {
                if rng.below(3) > 0 {
                    vines.push(Vine {
                        cell: IVec3::new(trunk.x + dx, y, trunk.z + dz),
                        side,
                        drop: 0,
                    });
                }
            }
        }
    }
    vines
}

pub(super) fn leaf_vines(rng: &mut Rng, leaves: &[IVec3]) -> Vec<Vine> {
    let mut vines = Vec::new();
    for &leaf in leaves {
        for (dx, dz, side) in HORIZONTAL_SIDES {
            if rng.below(4) == 0 {
                vines.push(Vine {
                    cell: leaf + IVec3::new(dx, 0, dz),
                    side,
                    drop: HANGING_VINE_DROP,
                });
            }
        }
    }
    vines
}

pub(super) fn jungle_tree(base: IVec3, height: i32, shape: u32, leaves: Vec<IVec3>) -> Parts {
    let mut rng = Rng(u64::from(shape) ^ 0x6a_17);
    let logs = column(base, height);
    let mut vines = trunk_vines(&mut rng, &[base], base.y + 1, base.y + height);
    let crown: Vec<IVec3> = leaves
        .iter()
        .copied()
        .filter(|l| l.y >= base.y + height - 3)
        .collect();
    vines.extend(leaf_vines(&mut rng, &crown));
    Parts {
        logs,
        leaves,
        vines,
    }
}

pub(super) fn swamp_oak(base: IVec3, height: i32, shape: u32) -> Parts {
    let mut rng = Rng(u64::from(shape) ^ 0x7c_41);
    let top = base.y + height;
    let mut leaves = Vec::new();
    for y in top - 3..=top {
        let layer = y - top;
        let reach = 2 - layer / 2;
        for dz in -reach..=reach {
            for dx in -reach..=reach {
                let corner = dx.abs() == reach && dz.abs() == reach;
                if !corner || (layer != 0 && rng.below(2) != 0) {
                    leaves.push(IVec3::new(base.x + dx, y, base.z + dz));
                }
            }
        }
    }
    let vines = leaf_vines(&mut rng, &leaves);
    Parts {
        logs: column(base, height),
        leaves,
        vines,
    }
}

pub(super) fn shrub(base: IVec3, shape: u32) -> Parts {
    let mut rng = Rng(u64::from(shape) ^ 0x5b_2b);
    let mut leaves = Vec::new();
    for dy in 0i32..=2 {
        let k = 2 - dy;
        for dz in -k..=k {
            for dx in -k..=k {
                if dx.abs() != k || dz.abs() != k || rng.below(2) == 0 {
                    leaves.push(base + IVec3::new(dx, dy, dz));
                }
            }
        }
    }
    Parts {
        logs: vec![log(base)],
        leaves,
        vines: Vec::new(),
    }
}

fn disk(leaves: &mut Vec<IVec3>, center: IVec3, radius: i32) {
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dz * dz <= radius * radius {
                leaves.push(center + IVec3::new(dx, 0, dz));
            }
        }
    }
}

fn wide_disk(leaves: &mut Vec<IVec3>, center: IVec3, radius: i32) {
    let r2 = radius * radius;
    for dz in -radius..=radius + 1 {
        for dx in -radius..=radius + 1 {
            let (ex, ez) = (dx - 1, dz - 1);
            if dx * dx + dz * dz <= r2
                || ex * ex + ez * ez <= r2
                || dx * dx + ez * ez <= r2
                || ex * ex + dz * dz <= r2
            {
                leaves.push(center + IVec3::new(dx, 0, dz));
            }
        }
    }
}

pub(super) fn mega_jungle(base: IVec3, height: i32, shape: u32) -> Parts {
    let mut rng = Rng(u64::from(shape) ^ 0x3e_9a);
    let mut parts = Parts::default();
    let top = base.y + height;
    for j in -2i32..=0 {
        wide_disk(
            &mut parts.leaves,
            IVec3::new(base.x, top + j, base.z),
            3 - j,
        );
    }
    let mut y = top - 2 - rng.below(4);
    while y > base.y + height / 2 {
        let angle = rng.below(6283) as f32 / 1000.0;
        let (mut bx, mut bz) = (base.x, base.z);
        for step in 0..MEGA_BRANCH_STEPS {
            bx = base.x + (1.5 + angle.cos() * step as f32) as i32;
            bz = base.z + (1.5 + angle.sin() * step as f32) as i32;
            parts.logs.push(log(IVec3::new(bx, y - 3 + step / 2, bz)));
        }
        let blob = 1 + rng.below(2);
        for ly in y - blob..=y {
            disk(&mut parts.leaves, IVec3::new(bx, ly, bz), 1 - (ly - y));
        }
        y -= 2 + rng.below(4);
    }
    let mut outer = Vec::new();
    for dy in 0..height {
        for (ox, oz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            if (ox, oz) == (0, 0) || dy < height - 1 {
                parts
                    .logs
                    .push(log(IVec3::new(base.x + ox, base.y + dy, base.z + oz)));
            }
        }
    }
    for (ox, oz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        outer.push(IVec3::new(base.x + ox, 0, base.z + oz));
    }
    for y in base.y + 1..base.y + height {
        for &trunk in &outer {
            for (dx, dz, side) in HORIZONTAL_SIDES {
                let (cx, cz) = (trunk.x + dx, trunk.z + dz);
                let inside =
                    (base.x..=base.x + 1).contains(&cx) && (base.z..=base.z + 1).contains(&cz);
                if !inside && rng.below(3) > 0 {
                    parts.vines.push(Vine {
                        cell: IVec3::new(cx, y, cz),
                        side,
                        drop: 0,
                    });
                }
            }
        }
    }
    parts
}

pub(super) fn acacia(base: IVec3, height: i32, shape: u32) -> Parts {
    let mut rng = Rng(u64::from(shape) ^ 0xac_ac);
    let mut parts = Parts::default();
    let (dx, dz, _) = SIDES[rng.below(4) as usize];
    let bend_at = height - rng.below(4) - 1;
    let mut bend = 3 - rng.below(3);
    let (mut x, mut z) = (base.x, base.z);
    let mut top = base.y;
    for dy in 0..height {
        if dy >= bend_at && bend > 0 {
            x += dx;
            z += dz;
            bend -= 1;
        }
        top = base.y + dy;
        parts.logs.push(log(IVec3::new(x, top, z)));
    }
    let crown = IVec3::new(x, top, z);
    for oz in -3i32..=3 {
        for ox in -3i32..=3 {
            if ox.abs() != 3 || oz.abs() != 3 {
                parts.leaves.push(crown + IVec3::new(ox, 0, oz));
            }
        }
    }
    let upper = crown + IVec3::Y;
    for oz in -1i32..=1 {
        for ox in -1i32..=1 {
            parts.leaves.push(upper + IVec3::new(ox, 0, oz));
        }
    }
    for (ox, oz, _) in SIDES {
        parts.leaves.push(upper + IVec3::new(2 * ox, 0, 2 * oz));
    }

    let (bx, bz, _) = SIDES[rng.below(4) as usize];
    if (bx, bz) != (dx, dz) {
        let mut at = base.x;
        let mut az = base.z;
        let mut y = bend_at - rng.below(2) - 1;
        let mut steps = 1 + rng.below(3);
        let mut branch_top = None;
        while y < height && steps > 0 {
            if y >= 1 {
                at += bx;
                az += bz;
                let pos = IVec3::new(at, base.y + y, az);
                parts.logs.push(log(pos));
                branch_top = Some(pos);
            }
            y += 1;
            steps -= 1;
        }
        if let Some(tip) = branch_top {
            for oz in -2i32..=2 {
                for ox in -2i32..=2 {
                    if ox.abs() != 2 || oz.abs() != 2 {
                        parts.leaves.push(tip + IVec3::new(ox, 0, oz));
                    }
                }
            }
            let above = tip + IVec3::Y;
            for oz in -1i32..=1 {
                for ox in -1i32..=1 {
                    parts.leaves.push(above + IVec3::new(ox, 0, oz));
                }
            }
        }
    }
    parts
}

pub(super) fn dark_oak(base: IVec3, height: i32, shape: u32) -> Parts {
    let mut rng = Rng(u64::from(shape) ^ 0xda_4c);
    let mut parts = Parts::default();
    let (dx, dz, _) = SIDES[rng.below(4) as usize];
    let bend_at = height - rng.below(4);
    let mut bend = 2 - rng.below(3);
    let (mut x, mut z) = (base.x, base.z);
    let mut top = base.y;
    for dy in 0..height {
        if dy >= bend_at && bend > 0 {
            x += dx;
            z += dz;
            bend -= 1;
        }
        top = base.y + dy;
        for (ox, oz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            parts.logs.push(log(IVec3::new(x + ox, top, z + oz)));
        }
    }
    let mirrored = |leaves: &mut Vec<IVec3>, i: i32, y: i32, l: i32| {
        for (px, pz) in [
            (x + i, z + l),
            (x + 1 - i, z + l),
            (x + i, z + 1 - l),
            (x + 1 - i, z + 1 - l),
        ] {
            leaves.push(IVec3::new(px, y, pz));
        }
    };
    for i in -2i32..=0 {
        for l in -2i32..=0 {
            mirrored(&mut parts.leaves, i, top - 1, l);
            if (i > -2 || l > -1) && (i != -1 || l != -2) {
                mirrored(&mut parts.leaves, i, top + 1, l);
            }
        }
    }
    if rng.below(2) == 0 {
        for (ox, oz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            parts.leaves.push(IVec3::new(x + ox, top + 2, z + oz));
        }
    }
    for i in -3i32..=4 {
        for l in -3i32..=4 {
            let corner = (i == -3 || i == 4) && (l == -3 || l == 4);
            if !corner && (i.abs() < 3 || l.abs() < 3) {
                parts.leaves.push(IVec3::new(x + i, top, z + l));
            }
        }
    }
    for i in -1i32..=2 {
        for l in -1i32..=2 {
            let inner = (0..=1).contains(&i) && (0..=1).contains(&l);
            if inner || rng.below(3) != 0 {
                continue;
            }
            let drop = rng.below(3) + CANOPY_TRUNK_TIERS - 1;
            for k in 0..drop {
                parts
                    .logs
                    .push(log(IVec3::new(base.x + i, top - k - 1, base.z + l)));
            }
            for ly in [top - 3, top - 2] {
                let reach: i32 = if ly == top - 3 { 1 } else { 2 };
                for oz in -reach..=reach {
                    for ox in -reach..=reach {
                        if ox.abs() != 2 || oz.abs() != 2 {
                            parts
                                .leaves
                                .push(IVec3::new(base.x + i + ox, ly, base.z + l + oz));
                        }
                    }
                }
            }
        }
    }
    parts
}

pub(super) fn mushroom(base: IVec3, height: i32, red: bool) -> Parts {
    let mut parts = Parts {
        logs: column(base, height),
        ..Parts::default()
    };
    let top = base.y + height;
    if red {
        for y in top - 3..top {
            for dz in -2i32..=2 {
                for dx in -2i32..=2 {
                    let ring = dx.abs() == 2 || dz.abs() == 2;
                    let corner = dx.abs() == 2 && dz.abs() == 2;
                    if ring && !corner {
                        parts.leaves.push(IVec3::new(base.x + dx, y, base.z + dz));
                    }
                }
            }
        }
        for dz in -1i32..=1 {
            for dx in -1i32..=1 {
                parts.leaves.push(IVec3::new(base.x + dx, top, base.z + dz));
            }
        }
    } else {
        for dz in -3i32..=3 {
            for dx in -3i32..=3 {
                if dx.abs() != 3 || dz.abs() != 3 {
                    parts.leaves.push(IVec3::new(base.x + dx, top, base.z + dz));
                }
            }
        }
    }
    parts
}

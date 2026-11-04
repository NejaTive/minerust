use super::*;

const SALT_ORES: u64 = 110;
const SALT_EMERALDS: u64 = 120;
const SHIFT: f64 = 8.0;
const EMERALD_MIN_COUNT: u32 = 3;
const EMERALD_EXTRA_COUNT: u32 = 6;
const EMERALD_MIN_Y: u32 = 4;
const EMERALD_Y_SPAN: u32 = 28;
const MONSTER_EGGS: Vein = uniform(Block::MonsterEgg, 9, 7, 64);
const MESA_GOLD: Vein = Vein::new(Block::GoldOre, 9, 20, Height::Uniform { min: 32, max: 80 });

#[derive(Clone, Copy)]
enum Height {
    Uniform { min: u32, max: u32 },
    Triangle { center: u32, spread: u32 },
}

impl Height {
    fn pick(self, rng: &mut Stream) -> i32 {
        match self {
            Height::Uniform { min, max } => (rng.below(max - min) + min) as i32,
            Height::Triangle { center, spread } => {
                (rng.below(spread) + rng.below(spread) + center - spread) as i32
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Vein {
    block: Block,
    size: u32,
    count: u32,
    height: Height,
}

impl Vein {
    const fn new(block: Block, size: u32, count: u32, height: Height) -> Self {
        Self {
            block,
            size,
            count,
            height,
        }
    }
}

const fn uniform(block: Block, size: u32, count: u32, max: u32) -> Vein {
    Vein::new(block, size, count, Height::Uniform { min: 0, max })
}

const VEINS: [Vein; 11] = [
    uniform(Block::Dirt, 33, 10, 256),
    uniform(Block::Gravel, 33, 8, 256),
    uniform(Block::Granite, 33, 10, 80),
    uniform(Block::Diorite, 33, 10, 80),
    uniform(Block::Andesite, 33, 10, 80),
    uniform(Block::CoalOre, 17, 20, 128),
    uniform(Block::IronOre, 9, 20, 64),
    uniform(Block::GoldOre, 9, 2, 32),
    uniform(Block::RedstoneOre, 8, 8, 16),
    uniform(Block::DiamondOre, 8, 1, 16),
    Vein::new(
        Block::LapisOre,
        7,
        1,
        Height::Triangle {
            center: 16,
            spread: 16,
        },
    ),
];

struct Box2 {
    x: i32,
    z: i32,
}

impl Terrain {
    /// `BiomeDecorator.generateOres`: жилы колонки `(sx, sz)` заходят на +8 блоков, поэтому
    /// колонку задевают жилы четырёх колонок.
    pub(crate) fn grow_ores(&self, key: ColumnPos, column: &mut Column) {
        if self.world_type != WorldType::Standard {
            return;
        }
        let origin = Box2 {
            x: key.origin().x,
            z: key.origin().z,
        };
        let top = *self
            .climate(key.x(), key.z())
            .surface
            .iter()
            .max()
            .unwrap_or(&0);
        for sz in key.z() - 1..=key.z() {
            for sx in key.x() - 1..=key.x() {
                let state = u64::from(hash(self.seed, sx, sz, SALT_ORES))
                    | u64::from(hash(self.seed, sx, sz, SALT_ORES + 1)) << 32;
                let mut rng = Stream::new(state);
                let biome = self.climate(sx, sz).biomes[ColumnClimate::index(CENTER, CENTER)];
                let extra = [
                    biome.is_mesa().then_some(MESA_GOLD),
                    biome.is_extreme_hills().then_some(MONSTER_EGGS),
                ];
                for vein in VEINS.iter().copied().chain(extra.into_iter().flatten()) {
                    for _ in 0..vein.count {
                        let x = sx * SECTION_SIZE + rng.below(16) as i32;
                        let y = vein.height.pick(&mut rng);
                        let z = sz * SECTION_SIZE + rng.below(16) as i32;
                        let mut vein_rng = rng.fork();
                        sow_vein(
                            column,
                            &origin,
                            top,
                            vein,
                            IVec3::new(x, y, z),
                            &mut vein_rng,
                        );
                    }
                }
            }
        }
        self.scatter_emeralds(key, column);
    }

    /// `BiomeHills.decorate`: одиночные изумруды в камне только в своей колонке.
    fn scatter_emeralds(&self, key: ColumnPos, column: &mut Column) {
        let biome = self.climate(key.x(), key.z()).biomes[ColumnClimate::index(CENTER, CENTER)];
        if !biome.is_extreme_hills() {
            return;
        }
        let state = u64::from(hash(self.seed, key.x(), key.z(), SALT_EMERALDS))
            | u64::from(hash(self.seed, key.x(), key.z(), SALT_EMERALDS + 1)) << 32;
        let mut rng = Stream::new(state);
        for _ in 0..EMERALD_MIN_COUNT + rng.below(EMERALD_EXTRA_COUNT) {
            let at = IVec3::new(
                rng.below(16) as i32,
                (rng.below(EMERALD_Y_SPAN) + EMERALD_MIN_Y) as i32,
                rng.below(16) as i32,
            );
            if column.get(at).is_stone() {
                column.set(at, Block::EmeraldOre);
            }
        }
    }
}

/// `WorldGenMinable.generate`: цепочка эллипсоидов вдоль отрезка; `start` в мировых координатах.
fn sow_vein(
    column: &mut Column,
    origin: &Box2,
    top: i32,
    vein: Vein,
    start: IVec3,
    rng: &mut Stream,
) {
    let size = f64::from(vein.size);
    let angle = rng.fraction() * std::f64::consts::PI;
    let (sin, cos) = angle.sin_cos();
    let reach = size / 8.0;
    let (cx, cz) = (f64::from(start.x) + SHIFT, f64::from(start.z) + SHIFT);
    let (x0, x1) = (cx + sin * reach, cx - sin * reach);
    let (z0, z1) = (cz + cos * reach, cz - cos * reach);
    let y0 = f64::from(start.y + rng.below(3) as i32 - 2);
    let y1 = f64::from(start.y + rng.below(3) as i32 - 2);

    let max_radius = size / 16.0 + 0.5;
    let outside = |low: f64, high: f64, base: i32| {
        high + max_radius < f64::from(base) || low - max_radius >= f64::from(base + SECTION_SIZE)
    };
    if y0.min(y1) - max_radius > f64::from(top) {
        return;
    }
    if outside(x1.min(x0), x0.max(x1), origin.x) || outside(z1.min(z0), z0.max(z1), origin.z) {
        return;
    }
    let sines = profile(vein.size);
    for i in 0..vein.size {
        let t = f64::from(i) / size;
        let centre = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, z0 + (z1 - z0) * t);
        if outside(centre.0, centre.0, origin.x)
            || outside(centre.2, centre.2, origin.z)
            || centre.1 - max_radius > f64::from(top)
        {
            rng.skip();
            continue;
        }
        let spread = rng.fraction() * size / 16.0;
        let diameter = (sines[i as usize] + 1.0) * spread + 1.0;
        let half = diameter / 2.0;
        let inverse = 1.0 / half;
        let center = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, z0 + (z1 - z0) * t);
        let span = |c: f64, low: i32, high: i32| {
            ((c - half).floor() as i32).max(low)..=((c + half).floor() as i32).min(high)
        };
        for x in span(center.0, origin.x, origin.x + SECTION_SIZE - 1) {
            let dx = (f64::from(x) + 0.5 - center.0) * inverse;
            if dx * dx >= 1.0 {
                continue;
            }
            let slab = 1.0 - dx * dx;
            let Some((z_low, z_high)) = interval(
                center.2,
                inverse,
                slab,
                origin.z,
                origin.z + SECTION_SIZE - 1,
            ) else {
                continue;
            };
            for z in z_low..=z_high {
                let dz = (f64::from(z) + 0.5 - center.2) * inverse;
                if let Some((low, high)) = interval(center.1, inverse, slab - dz * dz, 0, top) {
                    column.replace_stone_run((x - origin.x, z - origin.z), (low, high), vein.block);
                }
            }
        }
    }
}

/// Целые `i` в `lo..=hi`, для которых центр ячейки лежит строго внутри эллипса: `((i + 0.5 - center) * inverse)² < budget`.
fn interval(center: f64, inverse: f64, budget: f64, lo: i32, hi: i32) -> Option<(i32, i32)> {
    let inside = |i: i32| {
        let d = (f64::from(i) + 0.5 - center) * inverse;
        d * d < budget
    };
    let start = (center.floor() as i32).clamp(lo, hi);
    if !inside(start) {
        return None;
    }
    let (mut low, mut high) = (start, start);
    while low > lo && inside(low - 1) {
        low -= 1;
    }
    while high < hi && inside(high + 1) {
        high += 1;
    }
    Some((low, high))
}

fn profile(size: u32) -> &'static [f64] {
    static TABLE: std::sync::OnceLock<Vec<Vec<f64>>> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let max = VEINS
            .iter()
            .map(|v| v.size)
            .max()
            .unwrap_or(0)
            .max(MESA_GOLD.size);
        (0..=max)
            .map(|n| {
                (0..n)
                    .map(|i| (std::f64::consts::PI * (f64::from(i) / f64::from(n))).sin())
                    .collect()
            })
            .collect()
    });
    &table[size as usize]
}

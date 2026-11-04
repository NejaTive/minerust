use crate::biome::{Biome, TempCategory};

const MULTIPLIER: i64 = 6364136223846793005;
const INCREMENT: i64 = 1442695040888963407;
const SPECIAL_BITS: i32 = 0xF00;
const ICY: i32 = 4;

const OCEAN: i32 = Biome::Ocean as i32;
const PLAINS: i32 = Biome::Plains as i32;
const DESERT: i32 = Biome::Desert as i32;
const FROZEN_OCEAN: i32 = Biome::FrozenOcean as i32;
const EXTREME_HILLS: i32 = Biome::ExtremeHills as i32;
const FOREST: i32 = Biome::Forest as i32;
const TAIGA: i32 = Biome::Taiga as i32;
const SWAMPLAND: i32 = Biome::Swampland as i32;
const RIVER: i32 = Biome::River as i32;
const FROZEN_RIVER: i32 = Biome::FrozenRiver as i32;
const ICE_PLAINS: i32 = Biome::IcePlains as i32;
const ICE_MOUNTAINS: i32 = Biome::IceMountains as i32;
const MUSHROOM_ISLAND: i32 = Biome::MushroomIsland as i32;
const MUSHROOM_SHORE: i32 = Biome::MushroomIslandShore as i32;
const BEACH: i32 = Biome::Beach as i32;
const DESERT_HILLS: i32 = Biome::DesertHills as i32;
const FOREST_HILLS: i32 = Biome::ForestHills as i32;
const TAIGA_HILLS: i32 = Biome::TaigaHills as i32;
const EXTREME_HILLS_EDGE: i32 = Biome::ExtremeHillsEdge as i32;
const JUNGLE: i32 = Biome::Jungle as i32;
const JUNGLE_HILLS: i32 = Biome::JungleHills as i32;
const JUNGLE_EDGE: i32 = Biome::JungleEdge as i32;
const DEEP_OCEAN: i32 = Biome::DeepOcean as i32;
const STONE_BEACH: i32 = Biome::StoneBeach as i32;
const COLD_BEACH: i32 = Biome::ColdBeach as i32;
const BIRCH_FOREST: i32 = Biome::BirchForest as i32;
const BIRCH_FOREST_HILLS: i32 = Biome::BirchForestHills as i32;
const ROOFED_FOREST: i32 = Biome::RoofedForest as i32;
const COLD_TAIGA: i32 = Biome::ColdTaiga as i32;
const COLD_TAIGA_HILLS: i32 = Biome::ColdTaigaHills as i32;
const MEGA_TAIGA: i32 = Biome::MegaTaiga as i32;
const MEGA_TAIGA_HILLS: i32 = Biome::MegaTaigaHills as i32;
const EXTREME_HILLS_PLUS: i32 = Biome::ExtremeHillsPlus as i32;
const SAVANNA: i32 = Biome::Savanna as i32;
const SAVANNA_PLATEAU: i32 = Biome::SavannaPlateau as i32;
const MESA: i32 = Biome::Mesa as i32;
const MESA_PLATEAU_F: i32 = Biome::MesaPlateauF as i32;
const MESA_PLATEAU: i32 = Biome::MesaPlateau as i32;
const SUNFLOWER_PLAINS: i32 = Biome::SunflowerPlains as i32;

fn step(seed: i64, add: i64) -> i64 {
    seed.wrapping_mul(seed.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT))
        .wrapping_add(add)
}

fn scramble(seed: i64, salt: i64, rounds: usize) -> i64 {
    (0..rounds).fold(seed, |s, _| step(s, salt))
}

struct Rng {
    seed: i64,
    world: i64,
}

impl Rng {
    fn at(world: i64, x: i32, z: i32) -> Rng {
        let (x, z) = (i64::from(x), i64::from(z));
        let seed = [x, z, x, z].into_iter().fold(world, step);
        Rng { seed, world }
    }

    fn next(&mut self, bound: i32) -> i32 {
        let mut value = ((self.seed >> 24) % i64::from(bound)) as i32;
        if value < 0 {
            value += bound;
        }
        self.seed = step(self.seed, self.world);
        value
    }

    fn pick(&mut self, options: &[i32]) -> i32 {
        options[self.next(options.len() as i32) as usize]
    }

    fn mode_or_random(&mut self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        let triple_a = (a == b && (a == c || a == d)) || (a == c && a == d);
        let pair_a = (a == b && c != d) || (a == c && b != d) || (a == d && b != c);
        if b == c && c == d {
            b
        } else if triple_a || pair_a {
            a
        } else if (b == c && a != d) || (b == d && a != c) {
            b
        } else if c == d && a != b {
            c
        } else {
            self.pick(&[a, b, c, d])
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeMode {
    CoolWarm,
    HeatIce,
    Special,
}

#[derive(Clone, Copy)]
enum Kind {
    Island,
    Zoom { fuzzy: bool },
    AddIsland,
    RemoveTooMuchOcean,
    AddSnow,
    Edge(EdgeMode),
    AddMushroomIsland,
    DeepOcean,
    Biome,
    BiomeEdge,
    RiverInit,
    Hills,
    River,
    Smooth,
    RareBiome,
    Shore,
    RiverMix,
}

struct Layer {
    kind: Kind,
    base: i64,
    world: i64,
    parent: usize,
    second: usize,
}

/// Окно значений слоя: `data[(z - z0) * w + (x - x0)]`.
pub struct Coarse {
    pub x: i32,
    pub z: i32,
    pub w: usize,
    pub data: Vec<i32>,
}

impl Coarse {
    pub fn at(&self, x: i32, z: i32) -> i32 {
        self.data[(z - self.z) as usize * self.w + (x - self.x) as usize]
    }
}

/// Порт стека `GenLayer.initializeAllBiomeGenerators` из Minecraft 1.12 (размер биомов 4,
/// размер рек 4). Значения слоёв — id биомов `Biome`.
pub struct BiomeMap {
    layers: Vec<Layer>,
    river_mix: usize,
    voronoi_world: i64,
}

struct Builder {
    layers: Vec<Layer>,
}

impl Builder {
    fn push(&mut self, kind: Kind, base: i64, parent: usize, second: usize) -> usize {
        self.layers.push(Layer {
            kind,
            base: scramble(base, base, 3),
            world: 0,
            parent,
            second,
        });
        self.layers.len() - 1
    }

    fn add(&mut self, kind: Kind, base: i64, parent: usize) -> usize {
        self.push(kind, base, parent, parent)
    }

    fn zooms(&mut self, base: i64, mut parent: usize, count: i64) -> usize {
        for k in 0..count {
            parent = self.add(Kind::Zoom { fuzzy: false }, base + k, parent);
        }
        parent
    }
}

impl BiomeMap {
    pub fn new(seed: i32) -> BiomeMap {
        let seed = i64::from(seed);
        let mut b = Builder { layers: Vec::new() };
        let island = b.add(Kind::Island, 1, 0);
        let mut l = b.add(Kind::Zoom { fuzzy: true }, 2000, island);
        l = b.add(Kind::AddIsland, 1, l);
        l = b.add(Kind::Zoom { fuzzy: false }, 2001, l);
        for base in [2, 50, 70] {
            l = b.add(Kind::AddIsland, base, l);
        }
        l = b.add(Kind::RemoveTooMuchOcean, 2, l);
        l = b.add(Kind::AddSnow, 2, l);
        l = b.add(Kind::AddIsland, 3, l);
        l = b.add(Kind::Edge(EdgeMode::CoolWarm), 2, l);
        l = b.add(Kind::Edge(EdgeMode::HeatIce), 2, l);
        l = b.add(Kind::Edge(EdgeMode::Special), 3, l);
        l = b.add(Kind::Zoom { fuzzy: false }, 2002, l);
        l = b.add(Kind::Zoom { fuzzy: false }, 2003, l);
        l = b.add(Kind::AddIsland, 4, l);
        l = b.add(Kind::AddMushroomIsland, 5, l);
        let land = b.add(Kind::DeepOcean, 4, l);

        let river_init = b.add(Kind::RiverInit, 100, land);
        let river_zoomed = b.zooms(1000, river_init, 2);

        let mut biomes = b.add(Kind::Biome, 200, land);
        biomes = b.zooms(1000, biomes, 2);
        biomes = b.add(Kind::BiomeEdge, 1000, biomes);
        let hills = b.push(Kind::Hills, 1000, biomes, river_zoomed);

        let river_wide = b.zooms(1000, river_zoomed, 4);
        let river = b.add(Kind::River, 1, river_wide);
        let river_smooth = b.add(Kind::Smooth, 1000, river);

        let mut l = b.add(Kind::RareBiome, 1001, hills);
        for j in 0..4 {
            l = b.add(Kind::Zoom { fuzzy: false }, 1000 + j, l);
            if j == 0 {
                l = b.add(Kind::AddIsland, 3, l);
            }
            if j == 1 {
                l = b.add(Kind::Shore, 1000, l);
            }
        }
        let land_smooth = b.add(Kind::Smooth, 1000, l);
        let river_mix = b.push(Kind::RiverMix, 100, land_smooth, river_smooth);

        let mut layers = b.layers;
        for layer in &mut layers {
            layer.world = scramble(seed, layer.base, 4);
        }
        BiomeMap {
            layers,
            river_mix,
            voronoi_world: scramble(seed, scramble(10, 10, 3), 4),
        }
    }

    pub fn coarse(&self, x: i32, z: i32, w: usize, h: usize) -> Coarse {
        Coarse {
            x,
            z,
            w,
            data: self.ints(self.river_mix, x, z, w, h),
        }
    }

    pub fn blocks(&self, x: i32, z: i32, w: usize, h: usize) -> Vec<Biome> {
        let (i, j, k, l) = voronoi_parent(x, z, w, h);
        self.voronoi(x, z, w, h, &self.coarse(i, j, k, l))
    }

    /// Тот же результат, что у `blocks`, но поверх уже вычисленного окна `coarse`.
    pub fn blocks_within(&self, x: i32, z: i32, w: usize, h: usize, coarse: &Coarse) -> Vec<Biome> {
        self.voronoi(x, z, w, h, coarse)
    }

    fn voronoi(&self, x: i32, z: i32, w: usize, h: usize, parent: &Coarse) -> Vec<Biome> {
        let (i, j, k, l) = voronoi_parent(x, z, w, h);
        let (x, z) = (x - 2, z - 2);
        let width = k * 4;
        let mut out = vec![0; width * l * 4];
        let jitter = |rng: &mut Rng| (rng.next(1024) as f64 / 1024.0 - 0.5) * 3.6;
        for cz in 0..l - 1 {
            for cx in 0..k - 1 {
                let (gx, gz) = (i + cx as i32, j + cz as i32);
                let corner = |dx: i32, dz: i32| {
                    let mut rng = Rng::at(self.voronoi_world, (gx + dx) << 2, (gz + dz) << 2);
                    let px = jitter(&mut rng) + f64::from(dx * 4);
                    let pz = jitter(&mut rng) + f64::from(dz * 4);
                    (px, pz)
                };
                let corners = [corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1)];
                let values = [
                    parent.at(gx, gz),
                    parent.at(gx + 1, gz),
                    parent.at(gx, gz + 1),
                    parent.at(gx + 1, gz + 1),
                ];
                for bz in 0..4 {
                    for bx in 0..4 {
                        let nearest = corners
                            .iter()
                            .map(|&(px, pz)| (bx as f64 - px).powi(2) + (bz as f64 - pz).powi(2))
                            .enumerate()
                            .fold(
                                (0, f64::MAX),
                                |best, (n, d)| if d < best.1 { (n, d) } else { best },
                            )
                            .0;
                        out[(cz * 4 + bz) * width + cx * 4 + bx] = values[nearest] & 255;
                    }
                }
            }
        }
        let (ox, oz) = ((x & 3) as usize, (z & 3) as usize);
        (0..h)
            .flat_map(|row| (0..w).map(move |col| (row, col)))
            .map(|(row, col)| {
                let id = out[(row + oz) * width + col + ox];
                Biome::from_id(id).unwrap_or(Biome::Plains)
            })
            .collect()
    }

    fn ints(&self, index: usize, x: i32, z: i32, w: usize, h: usize) -> Vec<i32> {
        let layer = &self.layers[index];
        let world = layer.world;
        let mut out = vec![0; w * h];
        let mut put = |i: usize, j: usize, value: i32| out[j * w + i] = value;
        match layer.kind {
            Kind::Island => {
                for j in 0..h {
                    for i in 0..w {
                        let (gx, gz) = (x + i as i32, z + j as i32);
                        let mut value = i32::from(Rng::at(world, gx, gz).next(10) == 0);
                        if gx == 0 && gz == 0 {
                            value = 1;
                        }
                        put(i, j, value);
                    }
                }
            }
            Kind::Zoom { fuzzy } => {
                let (px, pz) = (x >> 1, z >> 1);
                let (k, l) = ((w >> 1) + 2, (h >> 1) + 2);
                let parent = self.ints(layer.parent, px, pz, k, l);
                let wide = k << 1;
                let mut grid = vec![0; wide * (l << 1)];
                for cz in 0..l - 1 {
                    for cx in 0..k - 1 {
                        let a = parent[cz * k + cx];
                        let b = parent[cz * k + cx + 1];
                        let c = parent[(cz + 1) * k + cx];
                        let d = parent[(cz + 1) * k + cx + 1];
                        let mut rng = Rng::at(world, (cx as i32 + px) << 1, (cz as i32 + pz) << 1);
                        let (ox, oz) = (cx << 1, cz << 1);
                        grid[oz * wide + ox] = a;
                        grid[(oz + 1) * wide + ox] = rng.pick(&[a, c]);
                        grid[oz * wide + ox + 1] = rng.pick(&[a, b]);
                        grid[(oz + 1) * wide + ox + 1] = if fuzzy {
                            rng.pick(&[a, b, c, d])
                        } else {
                            rng.mode_or_random(a, b, c, d)
                        };
                    }
                }
                let (sx, sz) = ((x & 1) as usize, (z & 1) as usize);
                for j in 0..h {
                    for i in 0..w {
                        put(i, j, grid[(j + sz) * wide + i + sx]);
                    }
                }
            }
            Kind::RiverInit => {
                let parent = self.ints(layer.parent, x, z, w, h);
                for j in 0..h {
                    for i in 0..w {
                        let value = parent[j * w + i];
                        let out = if value > 0 {
                            Rng::at(world, x + i as i32, z + j as i32).next(299_999) + 2
                        } else {
                            0
                        };
                        put(i, j, out);
                    }
                }
            }
            Kind::Hills => {
                let biomes = self.ints(layer.parent, x - 1, z - 1, w + 2, h + 2);
                let rivers = self.ints(layer.second, x - 1, z - 1, w + 2, h + 2);
                let k = w + 2;
                for j in 0..h {
                    for i in 0..w {
                        let mut rng = Rng::at(world, x + i as i32, z + j as i32);
                        let at = |a: usize, b: usize| biomes[(j + b) * k + i + a];
                        let center = at(1, 1);
                        let river = rivers[(j + 1) * k + i + 1];
                        let mutate = (river - 2) % 29 == 0;
                        let is_mutation = Biome::from_id(center).is_some_and(Biome::is_mutation);
                        let value =
                            if center != 0 && river >= 2 && (river - 2) % 29 == 1 && !is_mutation {
                                mutation_id(center).unwrap_or(center)
                            } else if rng.next(3) != 0 && !mutate {
                                center
                            } else {
                                let mut hill = hill_of(center, &mut rng);
                                if mutate && hill != center {
                                    hill = mutation_id(hill).unwrap_or(center);
                                }
                                if hill == center {
                                    center
                                } else {
                                    let same = [at(1, 0), at(2, 1), at(0, 1), at(1, 2)]
                                        .into_iter()
                                        .filter(|&n| equal_or_plateau(n, center))
                                        .count();
                                    if same >= 3 { hill } else { center }
                                }
                            };
                        put(i, j, value);
                    }
                }
            }
            Kind::RiverMix => {
                let biomes = self.ints(layer.parent, x, z, w, h);
                let rivers = self.ints(layer.second, x, z, w, h);
                for (n, value) in out.iter_mut().enumerate() {
                    let (biome, river) = (biomes[n], rivers[n]);
                    *value = if is_oceanic(biome) {
                        biome
                    } else if river == RIVER {
                        if biome == ICE_PLAINS {
                            FROZEN_RIVER
                        } else if biome != MUSHROOM_ISLAND && biome != MUSHROOM_SHORE {
                            river
                        } else {
                            MUSHROOM_SHORE
                        }
                    } else {
                        biome
                    };
                }
            }
            _ => {
                let parent = self.ints(layer.parent, x - 1, z - 1, w + 2, h + 2);
                let k = w + 2;
                for j in 0..h {
                    for i in 0..w {
                        let at = |a: usize, b: usize| parent[(j + b) * k + i + a];
                        let around = Around {
                            n: at(1, 0),
                            e: at(2, 1),
                            w: at(0, 1),
                            s: at(1, 2),
                            nw: at(0, 0),
                            ne: at(2, 0),
                            sw: at(0, 2),
                            se: at(2, 2),
                            center: at(1, 1),
                        };
                        let mut rng = Rng::at(world, x + i as i32, z + j as i32);
                        put(i, j, cell(layer.kind, &around, &mut rng));
                    }
                }
            }
        }
        out
    }
}

fn voronoi_parent(x: i32, z: i32, w: usize, h: usize) -> (i32, i32, usize, usize) {
    ((x - 2) >> 2, (z - 2) >> 2, (w >> 2) + 2, (h >> 2) + 2)
}

struct Around {
    n: i32,
    e: i32,
    w: i32,
    s: i32,
    nw: i32,
    ne: i32,
    sw: i32,
    se: i32,
    center: i32,
}

impl Around {
    fn sides(&self) -> [i32; 4] {
        [self.n, self.e, self.w, self.s]
    }

    fn corners(&self) -> [i32; 4] {
        [self.nw, self.ne, self.sw, self.se]
    }

    fn any_side(&self, test: impl Fn(i32) -> bool) -> bool {
        self.sides().into_iter().any(test)
    }
}

fn cell(kind: Kind, a: &Around, rng: &mut Rng) -> i32 {
    let c = a.center;
    match kind {
        Kind::AddIsland => {
            let corners = a.corners();
            if c == 0 && corners.iter().any(|&v| v != 0) {
                let mut count = 1;
                let mut pick = 1;
                for &v in &corners {
                    if v != 0 {
                        if rng.next(count) == 0 {
                            pick = v;
                        }
                        count += 1;
                    }
                }
                if rng.next(3) == 0 {
                    pick
                } else if pick == ICY {
                    ICY
                } else {
                    0
                }
            } else if c > 0 && corners.contains(&0) {
                if rng.next(5) == 0 {
                    if c == ICY { ICY } else { 0 }
                } else {
                    c
                }
            } else {
                c
            }
        }
        Kind::RemoveTooMuchOcean => {
            if c == 0 && a.sides().iter().all(|&v| v == 0) && rng.next(2) == 0 {
                1
            } else {
                c
            }
        }
        Kind::AddSnow => {
            if c == 0 {
                0
            } else {
                match rng.next(6) {
                    0 => 4,
                    1 => 3,
                    _ => 1,
                }
            }
        }
        Kind::Edge(EdgeMode::CoolWarm) => {
            if c == 1 && a.any_side(|v| v == 3 || v == 4) {
                2
            } else {
                c
            }
        }
        Kind::Edge(EdgeMode::HeatIce) => {
            if c == 4 && a.any_side(|v| v == 1 || v == 2) {
                3
            } else {
                c
            }
        }
        Kind::Edge(EdgeMode::Special) => {
            if c != 0 && rng.next(13) == 0 {
                c | (((1 + rng.next(15)) << 8) & SPECIAL_BITS)
            } else {
                c
            }
        }
        Kind::AddMushroomIsland => {
            if c == 0 && a.corners().iter().all(|&v| v == 0) && rng.next(100) == 0 {
                MUSHROOM_ISLAND
            } else {
                c
            }
        }
        Kind::DeepOcean => {
            let oceans = a.sides().iter().filter(|&&v| is_oceanic(v)).count();
            if is_oceanic(c) && oceans > 3 {
                DEEP_OCEAN
            } else {
                c
            }
        }
        Kind::Biome => climate_biome(c, rng),
        Kind::BiomeEdge => biome_edge(a),
        Kind::River => {
            let filter = |v: i32| if v >= 2 { 2 + (v & 1) } else { v };
            let f = filter(c);
            if a.sides().iter().all(|&v| filter(v) == f) {
                -1
            } else {
                RIVER
            }
        }
        Kind::Smooth => {
            if a.w == a.e && a.n == a.s {
                if rng.next(2) == 0 { a.w } else { a.n }
            } else if a.n == a.s {
                a.n
            } else if a.w == a.e {
                a.w
            } else {
                c
            }
        }
        Kind::RareBiome => {
            if rng.next(57) == 0 && c == PLAINS {
                SUNFLOWER_PLAINS
            } else {
                c
            }
        }
        Kind::Shore => shore(a),
        Kind::Island | Kind::Zoom { .. } | Kind::RiverInit | Kind::Hills | Kind::RiverMix => c,
    }
}

fn climate_biome(value: i32, rng: &mut Rng) -> i32 {
    let special = (value & SPECIAL_BITS) >> 8;
    let value = value & !SPECIAL_BITS;
    if is_oceanic(value) || value == MUSHROOM_ISLAND {
        return value;
    }
    match value {
        1 => {
            if special > 0 {
                if rng.next(3) == 0 {
                    MESA_PLATEAU_F
                } else {
                    MESA_PLATEAU
                }
            } else {
                rng.pick(&[DESERT, DESERT, DESERT, SAVANNA, SAVANNA, PLAINS])
            }
        }
        2 => {
            if special > 0 {
                JUNGLE
            } else {
                rng.pick(&[
                    FOREST,
                    ROOFED_FOREST,
                    EXTREME_HILLS,
                    PLAINS,
                    BIRCH_FOREST,
                    SWAMPLAND,
                ])
            }
        }
        3 => {
            if special > 0 {
                MEGA_TAIGA
            } else {
                rng.pick(&[FOREST, EXTREME_HILLS, TAIGA, PLAINS])
            }
        }
        4 => rng.pick(&[ICE_PLAINS, ICE_PLAINS, COLD_TAIGA]),
        _ => MUSHROOM_ISLAND,
    }
}

fn biome_edge(a: &Around) -> i32 {
    let c = a.center;
    let replace = |id: i32, replacement: i32| {
        equal_or_plateau(c, id).then(|| {
            if a.sides().iter().all(|&v| can_be_neighbors(v, id)) {
                c
            } else {
                replacement
            }
        })
    };
    if let Some(v) = replace(EXTREME_HILLS, EXTREME_HILLS_EDGE)
        .or_else(|| replace(MESA_PLATEAU_F, MESA))
        .or_else(|| replace(MESA_PLATEAU, MESA))
        .or_else(|| replace(MEGA_TAIGA, TAIGA))
    {
        return v;
    }
    if c == DESERT {
        if a.any_side(|v| v == ICE_PLAINS) {
            EXTREME_HILLS_PLUS
        } else {
            c
        }
    } else if c == SWAMPLAND {
        if a.any_side(|v| v == DESERT || v == COLD_TAIGA || v == ICE_PLAINS) {
            PLAINS
        } else if a.any_side(|v| v == JUNGLE || v == JUNGLE_HILLS) {
            JUNGLE_EDGE
        } else {
            c
        }
    } else {
        c
    }
}

fn shore(a: &Around) -> i32 {
    let c = a.center;
    let biome = Biome::from_id(c);
    let coast = a.any_side(is_oceanic);
    if c == MUSHROOM_ISLAND {
        if a.any_side(|v| v == OCEAN) {
            MUSHROOM_SHORE
        } else {
            c
        }
    } else if biome.is_some_and(Biome::is_jungle) {
        let compatible = |v: i32| {
            Biome::from_id(v).is_some_and(Biome::is_jungle)
                || matches!(v, FOREST | TAIGA)
                || is_oceanic(v)
        };
        if !a.sides().iter().all(|&v| compatible(v)) {
            JUNGLE_EDGE
        } else if coast {
            BEACH
        } else {
            c
        }
    } else if matches!(c, EXTREME_HILLS | EXTREME_HILLS_PLUS | EXTREME_HILLS_EDGE) {
        if !is_oceanic(c) && coast {
            STONE_BEACH
        } else {
            c
        }
    } else if biome.is_some_and(Biome::is_snowy) {
        if !is_oceanic(c) && coast {
            COLD_BEACH
        } else {
            c
        }
    } else if matches!(c, MESA | MESA_PLATEAU_F) {
        if coast { DESERT } else { c }
    } else if !matches!(c, OCEAN | DEEP_OCEAN | RIVER | SWAMPLAND) && coast {
        BEACH
    } else {
        c
    }
}

fn hill_of(center: i32, rng: &mut Rng) -> i32 {
    match center {
        DESERT => DESERT_HILLS,
        FOREST => FOREST_HILLS,
        BIRCH_FOREST => BIRCH_FOREST_HILLS,
        ROOFED_FOREST => PLAINS,
        TAIGA => TAIGA_HILLS,
        MEGA_TAIGA => MEGA_TAIGA_HILLS,
        COLD_TAIGA => COLD_TAIGA_HILLS,
        PLAINS => {
            if rng.next(3) == 0 {
                FOREST_HILLS
            } else {
                FOREST
            }
        }
        ICE_PLAINS => ICE_MOUNTAINS,
        JUNGLE => JUNGLE_HILLS,
        OCEAN => DEEP_OCEAN,
        EXTREME_HILLS => EXTREME_HILLS_PLUS,
        SAVANNA => SAVANNA_PLATEAU,
        v if equal_or_plateau(v, MESA_PLATEAU_F) => MESA,
        DEEP_OCEAN if rng.next(3) == 0 => {
            if rng.next(2) == 0 {
                PLAINS
            } else {
                FOREST
            }
        }
        v => v,
    }
}

fn mutation_id(id: i32) -> Option<i32> {
    Biome::from_id(id)?.mutation().map(|b| b as i32)
}

fn is_oceanic(id: i32) -> bool {
    matches!(id, OCEAN | FROZEN_OCEAN | DEEP_OCEAN)
}

fn equal_or_plateau(a: i32, b: i32) -> bool {
    a == b
        || (matches!(a, MESA_PLATEAU_F | MESA_PLATEAU)
            && matches!(b, MESA_PLATEAU_F | MESA_PLATEAU))
}

fn can_be_neighbors(a: i32, b: i32) -> bool {
    if equal_or_plateau(a, b) {
        return true;
    }
    match (Biome::from_id(a), Biome::from_id(b)) {
        (Some(a), Some(b)) => {
            let (a, b) = (a.temp_category(), b.temp_category());
            a == b || a == TempCategory::Medium || b == TempCategory::Medium
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;

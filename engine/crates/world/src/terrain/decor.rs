use super::*;
use crate::block::VINE_SIDES;
use crate::stream::Stream;

const SALT_DECOR: u64 = 33;
const REED_ATTEMPTS: u32 = 10;
const CACTUS_TRIES: u32 = 10;
const CACTUS_SPREAD: u32 = 8;
const CACTUS_SURFACE_REACH: u32 = 8;
const BUSH_TRIES: u32 = 4;
const BUSH_SPREAD: u32 = 8;
const BUSH_SKY_REACH: u32 = 32;
const REED_TRIES: u32 = 20;
const REED_SPREAD: u32 = 4;
const MELON_TRIES: u32 = 64;
const MELON_SPREAD: u32 = 8;
const MELON_VERTICAL_SPREAD: u32 = 4;
const VINE_ATTEMPTS: u32 = 50;
const VINE_SPREAD: u32 = 4;
const VINE_CEILING: i32 = 128;
const LILY_TRIES: u32 = 10;
const LILY_SPREAD: u32 = 8;
const LILY_VERTICAL_SPREAD: u32 = 4;
const MUSHROOM_TRIES: u32 = 64;
const MUSHROOM_SPREAD: u32 = 8;
const MUSHROOM_VERTICAL_SPREAD: u32 = 4;
const CAVE_MUSHROOM_CEILING: u32 = 128;
const BROWN_MUSHROOM_ONE_IN: u32 = 4;
const RED_MUSHROOM_ONE_IN: u32 = 8;
const SHADE_REACH: i32 = 8;
const SHADE_BLOCKS: i32 = 3;
const CENTER: i32 = 8;
const DOUBLE_TRIES: u32 = 64;
const DOUBLE_SPREAD: u32 = 8;
const DOUBLE_VERTICAL_SPREAD: u32 = 4;
const DOUBLE_SKY_REACH: u32 = 32;
const DOUBLE_SPOTS: u32 = 5;
const FLOWER_DOUBLE_KINDS: [Block; 3] =
    [Block::LilacLower, Block::RoseBushLower, Block::PeonyLower];
const FERN_DOUBLES: u32 = 7;
const GRASS_DOUBLES: u32 = 7;
const SUNFLOWERS: u32 = 10;
const SPIKES: u32 = 3;
const SPIKE_PATHS: u32 = 2;
const SPIKE_INNER_MARGIN: i32 = 3;
const SPIKE_FLOOR: i32 = 50;
const SPIKE_BASE_HEIGHT: i32 = 7;
const SPIKE_HIGH_ONE_IN: u32 = 60;
const SPIKE_HIGH_BASE: u32 = 10;
const SPIKE_HIGH_SPAN: u32 = 30;
const SPIKE_EDGE_KEPT: u32 = 3;
const SPIKE_SHAFT_LEN: i32 = 50;
const PATH_WIDTH_SPAN: u32 = 2;
const PATH_WIDTH_MIN: i32 = 2;
const BOULDER_PASSES: u32 = 3;
const BOULDER_MAX: u32 = 3;
const BOULDER_BLOB_SCALE: f32 = 0.333;
const BOULDER_BLOB_BASE: f32 = 0.5;
const PLAINS_TALL_GRASS_NOISE_SCALE: f32 = 200.0;
const PLAINS_TULIP_BELOW: f32 = -0.55;

const HORIZONTAL: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

struct Counts {
    cacti: u32,
    bushes: u32,
    reeds: u32,
    melons: u32,
    vines: u32,
    lilies: u32,
    mushrooms: u32,
    spikes: u32,
    paths: u32,
    boulders: bool,
    fern_doubles: u32,
    grass_doubles: u32,
    sunflowers: u32,
    flower_doubles: Option<i32>,
}

fn counts(biome: Biome) -> Counts {
    if biome.is_desert() {
        Counts {
            cacti: 10,
            bushes: 2,
            reeds: 50,
            ..Counts::NONE
        }
    } else if biome.is_mesa() {
        Counts {
            cacti: 5,
            bushes: 20,
            reeds: 3,
            ..Counts::NONE
        }
    } else if biome.is_jungle() {
        Counts {
            melons: 1,
            vines: VINE_ATTEMPTS,
            ..Counts::NONE
        }
    } else if biome.is_swamp() {
        Counts {
            reeds: 10,
            lilies: 4,
            mushrooms: 8,
            ..Counts::NONE
        }
    } else if biome.is_mushroom_island() {
        Counts {
            mushrooms: 1,
            ..Counts::NONE
        }
    } else if biome == Biome::IceSpikes {
        Counts {
            spikes: SPIKES,
            paths: SPIKE_PATHS,
            ..Counts::NONE
        }
    } else if biome.is_taiga() {
        Counts {
            boulders: biome.is_mega_taiga(),
            fern_doubles: FERN_DOUBLES,
            ..Counts::NONE
        }
    } else if matches!(biome, Biome::Plains | Biome::SunflowerPlains) {
        Counts {
            grass_doubles: GRASS_DOUBLES,
            sunflowers: if biome == Biome::SunflowerPlains {
                SUNFLOWERS
            } else {
                0
            },
            ..Counts::NONE
        }
    } else if matches!(
        biome,
        Biome::Savanna | Biome::SavannaM | Biome::SavannaPlateau | Biome::SavannaPlateauM
    ) {
        Counts {
            grass_doubles: GRASS_DOUBLES,
            ..Counts::NONE
        }
    } else if biome.is_forest() {
        Counts {
            flower_doubles: Some(if biome == Biome::FlowerForest { 2 } else { 0 }),
            ..Counts::NONE
        }
    } else {
        Counts::NONE
    }
}

impl Counts {
    const NONE: Counts = Counts {
        cacti: 0,
        bushes: 0,
        reeds: 0,
        melons: 0,
        vines: 0,
        lilies: 0,
        mushrooms: 0,
        spikes: 0,
        paths: 0,
        boulders: false,
        fern_doubles: 0,
        grass_doubles: 0,
        sunflowers: 0,
        flower_doubles: None,
    };
}

struct Spot {
    x: i32,
    z: i32,
}

impl Terrain {
    pub(crate) fn decorate(&self, key: ColumnPos, column: &mut Column) {
        if self.world_type != WorldType::Standard {
            return;
        }
        let climate = self.climate(key.x(), key.z());
        let biome = climate.biomes[ColumnClimate::index(CENTER, CENTER)];
        let counts = counts(biome);
        let state = u64::from(hash(self.seed, key.x(), key.z(), SALT_DECOR))
            | u64::from(hash(self.seed, key.x(), key.z(), SALT_DECOR + 1)) << 32;
        let mut rng = Stream::new(state);
        let surface = |spot: &Spot| climate.surface[ColumnClimate::index(spot.x, spot.z)];
        let pick = |rng: &mut Stream| Spot {
            x: rng.below(SECTION_SIZE as u32) as i32,
            z: rng.below(SECTION_SIZE as u32) as i32,
        };
        let near = |rng: &mut Stream, spot: &Spot, spread: u32| {
            let at = Spot {
                x: spot.x + rng.spread(spread),
                z: spot.z + rng.spread(spread),
            };
            (0..SECTION_SIZE)
                .contains(&at.x)
                .then_some(())
                .filter(|()| (0..SECTION_SIZE).contains(&at.z))
                .map(|()| at)
        };

        for _ in 0..counts.cacti {
            let spot = pick(&mut rng);
            let reach = 2 * surface(&spot) as u32;
            if !rng.chance(CACTUS_SURFACE_REACH, reach) {
                continue;
            }
            for _ in 0..CACTUS_TRIES {
                let height = 1 + rng.taper() as i32;
                if let Some(at) = near(&mut rng, &spot, CACTUS_SPREAD) {
                    self.plant_cactus(key, column, &climate, &at, height);
                }
            }
        }
        for _ in 0..counts.bushes {
            let spot = pick(&mut rng);
            if !rng.chance(BUSH_SKY_REACH, surface(&spot) as u32 + BUSH_SKY_REACH) {
                continue;
            }
            for _ in 0..BUSH_TRIES {
                if let Some(at) = near(&mut rng, &spot, BUSH_SPREAD) {
                    let ground = IVec3::new(at.x, surface(&at), at.z);
                    let above = ground + IVec3::Y;
                    if Block::DeadBush.stands_on(column.get(ground))
                        && column.get(above) == Block::Air
                    {
                        column.set(above, Block::DeadBush);
                    }
                }
            }
        }
        for _ in 0..REED_ATTEMPTS + counts.reeds {
            let spot = pick(&mut rng);
            if !rng.chance(1, 2 * surface(&spot) as u32) {
                continue;
            }
            for _ in 0..REED_TRIES {
                let height = 2 + rng.taper() as i32;
                if let Some(at) = near(&mut rng, &spot, REED_SPREAD) {
                    self.plant_reeds(key, column, &climate, &at, height);
                }
            }
        }
        for _ in 0..counts.melons {
            let spot = pick(&mut rng);
            let reach = 2 * surface(&spot) as u32;
            let y = rng.below(reach.max(1)) as i32;
            for _ in 0..MELON_TRIES {
                let at = IVec3::new(
                    spot.x + rng.spread(MELON_SPREAD),
                    y + rng.spread(MELON_VERTICAL_SPREAD),
                    spot.z + rng.spread(MELON_SPREAD),
                );
                if Column::contains(at)
                    && column.get(at) == Block::Air
                    && column.get(at - IVec3::Y) == Block::Grass
                {
                    column.set(at, Block::Melon);
                }
            }
        }
        for _ in 0..counts.vines {
            let spot = pick(&mut rng);
            self.grow_vines(&mut rng, column, IVec3::new(spot.x, WATER_LEVEL, spot.z));
        }
        for _ in 0..counts.lilies {
            let spot = pick(&mut rng);
            let mut y = (rng.below(2 * surface(&spot) as u32) as i32).min(WORLD_HEIGHT - 1);
            while y > 0 && column.get(IVec3::new(spot.x, y - 1, spot.z)) == Block::Air {
                y -= 1;
            }
            self.grow_lilies(&mut rng, column, IVec3::new(spot.x, y, spot.z));
        }
        for _ in 0..counts.mushrooms {
            if rng.below(BROWN_MUSHROOM_ONE_IN) == 0 {
                let spot = pick(&mut rng);
                let start = IVec3::new(spot.x, surface(&spot) + 1, spot.z);
                self.grow_mushrooms(&mut rng, column, &climate, Block::BrownMushroom, start);
            }
            if rng.below(RED_MUSHROOM_ONE_IN) == 0 {
                let spot = pick(&mut rng);
                let y = rng.below(2 * surface(&spot) as u32) as i32;
                let start = IVec3::new(spot.x, y, spot.z);
                self.grow_mushrooms(&mut rng, column, &climate, Block::RedMushroom, start);
            }
        }
        for (block, one_in) in [
            (Block::BrownMushroom, BROWN_MUSHROOM_ONE_IN),
            (Block::RedMushroom, RED_MUSHROOM_ONE_IN),
        ] {
            if rng.below(one_in) == 0 {
                let spot = pick(&mut rng);
                let y = rng.below(CAVE_MUSHROOM_CEILING) as i32;
                let start = IVec3::new(spot.x, y, spot.z);
                self.grow_mushrooms(&mut rng, column, &climate, block, start);
            }
        }
        self.grow_disks(key, column);
        self.decorate_cold_and_flowers(&mut rng, column, &climate, &counts, key);
        self.grow_pumpkins(key, column);
        self.grow_springs(key, column);
    }

    fn decorate_cold_and_flowers(
        &self,
        rng: &mut Stream,
        column: &mut Column,
        climate: &ColumnClimate,
        counts: &Counts,
        key: ColumnPos,
    ) {
        let inner = |rng: &mut Stream| Spot {
            x: SPIKE_INNER_MARGIN
                + rng.below((SECTION_SIZE - 2 * SPIKE_INNER_MARGIN) as u32) as i32,
            z: SPIKE_INNER_MARGIN
                + rng.below((SECTION_SIZE - 2 * SPIKE_INNER_MARGIN) as u32) as i32,
        };
        let anywhere = |rng: &mut Stream| Spot {
            x: rng.below(SECTION_SIZE as u32) as i32,
            z: rng.below(SECTION_SIZE as u32) as i32,
        };
        for _ in 0..counts.spikes {
            let spot = inner(rng);
            self.grow_ice_spike(rng, column, climate, &spot);
        }
        for _ in 0..counts.paths {
            let spot = inner(rng);
            self.grow_ice_path(rng, column, climate, &spot);
        }
        if counts.boulders {
            for _ in 0..rng.below(BOULDER_MAX) {
                let spot = inner(rng);
                self.grow_boulder(rng, column, climate, &spot);
            }
        }
        let origin = key.origin();
        let tall_grass_here = self.color_noise(
            origin.x + CENTER,
            origin.z + CENTER,
            PLAINS_TALL_GRASS_NOISE_SCALE,
        ) >= PLAINS_TULIP_BELOW;
        let doubles = [
            (counts.fern_doubles, Block::DoubleFernLower, true),
            (
                counts.grass_doubles,
                Block::DoubleGrassLower,
                tall_grass_here,
            ),
            (counts.sunflowers, Block::SunflowerLower, true),
        ];
        for (count, kind, enabled) in doubles {
            if !enabled {
                continue;
            }
            for _ in 0..count {
                let spot = anywhere(rng);
                let reach = climate.surface[ColumnClimate::index(spot.x, spot.z)] as u32;
                let y = rng.below(reach + DOUBLE_SKY_REACH) as i32;
                self.grow_double(rng, column, kind, IVec3::new(spot.x, y, spot.z));
            }
        }
        if let Some(extra) = counts.flower_doubles {
            let kinds = rng.below(5) as i32 - 3 + extra;
            for _ in 0..kinds.max(0) {
                let kind = FLOWER_DOUBLE_KINDS[rng.below(3) as usize];
                for _ in 0..DOUBLE_SPOTS {
                    let spot = anywhere(rng);
                    let reach = climate.surface[ColumnClimate::index(spot.x, spot.z)] as u32;
                    let y = rng.below(reach + DOUBLE_SKY_REACH) as i32;
                    if self.grow_double(rng, column, kind, IVec3::new(spot.x, y, spot.z)) {
                        break;
                    }
                }
            }
        }
    }

    fn grow_double(
        &self,
        rng: &mut Stream,
        column: &mut Column,
        lower: Block,
        start: IVec3,
    ) -> bool {
        let Some(upper) = lower.double_upper() else {
            return false;
        };
        let mut placed = false;
        for _ in 0..DOUBLE_TRIES {
            let at = start
                + IVec3::new(
                    rng.spread(DOUBLE_SPREAD),
                    rng.spread(DOUBLE_VERTICAL_SPREAD),
                    rng.spread(DOUBLE_SPREAD),
                );
            if at.y > 0
                && Column::contains(at + IVec3::Y)
                && column.get(at) == Block::Air
                && column.get(at + IVec3::Y) == Block::Air
                && matches!(
                    column.get(at - IVec3::Y),
                    Block::Grass | Block::Dirt | Block::Podzol | Block::CoarseDirt
                )
            {
                column.set(at, lower);
                column.set(at + IVec3::Y, upper);
                placed = true;
            }
        }
        placed
    }

    fn grow_ice_spike(
        &self,
        rng: &mut Stream,
        column: &mut Column,
        climate: &ColumnClimate,
        spot: &Spot,
    ) {
        let index = ColumnClimate::index(spot.x, spot.z);
        if climate.biomes[index] != Biome::IceSpikes || climate.surface[index] < WATER_LEVEL {
            return;
        }
        let mut base = IVec3::new(spot.x, climate.surface[index] + rng.below(4) as i32, spot.z);
        let height = rng.below(4) as i32 + SPIKE_BASE_HEIGHT;
        let radius = height / 4 + rng.below(2) as i32;
        if radius > 1 && rng.below(SPIKE_HIGH_ONE_IN) == 0 {
            base.y += (SPIKE_HIGH_BASE + rng.below(SPIKE_HIGH_SPAN)) as i32;
        }
        for k in 0..height {
            let reach = (1.0 - k as f32 / height as f32) * radius as f32;
            let l = reach.ceil() as i32;
            for dx in -l..=l {
                let fx = dx.abs() as f32 - 0.25;
                for dz in -l..=l {
                    let fz = dz.abs() as f32 - 0.25;
                    let inside = (dx == 0 && dz == 0) || fx * fx + fz * fz <= reach * reach;
                    let edge = dx == -l || dx == l || dz == -l || dz == l;
                    if !inside || (edge && rng.below(4) >= SPIKE_EDGE_KEPT) {
                        continue;
                    }
                    freeze(column, base + IVec3::new(dx, k, dz));
                    if k != 0 && l > 1 {
                        freeze(column, base + IVec3::new(dx, -k, dz));
                    }
                }
            }
        }
        let shaft = (radius - 1).clamp(0, 1);
        for dx in -shaft..=shaft {
            for dz in -shaft..=shaft {
                let mut at = base + IVec3::new(dx, -1, dz);
                let mut left = SPIKE_SHAFT_LEN;
                if dx.abs() == 1 && dz.abs() == 1 {
                    left = rng.below(5) as i32;
                }
                while at.y > SPIKE_FLOOR {
                    if !Column::contains(at) || !freezes(column.get(at)) {
                        break;
                    }
                    column.set(at, Block::IcePacked);
                    at.y -= 1;
                    left -= 1;
                    if left <= 0 {
                        at.y -= rng.below(5) as i32 + 1;
                        left = rng.below(5) as i32;
                    }
                }
            }
        }
    }

    fn grow_ice_path(
        &self,
        rng: &mut Stream,
        column: &mut Column,
        climate: &ColumnClimate,
        spot: &Spot,
    ) {
        let index = ColumnClimate::index(spot.x, spot.z);
        if climate.biomes[index] != Biome::IceSpikes || climate.surface[index] < WATER_LEVEL {
            return;
        }
        let ground = climate.surface[index];
        let radius = rng.below(PATH_WIDTH_SPAN) as i32 + PATH_WIDTH_MIN;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx * dx + dz * dz > radius * radius {
                    continue;
                }
                for y in ground - 1..=ground + 1 {
                    let at = IVec3::new(spot.x + dx, y, spot.z + dz);
                    if Column::contains(at)
                        && matches!(
                            column.get(at),
                            Block::Dirt | Block::Grass | Block::SnowyGrass | Block::Ice
                        )
                    {
                        column.set(at, Block::IcePacked);
                        if column.get(at + IVec3::Y) == Block::Snow {
                            column.set(at + IVec3::Y, Block::Air);
                        }
                    }
                }
            }
        }
    }

    fn grow_boulder(
        &self,
        rng: &mut Stream,
        column: &mut Column,
        climate: &ColumnClimate,
        spot: &Spot,
    ) {
        let surface = climate.surface[ColumnClimate::index(spot.x, spot.z)];
        let ground = IVec3::new(spot.x, surface, spot.z);
        if !matches!(
            column.get(ground),
            Block::Grass | Block::Dirt | Block::Podzol | Block::CoarseDirt
        ) && !column.get(ground).is_stone()
        {
            return;
        }
        let mut center = ground + IVec3::Y;
        for _ in 0..BOULDER_PASSES {
            let (j, k, l) = (
                rng.below(2) as i32,
                rng.below(2) as i32,
                rng.below(2) as i32,
            );
            let size = (j + k + l) as f32 * BOULDER_BLOB_SCALE + BOULDER_BLOB_BASE;
            for dx in -j..=j {
                for dy in -k..=k {
                    for dz in -l..=l {
                        let at = center + IVec3::new(dx, dy, dz);
                        if (dx * dx + dy * dy + dz * dz) as f32 <= size * size
                            && Column::contains(at)
                        {
                            column.set(at, Block::MossyCobblestone);
                        }
                    }
                }
            }
            center += IVec3::new(
                -1 + rng.below(2) as i32,
                -(rng.below(2) as i32),
                -1 + rng.below(2) as i32,
            );
        }
    }

    fn grow_lilies(&self, rng: &mut Stream, column: &mut Column, start: IVec3) {
        for _ in 0..LILY_TRIES {
            let at = start
                + IVec3::new(
                    rng.spread(LILY_SPREAD),
                    rng.spread(LILY_VERTICAL_SPREAD),
                    rng.spread(LILY_SPREAD),
                );
            if Column::contains(at)
                && column.get(at) == Block::Air
                && Column::contains(at - IVec3::Y)
                && column.get(at - IVec3::Y) == Block::Water
            {
                column.set(at, Block::Waterlily);
            }
        }
    }

    fn grow_mushrooms(
        &self,
        rng: &mut Stream,
        column: &mut Column,
        climate: &ColumnClimate,
        block: Block,
        start: IVec3,
    ) {
        for _ in 0..MUSHROOM_TRIES {
            let at = start
                + IVec3::new(
                    rng.spread(MUSHROOM_SPREAD),
                    rng.spread(MUSHROOM_VERTICAL_SPREAD),
                    rng.spread(MUSHROOM_SPREAD),
                );
            if Column::contains(at)
                && Column::contains(at - IVec3::Y)
                && column.get(at) == Block::Air
                && mushroom_stays(column, climate, at)
            {
                column.set(at, block);
            }
        }
    }

    fn grow_vines(&self, rng: &mut Stream, column: &mut Column, start: IVec3) {
        let mut at = start;
        while at.y < VINE_CEILING {
            if column.get(at) == Block::Air {
                let side = VINE_SIDES[..4].iter().find(|&&(_, dir)| {
                    Column::contains(at + dir) && column.get(at + dir).is_opaque()
                });
                if let Some(&(bit, _)) = side {
                    column.set_with_data(at, Block::Vine, bit);
                }
            } else {
                at.x += rng.spread(VINE_SPREAD);
                at.z += rng.spread(VINE_SPREAD);
                if !Column::contains(at) {
                    return;
                }
            }
            at.y += 1;
        }
    }

    fn plant_cactus(
        &self,
        key: ColumnPos,
        column: &mut Column,
        climate: &ColumnClimate,
        at: &Spot,
        height: i32,
    ) {
        let ground = IVec3::new(
            at.x,
            climate.surface[ColumnClimate::index(at.x, at.z)],
            at.z,
        );
        if !column.get(ground).is_sand() {
            return;
        }
        for y in ground.y + 1..ground.y + 1 + height {
            let pos = IVec3::new(at.x, y, at.z);
            if y >= WORLD_HEIGHT
                || column.get(pos) != Block::Air
                || self.solid_beside(key, column, pos)
            {
                return;
            }
            column.set(pos, Block::Cactus);
        }
    }

    fn solid_beside(&self, key: ColumnPos, column: &Column, pos: IVec3) -> bool {
        HORIZONTAL.iter().any(|&dir| {
            let side = pos + dir;
            if Column::contains(side) {
                return column.get(side).is_solid();
            }
            let origin = key.origin();
            self.surface_height(origin.x + side.x, origin.z + side.z) >= pos.y
        })
    }

    fn plant_reeds(
        &self,
        key: ColumnPos,
        column: &mut Column,
        climate: &ColumnClimate,
        at: &Spot,
        height: i32,
    ) {
        if climate.surface[ColumnClimate::index(at.x, at.z)] != WATER_LEVEL {
            return;
        }
        let ground = IVec3::new(at.x, WATER_LEVEL, at.z);
        let base = ground + IVec3::Y;
        if !Block::Reeds.stands_on(column.get(ground))
            || column.get(base) != Block::Air
            || !self.water_beside(key, column, ground)
        {
            return;
        }
        for y in base.y..base.y + height {
            let pos = IVec3::new(at.x, y, at.z);
            if column.get(pos) != Block::Air {
                return;
            }
            column.set(pos, Block::Reeds);
        }
    }

    fn water_beside(&self, key: ColumnPos, column: &Column, pos: IVec3) -> bool {
        HORIZONTAL.iter().any(|&dir| {
            let side = pos + dir;
            if Column::contains(side) {
                return column.get(side) == Block::Water;
            }
            let (x, z) = (key.origin().x + side.x, key.origin().z + side.z);
            self.surface_height(x, z) < WATER_LEVEL && !self.biome_at(x, z).is_snowy()
        })
    }
}

fn freezes(block: Block) -> bool {
    matches!(
        block,
        Block::Air
            | Block::Dirt
            | Block::Grass
            | Block::SnowyGrass
            | Block::Snow
            | Block::Ice
            | Block::IcePacked
    )
}

fn freeze(column: &mut Column, at: IVec3) {
    if Column::contains(at) && freezes(column.get(at)) {
        column.set(at, Block::IcePacked);
    }
}

fn mushroom_stays(column: &Column, climate: &ColumnClimate, at: IVec3) -> bool {
    let below = column.get(at - IVec3::Y);
    if below == Block::Mycelium {
        return true;
    }
    if !below.is_solid() {
        return false;
    }
    let surface = climate.surface[ColumnClimate::index(at.x, at.z)];
    at.y <= surface || shaded(column, at)
}

fn shaded(column: &Column, at: IVec3) -> bool {
    let covering = (1..=SHADE_REACH)
        .filter(|&dy| {
            let above = at + IVec3::Y * dy;
            Column::contains(above) && column.get(above) != Block::Air
        })
        .count() as i32;
    covering >= SHADE_BLOCKS
}

mod disks;
pub(in crate::terrain) mod lakes;
mod ores;
mod pumpkins;
mod springs;

#[cfg(test)]
mod tests;

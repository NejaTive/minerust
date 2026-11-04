use super::*;

const SALT_DISKS: u64 = 60;
const MIN_RADIUS: u32 = 2;
const SAND_DISKS: u32 = 3;
const CLAY_DISKS: u32 = 1;
const GRAVEL_DISKS: u32 = 1;

#[derive(Clone, Copy)]
enum Disk {
    Sand,
    Clay,
    Gravel,
}

impl Disk {
    const ORDER: [Disk; 3] = [Disk::Sand, Disk::Clay, Disk::Gravel];

    fn block(self) -> Block {
        match self {
            Disk::Sand => Block::Sand,
            Disk::Clay => Block::Clay,
            Disk::Gravel => Block::Gravel,
        }
    }

    fn max_radius(self) -> u32 {
        match self {
            Disk::Sand => 7,
            Disk::Clay => 4,
            Disk::Gravel => 6,
        }
    }

    fn reach(self) -> i32 {
        match self {
            Disk::Clay => 1,
            _ => 2,
        }
    }

    fn count(self) -> u32 {
        match self {
            Disk::Sand => SAND_DISKS,
            Disk::Clay => CLAY_DISKS,
            Disk::Gravel => GRAVEL_DISKS,
        }
    }

    fn replaces(self, block: Block) -> bool {
        match self {
            Disk::Clay => matches!(block, Block::Dirt | Block::Clay),
            _ => matches!(block, Block::Dirt | Block::Grass),
        }
    }
}

impl Terrain {
    /// `WorldGenSand` и `WorldGenClay`: диски у воды; пятна соседних колонок заходят в эту без швов.
    pub(super) fn grow_disks(&self, key: ColumnPos, column: &mut Column) {
        let origin = key.origin();
        for disk in Disk::ORDER {
            for dz in -1..=1 {
                for dx in -1..=1 {
                    let (cx, cz) = (key.x() + dx, key.z() + dz);
                    let climate = self.climate(cx, cz);
                    let state = u64::from(hash(self.seed, cx, cz, SALT_DISKS + disk as u64))
                        | u64::from(hash(self.seed, cx, cz, SALT_DISKS + 8 + disk as u64)) << 32;
                    let mut rng = Stream::new(state);
                    for _ in 0..disk.count() {
                        let (sx, sz) = (rng.below(16) as i32, rng.below(16) as i32);
                        let radius =
                            (rng.below(disk.max_radius() - MIN_RADIUS) + MIN_RADIUS) as i32;
                        let index = ColumnClimate::index(sx, sz);
                        let wet = climate.surface[index] < WATER_LEVEL
                            && climate.biomes[index].temperature() >= FREEZE_BELOW;
                        if wet {
                            let center = (
                                cx * SECTION_SIZE + sx - origin.x,
                                cz * SECTION_SIZE + sz - origin.z,
                            );
                            fill_disk(column, disk, center, radius);
                        }
                    }
                }
            }
        }
    }
}

fn fill_disk(column: &mut Column, disk: Disk, (cx, cz): (i32, i32), radius: i32) {
    for z in cz - radius..=cz + radius {
        for x in cx - radius..=cx + radius {
            let in_disk = (x - cx).pow(2) + (z - cz).pow(2) <= radius * radius;
            if !in_disk || !(0..SECTION_SIZE).contains(&x) || !(0..SECTION_SIZE).contains(&z) {
                continue;
            }
            for y in WATER_LEVEL - disk.reach()..=WATER_LEVEL + disk.reach() {
                let at = IVec3::new(x, y, z);
                let open_above = matches!(column.get(at + IVec3::Y), Block::Water | Block::Air);
                if disk.replaces(column.get(at)) && open_above {
                    column.set(at, disk.block());
                }
            }
        }
    }
}

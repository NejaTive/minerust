use super::*;
use crate::block::SPAWNER_PIG;

const SEED: i32 = 1337;

fn keys() -> impl Iterator<Item = ColumnPos> {
    (-20..20).flat_map(|z| (-20..20).map(move |x| ColumnPos::new(x, z)))
}

fn lined(block: Block) -> bool {
    matches!(block, Block::Cobblestone | Block::MossyCobblestone)
}

fn spawners(column: &Column) -> Vec<IVec3> {
    (0..WORLD_HEIGHT)
        .flat_map(|y| (0..16).flat_map(move |z| (0..16).map(move |x| IVec3::new(x, y, z))))
        .filter(|&p| column.get(p) == Block::MobSpawner)
        .collect()
}

fn half_of(column: &Column, center: IVec3) -> Option<IVec3> {
    let run = |dir: IVec3| {
        (0..=4)
            .take_while(|&k| lined(column.get(center + dir * k - IVec3::Y)))
            .count() as i32
    };
    let (x, z) = (run(IVec3::X) - 2, run(IVec3::Z) - 2);
    ([2, 3].contains(&x) && [2, 3].contains(&z)).then(|| IVec3::new(x, 0, z))
}

fn dungeons() -> Vec<(Column, IVec3, IVec3)> {
    let terrain = Terrain::new(SEED);
    let mut found = Vec::new();
    for key in keys() {
        let column = terrain.generate(key);
        for center in spawners(&column) {
            if let Some(half) = half_of(&column, center) {
                found.push((column.clone(), center, half));
            }
        }
    }
    found
}

#[test]
fn dungeons_are_deterministic() {
    let (a, b) = (Terrain::new(SEED), Terrain::new(SEED));
    for key in keys().take(60) {
        let (left, right) = (a.generate(key), b.generate(key));
        assert_eq!(spawners(&left), spawners(&right));
    }
}

#[test]
fn rooms_are_lined_with_cobblestone_and_have_one_to_five_openings() {
    let found = dungeons();
    assert!(!found.is_empty());
    for (column, center, half) in &found {
        let mut openings = 0;
        for at in frame(*center, *half) {
            let dy = at.y - center.y;
            if dy == -1 || dy == 4 {
                assert!(lined(column.get(at)), "{at} {:?}", column.get(at));
            }
            if dy == 0
                && on_perimeter(*center, *half, at)
                && column.get(at + IVec3::Y) == Block::Air
            {
                openings += usize::from(column.get(at) == Block::Air);
                assert!(column.get(at) == Block::Air || lined(column.get(at)));
            }
            let d = at - *center;
            if d.x.abs() <= half.x && d.z.abs() <= half.z && (0..=2).contains(&d.y) {
                let block = column.get(at);
                assert!(
                    matches!(block, Block::Air | Block::Chest | Block::MobSpawner),
                    "{at} {block:?}"
                );
            }
        }
        assert!((1..=5).contains(&openings), "{openings} openings");
    }
}

#[test]
fn chests_stand_by_a_wall_facing_away_from_it() {
    let found = dungeons();
    let mut chests = 0;
    for (column, center, half) in &found {
        for dz in -half.z..=half.z {
            for dx in -half.x..=half.x {
                let at = *center + IVec3::new(dx, 0, dz);
                if column.get(at) != Block::Chest {
                    continue;
                }
                chests += 1;
                let front = chest::front_of(column.get_data(at));
                assert!(column.get(at - front).is_solid(), "{at}");
            }
        }
    }
    assert!(chests > 0);
}

#[test]
fn spawners_hold_a_skeleton_zombie_or_spider() {
    for (column, center, _) in &dungeons() {
        let mob = column.get_data(*center);
        assert!(SPAWNER_MOBS.contains(&mob) && mob != SPAWNER_PIG, "{mob}");
    }
}

#[test]
fn dungeons_appear_in_a_small_share_of_columns() {
    let terrain = Terrain::new(SEED);
    let total = 40 * 40;
    let with = (-20..20)
        .flat_map(|z| (-20..20).map(move |x| ColumnPos::new(x, z)))
        .filter(|&key| !spawners(&terrain.generate(key)).is_empty())
        .count();
    println!("dungeon columns: {with} of {total}");
    assert!(
        (total / 400..=total / 20).contains(&with),
        "{with} of {total}"
    );
}

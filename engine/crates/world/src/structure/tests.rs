use super::*;
use crate::biome::Biome;
use crate::block::Block;
use crate::column::WORLD_HEIGHT;
use crate::terrain::{WATER_LEVEL, WorldType};
use crate::trees::tree_at;

const PROBE_WIDTH: i32 = 23;
const PROBE_HEIGHT: i32 = 6;
const PROBE_DEPTH: i32 = 19;

pub(super) fn layout(
    terrain: &Terrain,
    _kind: StructureKind,
    _variant: Variant,
    anchor: (i32, i32),
    rng: &mut Stream,
) -> Vec<Piece> {
    let facing = Facing::ALL[rng.below(4) as usize];
    let ground = terrain.surface_height(anchor.0, anchor.1);
    let (half_x, half_z) = match facing {
        Facing::North | Facing::South => (PROBE_WIDTH / 2, PROBE_DEPTH / 2),
        Facing::East | Facing::West => (PROBE_DEPTH / 2, PROBE_WIDTH / 2),
    };
    let bounds = BlockBox::new(
        IVec3::new(anchor.0 - half_x, ground + 1, anchor.1 - half_z),
        IVec3::new(anchor.0 + half_x, ground + PROBE_HEIGHT, anchor.1 + half_z),
    );
    vec![Piece {
        bounds,
        facing,
        kind: PieceKind::Probe,
    }]
}

pub(super) fn draw_probe(w: &mut PieceWriter) {
    let (width, depth) = (PROBE_WIDTH, PROBE_DEPTH);
    w.fill_hollow(
        IVec3::ZERO,
        IVec3::new(width - 1, PROBE_HEIGHT - 2, depth - 1),
        Block::Cobblestone,
        Block::Air,
    );
    w.set(width / 2, PROBE_HEIGHT - 1, depth / 2, Block::Glowstone);
    for (x, z) in [
        (0, 0),
        (width - 1, 0),
        (0, depth - 1),
        (width - 1, depth - 1),
    ] {
        w.fill_down(x, -1, z, Block::Cobblestone);
    }
    w.clear_up(width / 2, 1, depth / 2);
}

fn snapshot(column: &Column) -> Vec<(Block, u8)> {
    (0..16 * WORLD_HEIGHT * 16)
        .map(|i| {
            let at = IVec3::new(i % 16, i / 256, i / 16 % 16);
            (column.get(at), column.get_data(at))
        })
        .collect()
}

fn built_terrain(seed: i32, kind: StructureKind) -> Terrain {
    let terrain = Terrain::new(seed);
    terrain.set_structure_built(kind, true);
    terrain
}

fn first_start(terrain: &Terrain, kind: StructureKind) -> ColumnPos {
    for region in 0..400 {
        let center = ColumnPos::new(region % 20 * 40 - 400, region / 20 * 40 - 400);
        let starts = terrain.starts_near(kind, center, 20);
        if let Some(start) = starts.iter().find_map(|start| start.get()) {
            return start.origin;
        }
    }
    panic!("no {kind:?} start found");
}

#[test]
fn facing_rotates_in_four_steps() {
    for facing in Facing::ALL {
        let turned = (0..4).fold(facing, |f, _| f.rotated_right());
        assert_eq!(turned, facing);
        assert_eq!(facing.rotated_right().rotated_left(), facing);
        assert_eq!(facing.opposite().step(), -facing.step());
    }
}

#[test]
fn oriented_box_swaps_extent_for_east_and_west() {
    let origin = IVec3::new(10, 60, 20);
    let offset = IVec3::new(1, 0, 2);
    let size = IVec3::new(5, 4, 7);
    for facing in Facing::ALL {
        let bounds = BlockBox::oriented(origin, offset, size, facing);
        let expected = match facing {
            Facing::North | Facing::South => IVec3::new(5, 4, 7),
            Facing::East | Facing::West => IVec3::new(7, 4, 5),
        };
        assert_eq!(bounds.size(), expected, "{facing:?}");
    }
}

#[test]
fn local_corners_land_on_distinct_corners_of_the_box() {
    let size = IVec3::new(5, 4, 7);
    for facing in Facing::ALL {
        let bounds = BlockBox::oriented(IVec3::new(10, 60, 20), IVec3::new(1, 0, 2), size, facing);
        let piece = Piece {
            bounds,
            facing,
            kind: PieceKind::Probe,
        };
        let mut column = Column::new();
        let writer = PieceWriter::new(&mut column, ColumnPos::ZERO, &piece);
        let corners: Vec<IVec3> = [
            (0, 0),
            (size.x - 1, 0),
            (0, size.z - 1),
            (size.x - 1, size.z - 1),
        ]
        .into_iter()
        .map(|(x, z)| writer.world(x, 0, z))
        .collect();
        for corner in &corners {
            assert!(bounds.contains(*corner), "{facing:?} {corner}");
            assert!(corner.x == bounds.min.x || corner.x == bounds.max.x);
            assert!(corner.z == bounds.min.z || corner.z == bounds.max.z);
        }
        let mut unique = corners.clone();
        unique.sort_by_key(|c| (c.x, c.z));
        unique.dedup();
        assert_eq!(unique.len(), 4, "{facing:?}");
    }
}

#[test]
fn north_piece_grows_toward_negative_z() {
    let bounds = BlockBox::oriented(
        IVec3::new(0, 60, 0),
        IVec3::ZERO,
        IVec3::new(3, 2, 5),
        Facing::North,
    );
    let piece = Piece {
        bounds,
        facing: Facing::North,
        kind: PieceKind::Probe,
    };
    let mut column = Column::new();
    let writer = PieceWriter::new(&mut column, ColumnPos::ZERO, &piece);
    assert_eq!(writer.world(0, 0, 0), IVec3::new(0, 60, 0));
    assert_eq!(writer.world(2, 1, 4), IVec3::new(2, 61, -4));
}

#[test]
fn writer_drops_everything_outside_its_column() {
    let bounds = BlockBox::new(IVec3::new(8, 60, 8), IVec3::new(40, 70, 20));
    let piece = Piece {
        bounds,
        facing: Facing::South,
        kind: PieceKind::Probe,
    };
    let mut column = Column::new();
    let mut writer = PieceWriter::new(&mut column, ColumnPos::ZERO, &piece);
    writer.fill(IVec3::ZERO, IVec3::new(32, 10, 12), Block::Stone);
    assert_eq!(writer.get(40, 0, 0), Block::Air);
    let stone = |x, y, z| column.get(IVec3::new(x, y, z)) == Block::Stone;
    assert!(stone(8, 60, 8) && stone(15, 70, 15));
    assert!(!stone(8, 59, 8) && !stone(8, 71, 8));
    let count = snapshot(&column)
        .iter()
        .filter(|&&(block, _)| block == Block::Stone)
        .count();
    assert_eq!(count, 8 * 11 * 8);
}

#[test]
fn fill_down_passes_air_and_water_and_clear_up_stops_at_air() {
    let bounds = BlockBox::new(IVec3::new(0, 50, 0), IVec3::new(4, 60, 4));
    let piece = Piece {
        bounds,
        facing: Facing::South,
        kind: PieceKind::Probe,
    };
    let mut column = Column::new();
    for y in 0..=45 {
        column.set(IVec3::new(2, y, 2), Block::Stone);
    }
    for y in 46..=48 {
        column.set(IVec3::new(2, y, 2), Block::Water);
    }
    let mut writer = PieceWriter::new(&mut column, ColumnPos::ZERO, &piece);
    writer.fill_down(2, 1, 2, Block::Cobblestone);
    for y in 46..=51 {
        assert_eq!(column.get(IVec3::new(2, y, 2)), Block::Cobblestone, "y {y}");
    }
    assert_eq!(column.get(IVec3::new(2, 45, 2)), Block::Stone);

    column.set(IVec3::new(3, 55, 3), Block::Dirt);
    column.set(IVec3::new(3, 56, 3), Block::Dirt);
    column.set(IVec3::new(3, 58, 3), Block::Dirt);
    let mut writer = PieceWriter::new(&mut column, ColumnPos::ZERO, &piece);
    writer.clear_up(3, 5, 3);
    assert_eq!(column.get(IVec3::new(3, 55, 3)), Block::Air);
    assert_eq!(column.get(IVec3::new(3, 56, 3)), Block::Air);
    assert_eq!(column.get(IVec3::new(3, 58, 3)), Block::Dirt);
}

#[test]
fn grid_starts_keep_spacing_and_separation() {
    for kind in [
        StructureKind::Village,
        StructureKind::Temple,
        StructureKind::Monument,
        StructureKind::Mansion,
    ] {
        let grid = kind.grid().unwrap();
        for seed in [1, 77, -5] {
            let at = |rx, rz| placement::grid_origin(seed, grid, rx, rz);
            for rz in -6..6 {
                for rx in -6..6 {
                    let origin = at(rx, rz);
                    let range = grid.spacing - grid.separation;
                    assert!(
                        (0..range).contains(&(origin.x() - rx * grid.spacing)),
                        "{kind:?}"
                    );
                    assert!(
                        (0..range).contains(&(origin.z() - rz * grid.spacing)),
                        "{kind:?}"
                    );
                    assert!(
                        at(rx + 1, rz).x() - origin.x() >= grid.separation,
                        "{kind:?}"
                    );
                    assert!(
                        at(rx, rz + 1).z() - origin.z() >= grid.separation,
                        "{kind:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn triangular_grid_clusters_toward_region_center() {
    let grid = StructureKind::Mansion.grid().unwrap();
    let flat = StructureKind::Village.grid().unwrap();
    let spread = |grid: placement::Grid| -> f64 {
        let offsets: Vec<f64> = (-20..20)
            .flat_map(|rz| (-20..20).map(move |rx| (rx, rz)))
            .map(|(rx, rz)| {
                let origin = placement::grid_origin(9, grid, rx, rz);
                f64::from(origin.x() - rx * grid.spacing)
                    / f64::from(grid.spacing - grid.separation)
            })
            .collect();
        let mean = offsets.iter().sum::<f64>() / offsets.len() as f64;
        offsets.iter().map(|o| (o - mean).powi(2)).sum::<f64>() / offsets.len() as f64
    };
    assert!(spread(grid) < spread(flat));
}

#[test]
fn starts_are_deterministic_by_seed() {
    let describe = |seed| {
        let terrain = built_terrain(seed, StructureKind::Village);
        let starts: Vec<_> = (-6..6)
            .flat_map(|z| (-6..6).map(move |x| ColumnPos::new(x * 32, z * 32)))
            .flat_map(|center| terrain.starts_near(StructureKind::Village, center, 16))
            .filter_map(|start| start.get().map(|s| (s.origin, s.variant, s.pieces.clone())))
            .collect::<Vec<_>>();
        starts
    };
    assert!(!describe(3).is_empty());
    assert_eq!(describe(3), describe(3));
    assert_ne!(describe(3), describe(4));
}

#[test]
fn villages_only_start_in_village_biomes() {
    let terrain = built_terrain(5, StructureKind::Village);
    let mut found = 0;
    let mut rejected = 0;
    for rz in -12..12 {
        for rx in -12..12 {
            let center = ColumnPos::new(rx * 32 + 16, rz * 32 + 16);
            let candidates = placement::origins(&terrain, StructureKind::Village, center, 16);
            let starts = terrain.starts_near(StructureKind::Village, center, 16);
            rejected += candidates.len() - starts.len();
            for start in starts {
                let start = start.get().unwrap();
                let (x, z) = start.anchor;
                let cell = terrain.biome_map().coarse(x >> 2, z >> 2, 1, 1).data[0];
                let biome = Biome::from_id(cell).unwrap();
                assert!(placement::VILLAGE_BIOMES.contains(&biome), "{x} {z}");
                found += 1;
            }
        }
    }
    assert!(
        found > 0 && rejected > 0,
        "found {found}, rejected {rejected}"
    );
}

#[test]
fn temple_variant_follows_the_biome() {
    let terrain = built_terrain(8, StructureKind::Temple);
    let mut seen = 0;
    for rz in -20..20 {
        for rx in -20..20 {
            let center = ColumnPos::new(rx * 32 + 16, rz * 32 + 16);
            for start in terrain.starts_near(StructureKind::Temple, center, 16) {
                let start = start.get().unwrap();
                let biome = terrain.biome_at(start.anchor.0, start.anchor.1);
                let expected = match biome {
                    Biome::Desert | Biome::DesertHills => TempleKind::DesertPyramid,
                    Biome::Jungle | Biome::JungleHills => TempleKind::JunglePyramid,
                    Biome::Swampland => TempleKind::SwampHut,
                    Biome::IcePlains | Biome::ColdTaiga => TempleKind::Igloo,
                    other => panic!("temple in {other:?}"),
                };
                assert_eq!(start.variant, Variant::Temple(expected));
                seen += 1;
            }
        }
    }
    assert!(seen > 0);
}

#[test]
fn mineshafts_are_rare_and_grow_with_distance() {
    let terrain = Terrain::new(2);
    let count = |cx0: i32, cz0: i32| {
        (cz0..cz0 + 100)
            .flat_map(|cz| (cx0..cx0 + 100).map(move |cx| (cx, cz)))
            .filter(|&(cx, cz)| placement::mineshaft_at(terrain.seed(), cx, cz))
            .count()
    };
    assert!(count(0, 0) < 100 * 100 / 50);
    assert!(count(1000, 1000) > count(0, 0));
    assert!(!placement::mineshaft_at(terrain.seed(), 0, 0));
}

#[test]
fn strongholds_form_rings_around_the_origin() {
    let terrain = Terrain::new(11);
    let ring = terrain.structures().strongholds(&terrain);
    assert_eq!(ring.len(), placement::STRONGHOLD_COUNT);
    let blocks = |p: &ColumnPos| f64::from(p.x()).hypot(f64::from(p.z())) * 16.0;
    const SEARCH_SLACK: f64 = 128.0;
    for p in &ring[..3] {
        assert!(
            (1408.0 - SEARCH_SLACK..=2688.0 + SEARCH_SLACK).contains(&blocks(p)),
            "{p:?}"
        );
    }
    for p in &ring[3..9] {
        assert!(blocks(p) > 4000.0, "{p:?}");
    }
    let again = placement::stronghold_ring(&terrain);
    assert_eq!(again, ring);
}

#[test]
fn unbuilt_kinds_generate_nothing() {
    let terrain = Terrain::new(5);
    for kind in StructureKind::ALL
        .into_iter()
        .filter(|&kind| kind != StructureKind::Temple)
    {
        assert!(!terrain.structure_built(kind));
        assert!(terrain.starts_near(kind, ColumnPos::ZERO, 50).is_empty());
    }
}

#[test]
fn neighbour_columns_agree_in_any_order() {
    let probe = built_terrain(6, StructureKind::Village);
    let origin = first_start(&probe, StructureKind::Village);
    let (ax, az) = probe
        .starts_near(StructureKind::Village, origin, 0)
        .iter()
        .find_map(|s| s.get().map(|s| s.anchor))
        .unwrap();
    let corner = ColumnPos::new((ax + 8) >> 4, (az + 8) >> 4);
    let keys = [
        ColumnPos::new(corner.x() - 1, corner.z() - 1),
        ColumnPos::new(corner.x(), corner.z() - 1),
        ColumnPos::new(corner.x() - 1, corner.z()),
        corner,
    ];
    let generate = |order: &[usize]| {
        let terrain = built_terrain(6, StructureKind::Village);
        let mut out = vec![Vec::new(); 4];
        for &i in order {
            out[i] = snapshot(&terrain.generate(keys[i]));
        }
        out
    };
    let forward = generate(&[0, 1, 2, 3]);
    assert!(forward == generate(&[3, 2, 1, 0]));
    assert!(forward == generate(&[2, 0, 3, 1]));
    let count = |wanted| {
        forward
            .iter()
            .flatten()
            .filter(|&&(block, _)| block == wanted)
            .count()
    };
    assert!(count(Block::Cobblestone) > 100);
    assert_eq!(count(Block::Glowstone), 1);
}

#[test]
fn start_cache_does_not_change_the_result() {
    let terrain = built_terrain(6, StructureKind::Village);
    let origin = first_start(&terrain, StructureKind::Village);
    let first = terrain.starts_near(StructureKind::Village, origin, 0);
    let cached = terrain.starts_near(StructureKind::Village, origin, 0);
    terrain.set_structure_built(StructureKind::Village, true);
    let rebuilt = terrain.starts_near(StructureKind::Village, origin, 0);
    let pieces = |s: &Vec<StartRef>| s[0].get().unwrap().pieces.clone();
    assert_eq!(pieces(&first), pieces(&cached));
    assert_eq!(pieces(&first), pieces(&rebuilt));
}

#[test]
fn trees_do_not_grow_inside_structure_boxes() {
    for seed in 1..40 {
        let terrain = built_terrain(seed, StructureKind::Village);
        let free = Terrain::new(seed);
        for rz in -6..6 {
            for rx in -6..6 {
                let center = ColumnPos::new(rx * 32 + 16, rz * 32 + 16);
                for start in terrain.starts_near(StructureKind::Village, center, 16) {
                    let start = start.get().unwrap();
                    let b = start.pieces[0].bounds;
                    let tree = (b.min.z..=b.max.z)
                        .flat_map(|z| (b.min.x..=b.max.x).map(move |x| (x, z)))
                        .find_map(|(x, z)| tree_at(&terrain, x, z));
                    let Some(tree) = tree else { continue };
                    assert!(terrain.structure_covers(tree.base.x, tree.base.z));
                    let (log, _) = tree.logs()[0];
                    let key = ColumnPos::of_block(log);
                    let local = log - key.origin();
                    assert_eq!(free.generate(key).get(local), tree.kind.log(), "control");
                    assert_ne!(terrain.generate(key).get(local), tree.kind.log());
                    return;
                }
            }
        }
    }
    panic!("no tree inside a structure box");
}

#[test]
fn lakes_skip_columns_touched_by_a_village() {
    let terrain = built_terrain(6, StructureKind::Village);
    let origin = first_start(&terrain, StructureKind::Village);
    let mut checked = 0;
    for dz in -2..=2 {
        for dx in -2..=2 {
            let key = ColumnPos::new(origin.x() + dx, origin.z() + dz);
            if !terrain.village_touches(key) {
                continue;
            }
            checked += 1;
            let at = key.origin();
            assert!(
                (0..256).all(|i| !terrain.lake_covers(at.x + i % 16, at.z + i / 16)),
                "{key:?}"
            );
        }
    }
    assert!(checked > 0);
}

#[test]
fn nearest_structure_finds_closest_matching_start() {
    let terrain = built_terrain(6, StructureKind::Village);
    let origin = first_start(&terrain, StructureKind::Village);
    let name = LocateName::find("village").unwrap();
    let x = origin.x() * 16 + 200;
    let z = origin.z() * 16;
    let (ax, az, distance) = terrain.nearest_structure(name, x, z, 6400).unwrap();
    assert!(distance <= 6400);
    let all: Vec<i64> = terrain
        .starts_near(StructureKind::Village, ColumnPos::new(x >> 4, z >> 4), 300)
        .iter()
        .filter_map(|s| s.get())
        .map(|s| i64::from(s.anchor.0 - x).pow(2) + i64::from(s.anchor.1 - z).pow(2))
        .collect();
    assert_eq!(
        i64::from(ax - x).pow(2) + i64::from(az - z).pow(2),
        *all.iter().min().unwrap()
    );
    assert!(LocateName::find("DESERT_PYRAMID").is_some());
    assert!(LocateName::find("nether_fortress").is_none());
}

#[test]
fn showcase_terrain_is_flat_with_plot_biomes() {
    let terrain = Terrain::of(1, WorldType::Structures);
    let column = terrain.generate(ColumnPos::ZERO);
    let at = |y| column.get(IVec3::new(5, y, 5));
    assert_eq!(at(0), Block::Bedrock);
    assert_eq!(at(30), Block::Stone);
    assert_eq!(at(59), Block::Stone);
    assert_eq!((60..=62).map(at).collect::<Vec<_>>(), vec![Block::Dirt; 3]);
    assert_eq!(at(63), Block::Grass);
    assert_eq!(at(64), Block::Air);
    assert_eq!(terrain.surface_height(5, 5), 63);
    assert_eq!(terrain.biome_at(5, 5), Biome::Plains);
    let all: Vec<_> = plots().collect();
    assert_eq!(all.len(), 13);
    let mut edge = FIRST_PLOT_X;
    for plot in &all {
        assert_eq!(plot.center, glam::IVec2::new(edge + plot.width / 2, 0));
        edge += plot.width;
        let beyond = plot.width / 2;
        assert_eq!(terrain.biome_at(plot.center.x, plot.center.y), plot.biome);
        assert_eq!(
            terrain.surface_height(plot.center.x, plot.center.y),
            plot.surface()
        );
        assert_eq!(terrain.surface_height(plot.center.x, beyond), 63);
        assert_eq!(terrain.biome_at(plot.center.x, beyond), Biome::Plains);
        assert_eq!(
            plot_at(plot.center.x - beyond, 0).unwrap().center,
            plot.center
        );
        assert_ne!(
            plot_at(plot.center.x + beyond, 0).map(|p| p.center),
            Some(plot.center)
        );
    }
}

#[test]
fn showcase_plots_hold_their_structures_and_leave_the_spawn_free() {
    let order: Vec<_> = plots().map(|plot| plot.variant).collect();
    assert_eq!(
        order[..4],
        [
            Variant::Temple(TempleKind::SwampHut),
            Variant::Temple(TempleKind::Igloo),
            Variant::Temple(TempleKind::DesertPyramid),
            Variant::Temple(TempleKind::JunglePyramid),
        ]
    );
    assert!(plot_at(0, 0).is_none());
    let terrain = Terrain::of(1, WorldType::Structures);
    for kind in StructureKind::ALL {
        terrain.set_structure_built(kind, true);
    }
    for plot in plots().filter(|plot| plot.variant.ready()) {
        let start = build(&terrain, plot.kind, plot.column()).unwrap();
        let half = plot.width / 2;
        let (low, high) = (plot.center.x - half, plot.center.x + half);
        assert!(
            start.bounds.min.x >= low && start.bounds.max.x < high,
            "{:?} {:?} {low}..{high}",
            plot.variant,
            start.bounds
        );
        assert!(start.bounds.min.z >= -half && start.bounds.max.z < half);
    }
}

#[test]
fn showcase_monument_plot_is_an_ocean() {
    let terrain = Terrain::of(1, WorldType::Structures);
    let plot = plots()
        .find(|plot| plot.kind == StructureKind::Monument)
        .unwrap();
    let key = ColumnPos::of_block(IVec3::new(plot.center.x, 0, plot.center.y));
    let column = terrain.generate(key);
    let at = |y| column.get(IVec3::new(8, y, 8));
    assert_eq!(
        terrain.surface_height(plot.center.x, plot.center.y),
        SEABED_SURFACE
    );
    assert!(at(SEABED_SURFACE).is_solid());
    assert_eq!(at(SEABED_SURFACE + 1), Block::Water);
    assert_eq!(at(WATER_LEVEL), Block::Water);
    assert_eq!(at(WATER_LEVEL + 1), Block::Air);
}

#[test]
fn showcase_has_no_trees_or_decor() {
    let terrain = Terrain::of(1, WorldType::Structures);
    for plot in plots().filter(|plot| !is_built_temple(plot.variant)) {
        let column = terrain.generate(plot.column());
        for i in 0..16 * 16 {
            let (x, z) = (i % 16, i / 16);
            let top = (0..WORLD_HEIGHT)
                .rev()
                .find(|&y| column.get(IVec3::new(x, y, z)) != Block::Air)
                .unwrap();
            assert!(
                top <= WATER_LEVEL.max(plot.surface()),
                "{:?} {x} {z}",
                plot.variant
            );
        }
    }
}

#[test]
fn showcase_starts_use_the_same_build_as_the_standard_world() {
    let terrain = Terrain::of(1, WorldType::Structures);
    for plot in plots() {
        terrain.set_structure_built(plot.kind, true);
        let start = build(&terrain, plot.kind, plot.column());
        if !plot.variant.ready() {
            assert!(start.is_none(), "{:?}", plot.variant);
            continue;
        }
        let start = start.unwrap();
        assert_eq!(start.variant, plot.variant, "{:?}", plot.biome);
        assert_eq!(start.anchor, (plot.center.x, plot.center.y));
        let listed = terrain.starts_near(plot.kind, plot.column(), 1);
        assert!(
            listed
                .iter()
                .any(|s| s.get().is_some_and(|s| s.origin == start.origin))
        );
        let name = LOCATE_NAMES
            .iter()
            .find(|n| n.kind == plot.kind && n.variant == Some(plot.variant));
        let name = name.copied().unwrap_or_else(|| {
            LOCATE_NAMES
                .iter()
                .copied()
                .find(|n| n.kind == plot.kind)
                .unwrap()
        });
        let found = terrain.nearest_structure(name, plot.center.x, plot.center.y, 6400);
        assert_eq!(found, Some((plot.center.x, plot.center.y, 0)));
    }
}

fn writer_piece(facing: Facing) -> Piece {
    Piece {
        bounds: BlockBox::new(IVec3::new(0, 20, 0), IVec3::new(7, 25, 7)),
        facing,
        kind: PieceKind::Probe,
    }
}

#[test]
fn writer_places_empty_chests_furnaces_cauldrons_and_pots() {
    use crate::chest::{self, HALF_LEFT, HALF_NONE, HALF_RIGHT, front_of, half_of};
    for facing in Facing::ALL {
        let piece = writer_piece(facing);
        let mut column = Column::new();
        let front = IVec3::new(0, 0, -1);
        let (want, cells) = {
            let mut w = PieceWriter::new(&mut column, ColumnPos::ZERO, &piece);
            w.chest(1, 1, 1, front);
            w.double_chest(3, 1, 1, front);
            w.furnace(5, 1, 1, front, true);
            w.cauldron(6, 1, 1, 9);
            w.flower_pot(6, 1, 3, Some(Block::Fern));
            w.flower_pot(6, 1, 4, Some(Block::Stone));
            let at = |x, z| w.world(x, 1, z);
            (
                w.world_dir(front),
                [
                    at(1, 1),
                    at(3, 1),
                    at(3, 1) + chest::right_of(w.world_dir(front)),
                    at(5, 1),
                    at(6, 1),
                    at(6, 3),
                    at(6, 4),
                ],
            )
        };
        let [single, left, right, furnace, cauldron, pot, empty_pot] = cells;
        assert_eq!(column.get(single), Block::Chest);
        assert_eq!(front_of(column.get_data(single)), want);
        assert_eq!(half_of(column.get_data(single)), HALF_NONE);
        assert_eq!(column.chests().count(), 0);
        assert_eq!(half_of(column.get_data(left)), HALF_LEFT);
        assert_eq!(half_of(column.get_data(right)), HALF_RIGHT);
        assert_eq!(column.get(right), Block::Chest);
        assert_eq!(column.get(furnace), Block::LitFurnace);
        assert_eq!(Block::LitFurnace.facing_dir(column.get_data(furnace)), want);
        assert_eq!(column.get_data(cauldron), 3);
        assert_eq!(Block::pot_plant(column.get_data(pot)), Some(Block::Fern));
        assert_eq!(column.get_data(empty_pot), 0);
    }
}

const HUT_SEED: i32 = 1337;

fn is_built_temple(variant: Variant) -> bool {
    matches!(
        variant,
        Variant::Temple(TempleKind::SwampHut | TempleKind::Igloo)
    )
}

fn temples(terrain: &Terrain, variant: Variant, around: ColumnPos) -> Vec<StartRef> {
    let mut found = Vec::new();
    for rz in -12..12 {
        for rx in -12..12 {
            let center = ColumnPos::new(around.x() + rx * 32 + 16, around.z() + rz * 32 + 16);
            found.extend(terrain.starts_near(StructureKind::Temple, center, 16));
        }
    }
    found.retain(|start| start.get().is_some_and(|start| start.variant == variant));
    found.sort_by_key(|start| {
        let origin = start.get().unwrap().origin;
        (origin.x(), origin.z())
    });
    found.dedup_by_key(|start| start.get().unwrap().origin);
    found
}

fn huts(terrain: &Terrain) -> Vec<StartRef> {
    temples(
        terrain,
        Variant::Temple(TempleKind::SwampHut),
        ColumnPos::ZERO,
    )
}

/// Ближайшие к началу координат иглу сида `HUT_SEED` лежат около (−6472, 840).
const IGLOO_REGION: ColumnPos = ColumnPos::new(-405, 52);

fn igloos(terrain: &Terrain) -> Vec<StartRef> {
    temples(terrain, Variant::Temple(TempleKind::Igloo), IGLOO_REGION)
}

fn hut_columns(start: &StructureStart) -> Vec<ColumnPos> {
    let b = start.pieces[0].bounds;
    let (low, high) = (ColumnPos::of_block(b.min), ColumnPos::of_block(b.max));
    (low.z()..=high.z())
        .flat_map(|z| (low.x()..=high.x()).map(move |x| ColumnPos::new(x, z)))
        .collect()
}

fn count_in(columns: &[Column], wanted: Block) -> usize {
    columns
        .iter()
        .flat_map(snapshot)
        .filter(|&(block, _)| block == wanted)
        .count()
}

#[test]
fn huts_only_stand_in_swamps_and_are_deterministic() {
    let terrain = Terrain::new(HUT_SEED);
    let found = huts(&terrain);
    assert!(found.len() >= 2, "{}", found.len());
    for start in &found {
        let start = start.get().unwrap();
        assert_eq!(start.variant, Variant::Temple(TempleKind::SwampHut));
        assert_eq!(
            terrain.biome_at(start.anchor.0, start.anchor.1),
            Biome::Swampland
        );
        assert_eq!(start.pieces.len(), 1);
        assert_eq!(start.pieces[0].kind, PieceKind::SwampHut);
    }
    let again = huts(&Terrain::new(HUT_SEED));
    let key = |starts: &[StartRef]| -> Vec<_> {
        starts
            .iter()
            .map(|s| {
                let s = s.get().unwrap();
                (s.origin, s.pieces[0].bounds, s.pieces[0].facing)
            })
            .collect()
    };
    assert_eq!(key(&found), key(&again));
}

#[test]
fn hut_columns_agree_in_any_order_and_hold_the_hut() {
    let terrain = Terrain::new(HUT_SEED);
    let found = huts(&terrain);
    let start = found
        .iter()
        .map(|s| s.get().unwrap())
        .max_by_key(|s| hut_columns(s).len())
        .unwrap();
    let keys = hut_columns(start);
    let generate = |order: Vec<usize>| {
        let terrain = Terrain::new(HUT_SEED);
        let mut out: Vec<Option<Column>> = keys.iter().map(|_| None).collect();
        for i in order {
            out[i] = Some(terrain.generate(keys[i]));
        }
        out.into_iter().flatten().collect::<Vec<_>>()
    };
    let forward = generate((0..keys.len()).collect());
    let backward = generate((0..keys.len()).rev().collect());
    for (a, b) in forward.iter().zip(&backward) {
        assert!(snapshot(a) == snapshot(b));
    }
    assert!(count_in(&forward, Block::SprucePlanks) > 40);
    assert_eq!(count_in(&forward, Block::CraftingTable), 1);
    assert_eq!(count_in(&forward, Block::Cauldron), 1);
    assert_eq!(count_in(&forward, Block::FlowerPot), 1);
    assert_eq!(count_in(&forward, Block::SpruceStairs), 26);
}

#[test]
fn hut_posts_reach_solid_ground() {
    let terrain = Terrain::new(HUT_SEED);
    for start in huts(&terrain) {
        let start = start.get().unwrap();
        let piece = start.pieces[0];
        for (x, z) in [(1, 2), (5, 2), (1, 7), (5, 7)] {
            let mut scratch = Column::new();
            let writer = PieceWriter::new(&mut scratch, ColumnPos::ZERO, &piece);
            let top = writer.world(x, 0, z);
            let key = ColumnPos::of_block(top);
            let column = terrain.generate(key);
            let mut y = top.y;
            assert_eq!(
                column.get(top - key.origin()),
                Block::Log,
                "{top} {x} {z} {:?} {:?}",
                piece.facing,
                piece.bounds
            );
            while column.get(IVec3::new(
                top.x - key.origin().x,
                y,
                top.z - key.origin().z,
            )) == Block::Log
            {
                y -= 1;
            }
            let below = column.get(IVec3::new(
                top.x - key.origin().x,
                y,
                top.z - key.origin().z,
            ));
            assert!(
                below != Block::Air && !below.is_water(),
                "{below:?} at {top}"
            );
        }
    }
}

#[test]
fn trees_do_not_root_inside_the_hut_box() {
    let terrain = Terrain::new(HUT_SEED);
    for start in huts(&terrain) {
        let b = start.get().unwrap().pieces[0].bounds;
        for z in b.min.z..=b.max.z {
            for x in b.min.x..=b.max.x {
                assert!(terrain.structure_covers(x, z));
            }
        }
    }
}

#[test]
fn locate_finds_only_ready_temple_variants() {
    let terrain = Terrain::new(HUT_SEED);
    let nearest = |starts: Vec<StartRef>| {
        starts
            .into_iter()
            .map(|s| s.get().unwrap().anchor)
            .min_by_key(|a| i64::from(a.0).pow(2) + i64::from(a.1).pow(2))
            .unwrap()
    };
    let hut = nearest(huts(&terrain));
    let igloo = nearest(igloos(&terrain));
    let both = nearest([huts(&terrain), igloos(&terrain)].concat());
    for (name, expected) in [("Swamp_Hut", hut), ("Igloo", igloo), ("Temple", both)] {
        let target = LocateName::find(name).unwrap();
        assert!(target.ready());
        let (x, z, _) = terrain.nearest_structure(target, 0, 0, 20_000).unwrap();
        assert_eq!((x, z), expected, "{name}");
    }
    for name in ["Desert_Pyramid", "Jungle_Pyramid"] {
        let target = LocateName::find(name).unwrap();
        assert!(!target.ready());
        assert_eq!(
            terrain.nearest_structure(target, 0, 0, 20_000),
            None,
            "{name}"
        );
    }
}

#[test]
fn showcase_hut_plot_is_not_empty() {
    let terrain = Terrain::of(HUT_SEED, WorldType::Structures);
    let plot = plots()
        .find(|plot| plot.variant == Variant::Temple(TempleKind::SwampHut))
        .unwrap();
    let start = terrain
        .starts_near(plot.kind, plot.column(), 1)
        .into_iter()
        .find_map(|s| s.get().map(|s| s.origin))
        .expect("hut start on its plot");
    assert_eq!(start, plot.column());
    let start = build(&terrain, plot.kind, plot.column()).unwrap();
    let columns: Vec<Column> = hut_columns(&start)
        .into_iter()
        .map(|key| terrain.generate(key))
        .collect();
    assert!(count_in(&columns, Block::SprucePlanks) > 40);
    let floor = start.pieces[0].bounds.min.y;
    assert_eq!(floor, plot.surface() + 1);
}

fn piece_columns(piece: &Piece) -> Vec<ColumnPos> {
    let b = piece.bounds;
    let (low, high) = (ColumnPos::of_block(b.min), ColumnPos::of_block(b.max));
    (low.z()..=high.z())
        .flat_map(|z| (low.x()..=high.x()).map(move |x| ColumnPos::new(x, z)))
        .collect()
}

fn start_columns(start: &StructureStart) -> Vec<ColumnPos> {
    let mut keys: Vec<ColumnPos> = start.pieces.iter().flat_map(piece_columns).collect();
    keys.sort_by_key(|key| (key.x(), key.z()));
    keys.dedup();
    keys
}

fn igloo_start(terrain: &Terrain, basement: bool) -> StructureStart {
    let found = igloos(terrain);
    let index = found
        .iter()
        .position(|s| (s.get().unwrap().pieces.len() == 2) == basement)
        .unwrap_or_else(|| panic!("no igloo with basement={basement}"));
    let origin = found[index].get().unwrap().origin;
    build(terrain, StructureKind::Temple, origin).unwrap()
}

#[test]
fn igloos_only_stand_in_cold_biomes_and_are_deterministic() {
    let terrain = Terrain::new(HUT_SEED);
    let found = igloos(&terrain);
    assert!(found.len() >= 2, "{}", found.len());
    for start in &found {
        let start = start.get().unwrap();
        assert_eq!(start.variant, Variant::Temple(TempleKind::Igloo));
        assert!(matches!(
            terrain.biome_at(start.anchor.0, start.anchor.1),
            Biome::IcePlains | Biome::ColdTaiga
        ));
        assert_eq!(start.pieces[0].kind, PieceKind::Igloo);
        assert!(matches!(start.pieces.len(), 1 | 2));
    }
    let key = |starts: &[StartRef]| -> Vec<_> {
        starts
            .iter()
            .map(|s| {
                let s = s.get().unwrap();
                (
                    s.origin,
                    s.pieces
                        .iter()
                        .map(|p| (p.bounds, p.facing, p.kind))
                        .collect::<Vec<_>>(),
                )
            })
            .collect()
    };
    assert_eq!(key(&found), key(&igloos(&Terrain::new(HUT_SEED))));
}

#[test]
fn about_half_of_igloos_get_a_basement_of_four_to_eleven_segments() {
    let terrain = Terrain::new(HUT_SEED);
    let mut with = 0;
    let runs = 400;
    for seed in 0..runs {
        let pieces = igloo::layout(&terrain, (0, 0), &mut Stream::new(seed));
        if let Some(piece) = pieces.get(1) {
            with += 1;
            let PieceKind::IglooBasement(segments, _) = piece.kind else {
                panic!("{:?}", piece.kind);
            };
            assert!((4..=11).contains(&segments), "{segments}");
            assert_eq!(piece.bounds.size().y, 3 * i32::from(segments) + 8);
        }
    }
    assert!((160..=240).contains(&with), "{with} of {runs}");
}

#[test]
fn the_basement_hangs_under_the_dome_and_stays_in_the_ground() {
    let terrain = Terrain::new(HUT_SEED);
    for seed in 0..64 {
        let pieces = igloo::layout(&terrain, (100, -40), &mut Stream::new(seed));
        let [top, basement] = pieces[..] else {
            continue;
        };
        assert_eq!(basement.facing, top.facing);
        assert_eq!(basement.bounds.max.y, top.bounds.min.y + 1);
        assert!(basement.bounds.min.y >= 1);
        let (mut scratch, mut other) = (Column::new(), Column::new());
        let a = PieceWriter::new(&mut scratch, ColumnPos::ZERO, &top).world(0, 0, 0);
        let b = PieceWriter::new(&mut other, ColumnPos::ZERO, &basement).world(0, 0, 0);
        assert_eq!((a.x, a.z), (b.x, b.z), "{:?}", top.facing);
    }
}

#[test]
fn local_boxes_invert_the_writer_mapping() {
    let size = IVec3::new(7, 5, 9);
    for facing in Facing::ALL {
        let origin = IVec3::new(-33, 20, 71);
        let bounds = BlockBox::from_local(origin, size, facing);
        let piece = Piece {
            bounds,
            facing,
            kind: PieceKind::Probe,
        };
        let mut column = Column::new();
        let writer = PieceWriter::new(&mut column, ColumnPos::ZERO, &piece);
        assert_eq!(writer.world(0, 0, 0), origin, "{facing:?}");
        assert_eq!(bounds.size().y, size.y);
        for (x, z) in [(6, 0), (0, 8), (6, 8)] {
            assert!(bounds.contains(writer.world(x, 4, z)), "{facing:?}");
        }
    }
}

#[test]
fn igloo_columns_agree_in_any_order_and_hold_the_dome() {
    let terrain = Terrain::new(HUT_SEED);
    let start = igloo_start(&terrain, false);
    let keys = start_columns(&start);
    let generate = |order: Vec<usize>| {
        let terrain = Terrain::new(HUT_SEED);
        let mut out: Vec<Option<Column>> = keys.iter().map(|_| None).collect();
        for i in order {
            out[i] = Some(terrain.generate(keys[i]));
        }
        out.into_iter().flatten().collect::<Vec<_>>()
    };
    let forward = generate((0..keys.len()).collect());
    let backward = generate((0..keys.len()).rev().collect());
    for (a, b) in forward.iter().zip(&backward) {
        assert!(snapshot(a) == snapshot(b));
    }
    assert!(count_in(&forward, Block::SnowBlock) > 60);
    assert_eq!(count_in(&forward, Block::Bed), 2);
    assert_eq!(count_in(&forward, Block::CraftingTable), 1);
    assert_eq!(count_in(&forward, Block::Furnace), 1);
    assert_eq!(count_in(&forward, Block::RedstoneTorch), 1);
    assert_eq!(count_in(&forward, Block::Ice), 3);
    assert_eq!(count_in(&forward, Block::OakTrapdoor), 0);
}

#[test]
fn an_igloo_basement_is_seamless_and_complete() {
    let terrain = Terrain::new(HUT_SEED);
    let start = igloo_start(&terrain, true);
    let PieceKind::IglooBasement(segments, _) = start.pieces[1].kind else {
        panic!();
    };
    let keys = start_columns(&start);
    let generate = |order: Vec<usize>| {
        let terrain = Terrain::new(HUT_SEED);
        let mut out: Vec<Option<Column>> = keys.iter().map(|_| None).collect();
        for i in order {
            out[i] = Some(terrain.generate(keys[i]));
        }
        out.into_iter().flatten().collect::<Vec<_>>()
    };
    let forward = generate((0..keys.len()).collect());
    let backward = generate((0..keys.len()).rev().collect());
    for (a, b) in forward.iter().zip(&backward) {
        assert!(snapshot(a) == snapshot(b));
    }
    let floor = 3 * usize::from(segments) + 6;
    assert_eq!(count_in(&forward, Block::Ladder), floor);
    assert_eq!(count_in(&forward, Block::OakTrapdoor), 1);
    assert_eq!(count_in(&forward, Block::Chest), 1);
    assert_eq!(count_in(&forward, Block::BrewingStand), 1);
    assert_eq!(count_in(&forward, Block::IronBars), 2 * 4);
    let bricks = [
        Block::StoneBricks,
        Block::MossyStoneBricks,
        Block::CrackedStoneBricks,
    ];
    for brick in bricks {
        assert!(count_in(&forward, brick) > 20, "{brick:?}");
    }
    assert!(count_in(&forward, Block::StoneBricks) > count_in(&forward, Block::CrackedStoneBricks));
    let eggs: Vec<u8> = forward
        .iter()
        .flat_map(|column| {
            (0..16 * WORLD_HEIGHT * 16).filter_map(|i| {
                let at = IVec3::new(i % 16, i / 256, i / 16 % 16);
                (column.get(at) == Block::MonsterEgg).then(|| column.get_data(at))
            })
        })
        .collect();
    assert!(!eggs.is_empty());
    assert!(eggs.iter().all(|data| (2..=4).contains(data)), "{eggs:?}");
}

#[test]
fn the_ladder_runs_unbroken_from_the_room_to_the_hatch() {
    let terrain = Terrain::new(HUT_SEED);
    let start = igloo_start(&terrain, true);
    let basement = start.pieces[1];
    let mut scratch = Column::new();
    let writer = PieceWriter::new(&mut scratch, ColumnPos::ZERO, &basement);
    let PieceKind::IglooBasement(segments, _) = basement.kind else {
        panic!();
    };
    let top = 3 * i32::from(segments) + 6;
    let (x, z) = (writer.world(3, 0, 4).x, writer.world(3, 0, 4).z);
    let key = ColumnPos::of_block(IVec3::new(x, 0, z));
    let column = terrain.generate(key);
    let local = |y: i32| {
        IVec3::new(
            x - key.origin().x,
            basement.bounds.min.y + y,
            z - key.origin().z,
        )
    };
    for y in 1..=top {
        assert_eq!(column.get(local(y)), Block::Ladder, "y {y}");
    }
    assert_eq!(column.get(local(top + 1)), Block::OakTrapdoor);
    assert!(column.get(local(0)).is_solid());
    let sky = column.get(local(top + 2));
    assert!(
        sky == Block::Air || sky == Block::Carpet || sky == Block::Snow,
        "{sky:?}"
    );
}

#[test]
fn igloo_pieces_never_write_outside_their_boxes() {
    let terrain = Terrain::new(HUT_SEED);
    for seed in 0..24 {
        let pieces = igloo::layout(&terrain, (-77, 155), &mut Stream::new(seed));
        for piece in pieces {
            let mut cells = 0;
            let keys = piece_columns(&piece);
            let ring = ColumnPos::new(keys[0].x() - 1, keys[0].z() - 1);
            for z in ring.z()..ring.z() + 6 {
                for x in ring.x()..ring.x() + 6 {
                    let key = ColumnPos::new(x, z);
                    let mut column = Column::new();
                    piece.place(&mut PieceWriter::new(&mut column, key, &piece));
                    for (i, (block, _)) in snapshot(&column).into_iter().enumerate() {
                        if block == Block::Air {
                            continue;
                        }
                        let at = key.origin()
                            + IVec3::new(i as i32 % 16, i as i32 / 256, i as i32 / 16 % 16);
                        cells += 1;
                        let below_floor = at.y < piece.bounds.min.y;
                        assert!(
                            piece.bounds.contains(at)
                                || (below_floor && piece.bounds.contains_xz(at.x, at.z)),
                            "{:?} {at} {:?}",
                            piece.kind,
                            piece.bounds
                        );
                    }
                }
            }
            assert!(cells > 50);
        }
    }
}

#[test]
fn locate_finds_the_igloo() {
    let terrain = Terrain::new(HUT_SEED);
    let target = LocateName::find("Igloo").unwrap();
    assert!(target.ready());
    let (x, z, _) = terrain.nearest_structure(target, 0, 0, 20_000).unwrap();
    assert!(matches!(
        terrain.biome_at(x, z),
        Biome::IcePlains | Biome::ColdTaiga
    ));
}

#[test]
fn showcase_igloo_plot_shows_the_dome_over_a_basement() {
    let terrain = Terrain::of(HUT_SEED, WorldType::Structures);
    let plot = plots()
        .find(|plot| plot.variant == Variant::Temple(TempleKind::Igloo))
        .unwrap();
    let start = build(&terrain, plot.kind, plot.column()).unwrap();
    assert_eq!(start.pieces.len(), 2);
    let columns: Vec<Column> = start_columns(&start)
        .into_iter()
        .map(|key| terrain.generate(key))
        .collect();
    assert!(count_in(&columns, Block::SnowBlock) > 60);
    assert!(count_in(&columns, Block::StoneBricks) > 20);
    assert_eq!(start.pieces[0].bounds.min.y, plot.surface());
}

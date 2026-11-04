use super::*;
use crate::tile::DEFAULT_TILE_SIZE;

fn tile_pixels(tile: Tile) -> Vec<u8> {
    TextureSet::standard().tile_pixels(tile)
}

fn icon_pixels(block: Block) -> Option<Vec<u8>> {
    block.icon_pixels(&TextureSet::standard(), 0)
}

const ALL: [Block; 179] = Block::ALL;

fn is_connected_family(block: Block) -> bool {
    block as usize >= Block::OakFence as usize
}

const FLOWERS: [Block; 8] = [
    Block::Allium,
    Block::BlueOrchid,
    Block::Houstonia,
    Block::OxeyeDaisy,
    Block::RedTulip,
    Block::OrangeTulip,
    Block::WhiteTulip,
    Block::PinkTulip,
];

const DOUBLE_HALVES: [Block; 12] = [
    Block::DoubleGrassLower,
    Block::DoubleGrassUpper,
    Block::DoubleFernLower,
    Block::DoubleFernUpper,
    Block::SunflowerLower,
    Block::SunflowerUpper,
    Block::LilacLower,
    Block::LilacUpper,
    Block::RoseBushLower,
    Block::RoseBushUpper,
    Block::PeonyLower,
    Block::PeonyUpper,
];

fn is_new_plant(block: Block) -> bool {
    block == Block::Fern || FLOWERS.contains(&block) || DOUBLE_HALVES.contains(&block)
}

#[test]
fn byte_round_trip() {
    for (i, block) in ALL.into_iter().enumerate() {
        assert_eq!(block as usize, i, "save numbers must not move");
        assert_eq!(Block::try_from(block as u8), Ok(block));
    }
    assert_eq!(Block::try_from(179), Err(179));
    assert_eq!(Block::try_from(255), Err(255));
}

#[test]
fn grass_has_distinct_top_side_bottom() {
    assert_eq!(Block::Grass.tile(Facing::Top), Some(Tile::GrassTop));
    assert_eq!(Block::Grass.tile(Facing::Side), Some(Tile::GrassSide));
    assert_eq!(Block::Grass.tile(Facing::Bottom), Some(Tile::Dirt));
    assert_eq!(Block::Air.tile(Facing::Top), None);
}

#[test]
fn selectable_by_name() {
    assert_eq!(Block::from_name("grass"), Some(Block::Grass));
    assert_eq!(Block::from_name("dirt"), Some(Block::Dirt));
    assert_eq!(Block::from_name("stone"), Some(Block::Stone));
    assert_eq!(Block::from_name("sand"), Some(Block::Sand));
    assert_eq!(Block::from_name("log"), Some(Block::Log));
    assert_eq!(Block::from_name("leaves"), Some(Block::Leaves));
    assert_eq!(Block::from_name("glowstone"), Some(Block::Glowstone));
    assert_eq!(Block::from_name("air"), None);
    assert_eq!(Block::from_name("water"), Some(Block::Water));
    assert_eq!(Block::from_name("Stone"), None);
}

#[test]
fn icon_is_side_face() {
    let set = TextureSet::standard();
    let pixels = |tile| Some(set.icon_pixels(tile));
    assert_eq!(icon_pixels(Block::Grass), pixels(Tile::GrassSide));
    assert_eq!(icon_pixels(Block::Log), pixels(Tile::LogSide));
    assert_eq!(icon_pixels(Block::Stone), pixels(Tile::Stone));
    assert_eq!(icon_pixels(Block::Air), None);
    let grass = icon_pixels(Block::Grass).unwrap();
    let (top, bottom) = (&grass[..3], &grass[grass.len() - 4..][..3]);
    assert_ne!(top, bottom);
}

#[test]
fn solid_and_opaque() {
    for block in ALL {
        if is_connected_family(block) {
            continue;
        }
        let plant = matches!(
            block,
            Block::TallGrass
                | Block::Poppy
                | Block::Dandelion
                | Block::Torch
                | Block::DeadBush
                | Block::Reeds
                | Block::Vine
                | Block::Waterlily
                | Block::BrownMushroom
                | Block::RedMushroom
                | Block::Cocoa
        ) || is_new_plant(block);
        let solid =
            !matches!(block, Block::Air | Block::Water | Block::Lava | Block::Snow) && !plant;
        assert_eq!(block.is_solid(), solid, "{block:?}");
        let opaque = !matches!(
            block,
            Block::Air
                | Block::Leaves
                | Block::BirchLeaves
                | Block::SpruceLeaves
                | Block::JungleLeaves
                | Block::AcaciaLeaves
                | Block::DarkOakLeaves
                | Block::Water
                | Block::Lava
                | Block::Snow
                | Block::Ice
                | Block::Barrier
                | Block::Cactus
                | Block::Glass
        ) && !plant
            && block.shape() != Shape::Boxes;
        assert_eq!(
            block.is_targetable(),
            solid || plant || block == Block::Snow,
            "{block:?}"
        );
        assert_eq!(
            block.needs_support(),
            (plant && block != Block::Vine)
                || matches!(block, Block::Snow | Block::Cactus | Block::Carpet),
            "{block:?}"
        );
        assert_eq!(block.is_opaque(), opaque, "{block:?}");
        assert_eq!(block.is_water(), block == Block::Water, "{block:?}");
    }
}

#[test]
fn render_layers() {
    for block in ALL {
        if is_connected_family(block) {
            continue;
        }
        let layer = match block {
            Block::Leaves
            | Block::BirchLeaves
            | Block::SpruceLeaves
            | Block::JungleLeaves
            | Block::AcaciaLeaves
            | Block::DarkOakLeaves
            | Block::Vine
            | Block::TallGrass
            | Block::Poppy
            | Block::Dandelion
            | Block::Torch
            | Block::Cactus
            | Block::DeadBush
            | Block::Reeds
            | Block::Waterlily
            | Block::BrownMushroom
            | Block::RedMushroom
            | Block::Cocoa
            | Block::Glass => RenderLayer::Cutout,
            block if is_new_plant(block) => RenderLayer::Cutout,
            Block::Water | Block::Ice => RenderLayer::Translucent,
            _ => RenderLayer::Opaque,
        };
        assert_eq!(block.render_layer(), layer, "{block:?}");
    }
    assert_eq!(Block::Water.tile(Facing::Top), Some(Tile::Water));
}

#[test]
fn log_has_rings_on_ends_and_bark_on_sides() {
    assert_eq!(Block::Log.tile(Facing::Top), Some(Tile::LogTop));
    assert_eq!(Block::Log.tile(Facing::Bottom), Some(Tile::LogTop));
    assert_eq!(Block::Log.tile(Facing::Side), Some(Tile::LogSide));
    assert_eq!(Block::Sand.tile(Facing::Top), Some(Tile::Sand));
    assert_eq!(Block::Leaves.tile(Facing::Side), Some(Tile::Leaves));
}

#[test]
fn bounds_follow_shape() {
    assert_eq!(Block::Stone.bounds(0), (Vec3::ZERO, Vec3::ONE));
    assert_eq!(Block::Poppy.bounds(0), (Vec3::ZERO, Vec3::ONE));
    let (lo, hi) = Block::Torch.bounds(0);
    assert_eq!((lo.x, hi.x, hi.y), (7.0 / 16.0, 9.0 / 16.0, 10.0 / 16.0));
}

#[test]
fn replaceable_cells() {
    for block in ALL {
        let expected = matches!(
            block,
            Block::Air | Block::Water | Block::Lava | Block::TallGrass | Block::Snow
        );
        assert_eq!(block.is_replaceable(), expected, "{block:?}");
    }
}

#[test]
fn shapes_and_emission() {
    for block in [Block::TallGrass, Block::Poppy, Block::Dandelion] {
        assert_eq!(block.shape(), Shape::Cross, "{block:?}");
        assert_eq!(block.emission(), 0);
    }
    assert_eq!(Block::Torch.shape(), Shape::Torch);
    assert_eq!(Block::Torch.emission(), 14);
    assert_eq!(Block::Stone.shape(), Shape::Cube);
    assert_eq!(Block::Water.shape(), Shape::Cube);
    assert!(!Block::Air.is_targetable() && !Block::Water.is_targetable());
}

#[test]
fn new_blocks_selectable_by_name() {
    assert_eq!(Block::from_name("tall_grass"), Some(Block::TallGrass));
    assert_eq!(Block::from_name("poppy"), Some(Block::Poppy));
    assert_eq!(Block::from_name("dandelion"), Some(Block::Dandelion));
    assert_eq!(Block::from_name("torch"), Some(Block::Torch));
    for block in [
        Block::TallGrass,
        Block::Poppy,
        Block::Dandelion,
        Block::Torch,
    ] {
        assert!(icon_pixels(block).is_some());
    }
}

#[test]
fn table_rows_follow_save_numbers() {
    assert_eq!(BLOCKS.len(), Block::ALL.len());
    for (i, block) in Block::ALL.into_iter().enumerate() {
        assert_eq!(block as usize, i, "{block:?}");
        assert_eq!(block.def().name, BLOCKS[i].name);
    }
}

#[test]
fn names_are_unique_and_labeled() {
    let mut names: Vec<_> = BLOCKS.iter().map(|d| d.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), BLOCKS.len());
    assert!(
        BLOCKS
            .iter()
            .all(|d| !d.name.is_empty() && !d.label.is_empty())
    );
    assert_eq!(Block::TallGrass.def().label, "высокая трава");
}

#[test]
fn air_has_no_tiles_and_others_have_all() {
    let air = Block::Air.def();
    assert!(air.top.is_none() && air.side.is_none() && air.bottom.is_none());
    for block in Block::ALL.into_iter().skip(1) {
        if block == Block::Barrier {
            continue;
        }
        let def = block.def();
        assert!(def.top.is_some() && def.side.is_some() && def.bottom.is_some());
    }
}

#[test]
fn support_is_for_non_solid_blocks() {
    for def in &BLOCKS {
        assert!(
            !(def.needs_support && def.solid) || matches!(def.shape, Shape::Cactus | Shape::Boxes),
            "{}",
            def.name
        );
        assert!(
            !def.opaque || def.solid && def.shape == Shape::Cube,
            "{}",
            def.name
        );
    }
}

#[test]
fn wall_torch_bounds_lean_away_from_the_wall() {
    use crate::block::{ATTACH_NEG_X, ATTACH_POS_X, ATTACH_POS_Z};
    let u = 1.0 / 16.0;
    let (lo, hi) = Block::Torch.bounds(ATTACH_NEG_X);
    assert_eq!((lo.x, hi.x, lo.y, hi.y), (0.0, 6.0 * u, 3.0 * u, 13.0 * u));
    assert_eq!((lo.z, hi.z), (7.0 * u, 9.0 * u));
    let (lo, hi) = Block::Torch.bounds(ATTACH_POS_X);
    assert_eq!((lo.x, hi.x), (10.0 * u, 1.0));
    let (lo, hi) = Block::Torch.bounds(ATTACH_POS_Z);
    assert_eq!((lo.z, hi.z), (10.0 * u, 1.0));
    assert_eq!((lo.x, hi.x), (7.0 * u, 9.0 * u));
    assert_eq!(Block::Torch.bounds(77), Block::Torch.bounds(0));
}

#[test]
fn support_direction_by_block() {
    use crate::block::ATTACH_POS_Z;
    assert_eq!(Block::Stone.support_dir(0), None);
    assert_eq!(Block::Poppy.support_dir(3), Some(IVec3::NEG_Y));
    assert_eq!(Block::Torch.support_dir(0), Some(IVec3::NEG_Y));
    assert_eq!(Block::Torch.support_dir(ATTACH_POS_Z), Some(IVec3::Z));
    assert_eq!(Block::Torch.support_dir(200), Some(IVec3::NEG_Y));
}

#[test]
fn only_log_torch_and_water_use_data() {
    for block in ALL {
        if is_connected_family(block) {
            continue;
        }
        let expected = match block {
            Block::Log
            | Block::BirchLog
            | Block::SpruceLog
            | Block::JungleLog
            | Block::AcaciaLog
            | Block::DarkOakLog
            | Block::BoneBlock => DataKind::Axis,
            Block::Cocoa => DataKind::Cocoa,
            Block::Vine => DataKind::Sides,
            Block::Pumpkin => DataKind::Facing,
            Block::Torch => DataKind::Attachment,
            Block::Water | Block::Lava => DataKind::Fluid,
            Block::Wool | Block::Carpet => DataKind::Color,
            Block::MonsterEgg => DataKind::Egg,
            block if block.is_slab() => DataKind::Slab,
            block if block.is_stairs() => DataKind::Stairs,
            _ => DataKind::None,
        };
        assert_eq!(block.data_kind(), expected, "{block:?}");
    }
}

#[test]
fn fluid_heights_fall_with_level_and_falling_is_full() {
    use crate::block::{FLUID_FALLING, FLUID_MAX_LEVEL, FLUID_SOURCE, fluid_height_units};
    assert_eq!(fluid_height_units(FLUID_SOURCE), 14);
    assert_eq!(fluid_height_units(FLUID_MAX_LEVEL), 2);
    let heights: Vec<u8> = (0..=FLUID_MAX_LEVEL).map(fluid_height_units).collect();
    assert!(
        heights.windows(2).all(|pair| pair[0] > pair[1]),
        "{heights:?}"
    );
    assert_eq!(fluid_height_units(FLUID_FALLING), 16);
    assert_eq!(fluid_height_units(FLUID_FALLING | 3), 16);
}

#[test]
fn water_is_placed_as_source() {
    assert_eq!(Block::Water.placement_data(IVec3::Y), Some(0));
    assert_eq!(Block::Water.placement_data(IVec3::NEG_X), Some(0));
}

#[test]
fn water_has_icon_for_the_palette() {
    let icon = icon_pixels(Block::Water).expect("water icon");
    assert_eq!(
        icon.len(),
        (DEFAULT_TILE_SIZE * DEFAULT_TILE_SIZE * 4) as usize
    );
}

#[test]
fn lava_is_an_emissive_fluid_and_hardened_blocks_are_cubes() {
    assert_eq!(Block::Lava.fluid(), Some(FluidKind::Lava));
    assert_eq!(Block::Water.fluid(), Some(FluidKind::Water));
    assert_eq!(Block::Stone.fluid(), None);
    assert_eq!(FluidKind::Lava.block(), Block::Lava);
    assert_eq!(Block::Lava.emission(), 15);
    assert!(!Block::Lava.is_solid() && !Block::Lava.is_targetable());
    for block in [Block::Obsidian, Block::Cobblestone] {
        assert!(block.is_solid() && block.is_opaque(), "{block:?}");
    }
    assert_eq!(Block::from_name("lava"), Some(Block::Lava));
    assert_eq!(Block::from_name("obsidian"), Some(Block::Obsidian));
    assert_eq!(Block::from_name("cobblestone"), Some(Block::Cobblestone));
}

#[test]
fn tree_blocks_are_selectable_with_russian_labels() {
    for (name, block, label) in [
        ("birch_log", Block::BirchLog, "берёзовое бревно"),
        ("birch_leaves", Block::BirchLeaves, "берёзовая листва"),
        ("spruce_log", Block::SpruceLog, "еловое бревно"),
        ("spruce_leaves", Block::SpruceLeaves, "хвоя"),
    ] {
        assert_eq!(Block::from_name(name), Some(block));
        assert_eq!(block.def().label, label);
        assert!(icon_pixels(block).is_some(), "{name}");
    }
}

#[test]
fn tree_logs_are_axis_blocks_with_their_own_tiles() {
    for (block, side, end) in [
        (Block::BirchLog, Tile::BirchLogSide, Tile::BirchLogTop),
        (Block::SpruceLog, Tile::SpruceLogSide, Tile::SpruceLogTop),
    ] {
        assert!(block.is_log() && block.is_solid() && block.is_opaque());
        assert_eq!(block.tile(Facing::Side), Some(side));
        assert_eq!(block.tile(Facing::Top), Some(end));
        assert_eq!(block.tile(Facing::Bottom), Some(end));
        assert_eq!(block.placement_data(IVec3::Y), Some(AXIS_Y));
        assert_eq!(block.placement_data(IVec3::NEG_X), Some(AXIS_X));
        assert_eq!(block.placement_data(IVec3::Z), Some(AXIS_Z));
    }
    assert!(Block::Log.is_log() && !Block::Leaves.is_log() && !Block::Stone.is_log());
}

#[test]
fn tree_leaves_are_translucent_cubes_like_oak_leaves() {
    for (block, tile) in [
        (Block::BirchLeaves, Tile::BirchLeaves),
        (Block::SpruceLeaves, Tile::SpruceLeaves),
    ] {
        assert!(block.is_solid() && !block.is_opaque());
        assert_eq!(block.render_layer(), RenderLayer::Cutout);
        assert_eq!(block.tile(Facing::Side), Some(tile));
        assert!(tile.is_cutout());
        assert_eq!(block.emission(), 0);
    }
}

#[test]
fn only_bedrock_is_unbreakable() {
    for block in Block::ALL {
        assert_eq!(block.is_unbreakable(), block == Block::Bedrock, "{block:?}");
    }
    let def = Block::Bedrock.def();
    assert!(def.solid && def.opaque && def.selectable);
    assert_eq!(def.label, "коренная порода");
    assert_eq!(Block::from_name("bedrock"), Some(Block::Bedrock));
}

#[test]
fn snow_is_a_thin_replaceable_layer_on_support() {
    let snow = Block::Snow;
    assert_eq!(snow.shape(), Shape::Layer);
    assert!(!snow.is_solid() && !snow.is_opaque());
    assert!(snow.needs_support() && snow.is_replaceable() && snow.is_targetable());
    assert_eq!(snow.support_dir(0), Some(IVec3::NEG_Y));
    assert_eq!(snow.collision_height(), 2.0 / 16.0);
    assert_eq!(
        snow.bounds(0),
        (Vec3::ZERO, Vec3::new(1.0, 2.0 / 16.0, 1.0))
    );
    assert_eq!(snow.render_layer(), RenderLayer::Opaque);
    assert_eq!(Block::from_name("snow"), Some(Block::Snow));
}

#[test]
fn collision_height_is_full_for_solids_only() {
    for block in ALL {
        if is_connected_family(block) {
            continue;
        }
        let expected = match block {
            Block::Snow => 2.0 / 16.0,
            Block::Cactus => 15.0 / 16.0,
            block if block.is_solid() => 1.0,
            _ => 0.0,
        };
        assert_eq!(block.collision_height(), expected, "{block:?}");
    }
}

#[test]
fn only_ice_is_slippery() {
    for block in ALL {
        assert_eq!(
            block.is_slippery(),
            matches!(block, Block::Ice | Block::IcePacked),
            "{block:?}"
        );
    }
    let ice = Block::Ice;
    assert!(ice.is_solid() && !ice.is_opaque());
    assert_eq!(ice.render_layer(), RenderLayer::Translucent);
    assert_eq!(Block::from_name("ice"), Some(Block::Ice));
}

#[test]
fn snowy_grass_has_snow_top_striped_side_and_dirt_bottom() {
    let block = Block::SnowyGrass;
    assert_eq!(block.tile(Facing::Top), Some(Tile::Snow));
    assert_eq!(block.tile(Facing::Side), Some(Tile::SnowyGrassSide));
    assert_eq!(block.tile(Facing::Bottom), Some(Tile::Dirt));
    assert_eq!(block.random_ticks(), Block::Grass.random_ticks());
    assert!(block.is_grass() && Block::Grass.is_grass() && !Block::Dirt.is_grass());
    assert_eq!(Block::from_name("snowy_grass"), Some(Block::SnowyGrass));
}

#[test]
fn snow_tile_is_whitish_and_ice_is_light_blue() {
    let mean = |tile| {
        let pixels = tile_pixels(tile);
        let n = (pixels.len() / 4) as u32;
        [0, 1, 2].map(|c| pixels.chunks(4).map(|p| u32::from(p[c])).sum::<u32>() / n)
    };
    let [r, g, b] = mean(Tile::Snow);
    assert!(r > 215 && g > 220 && b > 230 && b >= r, "{r} {g} {b}");
    let [r, g, b] = mean(Tile::Ice);
    assert!(b > r + 40 && g > r, "{r} {g} {b}");
}

#[test]
fn snowy_side_has_white_stripe_over_dirt() {
    let pixels = tile_pixels(Tile::SnowyGrassSide);
    let row_bytes = DEFAULT_TILE_SIZE as usize * 4;
    let row = |y: usize| &pixels[y * row_bytes..(y + 1) * row_bytes];
    assert!(row(0).chunks(4).all(|p| p[0] > 190 && p[2] > 220));
    let bottom = row(DEFAULT_TILE_SIZE as usize - 1);
    let mean = |c: usize| bottom.chunks(4).map(|p| u32::from(p[c])).sum::<u32>();
    assert!(mean(2) < mean(0));
}

#[test]
fn barrier_is_solid_invisible_and_light_transparent() {
    let def = Block::Barrier.def();
    assert!(def.solid && def.targetable && !def.opaque && !def.selectable);
    assert_eq!(Block::Barrier.emission(), 0);
    assert_eq!(Block::from_name("barrier"), None);
    assert!(
        [Facing::Top, Facing::Bottom, Facing::Side]
            .into_iter()
            .all(|facing| Block::Barrier.tile(facing).is_none())
    );
}

#[test]
fn states_cover_every_data_kind() {
    for block in Block::ALL {
        let states: Vec<u8> = block.states().collect();
        let expected = match block.data_kind() {
            DataKind::None => 1,
            DataKind::Axis => 3,
            DataKind::Attachment => 5,
            DataKind::Fluid => usize::from(FLUID_MAX_LEVEL) + 2,
            DataKind::Sides => 5,
            DataKind::Facing => 4,
            DataKind::Cocoa => usize::from(COCOA_STAGES) * 4,
            DataKind::Color => 16,
            DataKind::Slab => 3,
            DataKind::Stairs | DataKind::Gate | DataKind::Door => 8,
            DataKind::Hinge => 2,
            DataKind::Ladder => 4,
            DataKind::Rail => 10,
            DataKind::Chest => 12,
            DataKind::Furnace | DataKind::Cauldron => 4,
            DataKind::Pot => 16,
            DataKind::Spawner => 4,
            DataKind::Bed => 128,
            DataKind::Trapdoor => 16,
            DataKind::Egg => 6,
        };
        assert_eq!(states.len(), expected, "{block:?}");
        let mut unique = states.clone();
        unique.dedup();
        assert_eq!(unique.len(), states.len());
    }
    assert!(Block::Water.states().any(|d| d & FLUID_FALLING != 0));
}

#[test]
fn desert_blocks_are_appended_after_the_barrier() {
    let names = [
        "cactus",
        "dead_bush",
        "red_sand",
        "terracotta",
        "white_terracotta",
        "orange_terracotta",
        "yellow_terracotta",
        "brown_terracotta",
        "red_terracotta",
        "silver_terracotta",
        "reeds",
        "sandstone",
        "red_sandstone",
    ];
    for (i, name) in names.into_iter().enumerate() {
        assert_eq!(
            Block::from_name(name),
            Some(ALL[Block::Barrier as usize + 1 + i])
        );
    }
}

#[test]
fn red_sand_falls_like_sand_and_sandstone_does_not() {
    assert!(Block::RedSand.falls() && Block::Sand.falls());
    assert!(!Block::Sandstone.falls() && !Block::RedSandstone.falls());
    assert!(!Block::Terracotta.falls());
}

#[test]
fn cactus_box_is_narrower_and_lower_than_a_cube() {
    let (lo, hi) = Block::Cactus.bounds(0);
    let u = 1.0 / 16.0;
    assert_eq!((lo.x, lo.z, hi.x, hi.z), (u, u, 15.0 * u, 15.0 * u));
    assert_eq!((lo.y, hi.y), (0.0, 15.0 * u));
    assert!(Block::Cactus.is_solid() && Block::Cactus.is_targetable());
    assert!(!Block::Cactus.is_opaque());
}

#[test]
fn sandstone_has_distinct_top_side_bottom() {
    assert_eq!(Block::Sandstone.tile(Facing::Top), Some(Tile::SandstoneTop));
    assert_eq!(
        Block::Sandstone.tile(Facing::Side),
        Some(Tile::SandstoneSide)
    );
    assert_eq!(
        Block::RedSandstone.tile(Facing::Bottom),
        Some(Tile::RedSandstoneBottom)
    );
    assert_eq!(Block::Cactus.tile(Facing::Top), Some(Tile::CactusTop));
}

#[test]
fn only_dead_bush_and_reeds_are_new_crosses() {
    for block in ALL {
        if is_connected_family(block) {
            continue;
        }
        assert_eq!(
            block.shape() == Shape::Cross,
            matches!(
                block,
                Block::TallGrass
                    | Block::Poppy
                    | Block::Dandelion
                    | Block::DeadBush
                    | Block::Reeds
                    | Block::BrownMushroom
                    | Block::RedMushroom
            ) || (is_new_plant(block) && block != Block::SunflowerUpper),
            "{block:?}"
        );
    }
}

#[test]
fn jungle_savanna_and_roofed_blocks_are_appended_after_red_sandstone() {
    let names = [
        "jungle_log",
        "jungle_leaves",
        "acacia_log",
        "acacia_leaves",
        "dark_oak_log",
        "dark_oak_leaves",
        "vine",
        "melon",
        "brown_mushroom_block",
        "red_mushroom_block",
        "mushroom_stem",
    ];
    for (i, name) in names.into_iter().enumerate() {
        let block = ALL[Block::RedSandstone as usize + 1 + i];
        assert_eq!(block.def().name, name);
        assert_eq!(Block::from_name(name), Some(block));
    }
}

#[test]
fn new_logs_have_axes_and_new_leaves_are_foliage_cutouts() {
    for (log, side, end) in [
        (Block::JungleLog, Tile::JungleLogSide, Tile::JungleLogTop),
        (Block::AcaciaLog, Tile::AcaciaLogSide, Tile::AcaciaLogTop),
        (Block::DarkOakLog, Tile::DarkOakLogSide, Tile::DarkOakLogTop),
    ] {
        assert!(log.is_log());
        assert_eq!(log.tile(Facing::Side), Some(side));
        assert_eq!(log.tile(Facing::Top), Some(end));
        assert_eq!(log.tile(Facing::Bottom), Some(end));
    }
    for leaves in [
        Block::JungleLeaves,
        Block::AcaciaLeaves,
        Block::DarkOakLeaves,
    ] {
        assert!(leaves.is_leaves() && leaves.holds_vine());
        assert!(!leaves.is_opaque() && leaves.is_solid());
        assert_eq!(leaves.render_layer(), RenderLayer::Cutout);
    }
    assert!(!Block::Leaves.is_log() && Block::Leaves.is_leaves() && !Block::Dirt.is_leaves());
}

#[test]
fn melon_and_mushroom_blocks_are_opaque_cubes_with_their_faces() {
    assert_eq!(Block::Melon.tile(Facing::Top), Some(Tile::MelonTop));
    assert_eq!(Block::Melon.tile(Facing::Side), Some(Tile::MelonSide));
    for (block, skin) in [
        (Block::BrownMushroomBlock, Tile::MushroomSkinBrown),
        (Block::RedMushroomBlock, Tile::MushroomSkinRed),
    ] {
        assert_eq!(block.tile(Facing::Top), Some(skin));
        assert_eq!(block.tile(Facing::Side), Some(skin));
        assert_eq!(block.tile(Facing::Bottom), Some(Tile::MushroomInside));
    }
    assert_eq!(
        Block::MushroomStem.tile(Facing::Side),
        Some(Tile::MushroomStem)
    );
    assert_eq!(
        Block::MushroomStem.tile(Facing::Top),
        Some(Tile::MushroomInside)
    );
    for block in [
        Block::Melon,
        Block::BrownMushroomBlock,
        Block::RedMushroomBlock,
        Block::MushroomStem,
    ] {
        assert!(block.is_opaque() && block.is_solid() && block.shape() == Shape::Cube);
    }
}

#[test]
fn vine_is_a_non_solid_targetable_side_mask() {
    use crate::block::{VINE_NEG_X, VINE_POS_X, VINE_TOP};
    let vine = Block::Vine;
    assert_eq!(vine.shape(), Shape::Vine);
    assert_eq!(vine.data_kind(), DataKind::Sides);
    assert!(!vine.is_solid() && !vine.is_opaque() && vine.is_targetable());
    assert!(vine.random_ticks() && !vine.is_replaceable());
    assert_eq!(vine.collision_height(), 0.0);
    assert_eq!(vine.tile(Facing::Side), Some(Tile::Vine));
    assert_eq!(vine.placement_data(IVec3::X), Some(VINE_NEG_X));
    assert_eq!(vine.placement_data(IVec3::NEG_X), Some(VINE_POS_X));
    assert_eq!(vine.placement_data(IVec3::NEG_Y), Some(VINE_TOP));
    assert_eq!(vine.placement_data(IVec3::Y), None);
}

#[test]
fn vine_bounds_hug_the_attached_sides() {
    use crate::block::{VINE_INSET, VINE_NEG_X, VINE_POS_X, VINE_POS_Z, VINE_TOP};
    let (min, max) = Block::Vine.bounds(VINE_POS_X);
    assert_eq!(
        (min, max),
        (Vec3::new(1.0 - VINE_INSET, 0.0, 0.0), Vec3::ONE)
    );
    let (min, max) = Block::Vine.bounds(VINE_NEG_X);
    assert_eq!((min, max), (Vec3::ZERO, Vec3::new(VINE_INSET, 1.0, 1.0)));
    let (min, max) = Block::Vine.bounds(VINE_TOP | VINE_POS_Z);
    assert_eq!(
        min,
        Vec3::new(0.0, 1.0 - VINE_INSET, 1.0 - VINE_INSET).min(Vec3::new(0.0, 0.0, 0.0))
    );
    assert_eq!(max, Vec3::ONE);
}

#[test]
fn swamp_and_island_blocks_are_declared() {
    assert_eq!(Block::from_name("waterlily"), Some(Block::Waterlily));
    assert_eq!(Block::from_name("mycelium"), Some(Block::Mycelium));
    assert_eq!(Block::from_name("clay"), Some(Block::Clay));
    assert_eq!(Block::Mycelium.tile(Facing::Top), Some(Tile::MyceliumTop));
    assert_eq!(Block::Mycelium.tile(Facing::Side), Some(Tile::MyceliumSide));
    assert_eq!(Block::Mycelium.tile(Facing::Bottom), Some(Tile::Dirt));
    assert!(Block::Mycelium.random_ticks() && Block::Mycelium.is_opaque());
    assert_eq!(Block::Clay.tile(Facing::Top), Some(Tile::Clay));
    assert_eq!(
        Block::BrownMushroom.tile(Facing::Side),
        Some(Tile::BrownMushroom)
    );
    assert_eq!(
        Block::RedMushroom.tile(Facing::Side),
        Some(Tile::RedMushroom)
    );
    for block in [Block::BrownMushroom, Block::RedMushroom] {
        assert_eq!(block.shape(), Shape::Cross);
        assert!(block.is_mushroom() && block.needs_support() && !block.is_solid());
    }
}

#[test]
fn lily_floats_on_water_only() {
    let lily = Block::Waterlily;
    assert_eq!(lily.shape(), Shape::Lily);
    assert!(lily.stands_on(Block::Water));
    for support in [
        Block::Stone,
        Block::Dirt,
        Block::Air,
        Block::Lava,
        Block::Ice,
    ] {
        assert!(!lily.stands_on(support), "{support:?}");
    }
    let (min, max) = lily.bounds(0);
    assert_eq!((min, max), (Vec3::ZERO, Vec3::new(1.0, LILY_HEIGHT, 1.0)));
    assert_eq!(lily.collision_height(), 0.0);
}

#[test]
fn mushrooms_stand_on_any_solid_block() {
    for mushroom in [Block::BrownMushroom, Block::RedMushroom] {
        for support in [Block::Mycelium, Block::Stone, Block::Dirt, Block::Grass] {
            assert!(mushroom.stands_on(support), "{support:?}");
        }
        assert!(!mushroom.stands_on(Block::Water) && !mushroom.stands_on(Block::Air));
    }
}

#[test]
fn stage_five_blocks_are_appended_after_clay() {
    let names = [
        "ice_packed",
        "podzol",
        "coarse_dirt",
        "mossy_cobblestone",
        "fern",
        "double_grass_lower",
        "double_grass_upper",
        "double_fern_lower",
        "double_fern_upper",
        "sunflower_lower",
        "sunflower_upper",
        "lilac_lower",
        "lilac_upper",
        "rose_bush_lower",
        "rose_bush_upper",
        "peony_lower",
        "peony_upper",
        "allium",
        "blue_orchid",
        "houstonia",
        "oxeye_daisy",
        "red_tulip",
        "orange_tulip",
        "white_tulip",
        "pink_tulip",
    ];
    for (i, name) in names.into_iter().enumerate() {
        assert_eq!(BLOCKS[Block::Clay as usize + 1 + i].name, name);
    }
}

#[test]
fn packed_ice_is_an_opaque_slippery_cube_and_ice_stays_translucent() {
    let ice = Block::IcePacked;
    assert!(ice.is_solid() && ice.is_opaque() && ice.is_slippery());
    assert_eq!(ice.render_layer(), RenderLayer::Opaque);
    assert_eq!(ice.tile(Facing::Side), Some(Tile::IcePacked));
    assert_eq!(Block::Ice.render_layer(), RenderLayer::Translucent);
}

#[test]
fn podzol_and_coarse_dirt_are_plain_solid_cubes() {
    assert_eq!(Block::Podzol.tile(Facing::Top), Some(Tile::PodzolTop));
    assert_eq!(Block::Podzol.tile(Facing::Side), Some(Tile::PodzolSide));
    assert_eq!(Block::Podzol.tile(Facing::Bottom), Some(Tile::Dirt));
    assert_eq!(Block::CoarseDirt.tile(Facing::Top), Some(Tile::CoarseDirt));
    assert_eq!(
        Block::MossyCobblestone.tile(Facing::Side),
        Some(Tile::MossyCobblestone)
    );
    for block in [Block::Podzol, Block::CoarseDirt, Block::MossyCobblestone] {
        assert!(block.is_solid() && block.is_opaque() && !block.is_slippery());
    }
}

#[test]
fn double_plants_pair_up_and_only_the_sunflower_top_has_a_head() {
    for lower in Block::DOUBLE_PLANTS {
        let upper = lower.double_upper().expect("upper half");
        assert_eq!(upper.double_lower(), Some(lower));
        assert!(!lower.is_solid() && !upper.is_solid());
        assert!(upper.stands_on(lower) && !upper.stands_on(Block::Grass));
        assert!(lower.stands_on(Block::Grass));
        assert!(!lower.def().selectable && !upper.def().selectable);
        assert_eq!(
            upper.shape() == Shape::Sunflower,
            upper == Block::SunflowerUpper
        );
    }
    assert_eq!(Block::Fern.double_upper(), None);
}

#[test]
fn flowers_are_selectable_non_solid_crosses_with_their_own_tiles() {
    let tiles: Vec<_> = FLOWERS.iter().map(|b| b.tile(Facing::Side)).collect();
    for (i, flower) in FLOWERS.into_iter().enumerate() {
        assert!(flower.def().selectable && !flower.is_solid() && flower.needs_support());
        assert!(!tiles[..i].contains(&tiles[i]), "{flower:?}");
    }
}

#[test]
fn pumpkin_faces_the_side_it_was_placed_against() {
    use crate::block::{FACING_NEG_X, FACING_NEG_Z, FACING_POS_X, FACING_POS_Z};
    let pumpkin = Block::Pumpkin;
    assert_eq!(pumpkin.data_kind(), DataKind::Facing);
    assert!(pumpkin.is_opaque() && pumpkin.is_solid());
    assert_eq!(pumpkin.tile(Facing::Top), Some(Tile::PumpkinTop));
    assert_eq!(pumpkin.tile(Facing::Side), Some(Tile::PumpkinSide));
    for (normal, data) in [
        (IVec3::X, FACING_POS_X),
        (IVec3::NEG_X, FACING_NEG_X),
        (IVec3::Z, FACING_POS_Z),
        (IVec3::NEG_Z, FACING_NEG_Z),
    ] {
        assert_eq!(pumpkin.placement_data(normal), Some(data));
        assert_eq!(pumpkin.facing_dir(data), normal);
    }
    assert_eq!(pumpkin.placement_data(IVec3::Y), Some(FACING_NEG_Z));
}

#[test]
fn cocoa_hangs_on_the_jungle_log_it_faces() {
    use crate::block::{COCOA_STAGE_SHIFT, FACING_NEG_X, FACING_POS_Z};
    let cocoa = Block::Cocoa;
    assert!(!cocoa.is_solid() && !cocoa.is_opaque() && cocoa.needs_support());
    assert!(cocoa.stands_on(Block::JungleLog) && !cocoa.stands_on(Block::Log));
    assert_eq!(cocoa.placement_data(IVec3::X), Some(FACING_NEG_X));
    assert_eq!(cocoa.placement_data(IVec3::NEG_Z), Some(FACING_POS_Z));
    assert_eq!(cocoa.placement_data(IVec3::Y), None);
    let data = 2 << COCOA_STAGE_SHIFT | FACING_NEG_X;
    assert_eq!(cocoa.support_dir(data), Some(IVec3::NEG_X));
    let (lo, hi) = cocoa.bounds(data);
    assert_eq!((lo.x, hi.x), (1.0 / 16.0, 9.0 / 16.0));
    assert_eq!((hi.y - lo.y) * 16.0, 9.0);
}

#[test]
fn cocoa_pods_grow_with_the_stage() {
    let sizes: Vec<(f32, f32)> = (0..COCOA_STAGES)
        .map(|stage| {
            let (lo, hi) = Block::Cocoa.bounds(stage << COCOA_STAGE_SHIFT | FACING_POS_Z);
            ((hi.x - lo.x) * 16.0, (hi.y - lo.y) * 16.0)
        })
        .collect();
    assert_eq!(sizes, [(4.0, 5.0), (6.0, 7.0), (8.0, 9.0)]);
}

#[test]
fn snow_block_monster_egg_and_bone_block_look_like_their_sources() {
    assert_eq!(Block::SnowBlock.tile(Facing::Top), Some(Tile::Snow));
    assert!(Block::SnowBlock.is_opaque() && Block::SnowBlock.shape() == Shape::Cube);
    assert_ne!(Block::SnowBlock, Block::Snow);
    assert_eq!(Block::MonsterEgg.tile(Facing::Side), Some(Tile::Stone));
    assert!(!Block::MonsterEgg.is_stone());
    assert!(Block::BoneBlock.is_log());
    assert_eq!(Block::BoneBlock.tile(Facing::Top), Some(Tile::BoneBlockTop));
    assert_eq!(
        Block::BoneBlock.tile(Facing::Side),
        Some(Tile::BoneBlockSide)
    );
}

fn no_neighbors(_: IVec3) -> (Block, u8) {
    (Block::Air, 0)
}

fn stairs_at(facing: u8, top: bool) -> u8 {
    facing | if top { STAIRS_TOP } else { 0 }
}

fn stair_with(neighbors: &[(IVec3, u8)]) -> impl Fn(IVec3) -> (Block, u8) + use<'_> {
    move |offset| {
        neighbors
            .iter()
            .find(|(at, _)| *at == offset)
            .map_or((Block::Air, 0), |&(_, data)| (Block::OakStairs, data))
    }
}

fn volume(block: Block, data: u8, around: impl Fn(IVec3) -> (Block, u8)) -> u32 {
    block
        .boxes(data, around)
        .iter()
        .map(|b| {
            (0..3)
                .map(|k| u32::from(b.max[k] - b.min[k]))
                .product::<u32>()
        })
        .sum()
}

#[test]
fn slab_boxes_for_all_three_halves() {
    let boxes = |data| Block::OakSlab.boxes(data, no_neighbors);
    assert_eq!(
        boxes(SLAB_BOTTOM).as_slice(),
        [VoxelBox::new([0, 0, 0], [16, 8, 16])]
    );
    assert_eq!(
        boxes(SLAB_TOP).as_slice(),
        [VoxelBox::new([0, 8, 0], [16, 16, 16])]
    );
    assert_eq!(boxes(SLAB_DOUBLE).as_slice(), [VoxelBox::FULL]);
}

#[test]
fn carpet_is_one_sixteenth_high() {
    let list = Block::Carpet.boxes(3, no_neighbors);
    assert_eq!(list.as_slice(), [VoxelBox::new([0, 0, 0], [16, 1, 16])]);
    assert_eq!(list.top(), 1.0 / 16.0);
}

#[test]
fn straight_stairs_have_the_high_half_on_the_facing_side() {
    let east = Block::OakStairs.boxes(stairs_at(FACING_POS_X, false), no_neighbors);
    assert_eq!(
        east.as_slice(),
        [
            VoxelBox::new([0, 0, 0], [16, 8, 16]),
            VoxelBox::new([8, 8, 0], [16, 16, 16]),
        ]
    );
    let north = Block::OakStairs.boxes(stairs_at(FACING_NEG_Z, false), no_neighbors);
    assert_eq!(north.as_slice()[1], VoxelBox::new([0, 8, 0], [16, 16, 8]));
    let west_top = Block::OakStairs.boxes(stairs_at(FACING_NEG_X, true), no_neighbors);
    assert_eq!(
        west_top.as_slice()[0],
        VoxelBox::new([0, 8, 0], [16, 16, 16])
    );
    assert_eq!(west_top.as_slice()[1], VoxelBox::new([0, 0, 0], [8, 8, 16]));
}

#[test]
fn stair_corners_follow_the_neighbors() {
    let data = stairs_at(FACING_POS_X, false);
    let at = |neighbors: &[(IVec3, u8)]| Block::OakStairs.stair_shape(data, stair_with(neighbors));
    assert_eq!(at(&[]), StairShape::Straight);
    // сосед за высокой стороной смотрит на север, то есть влево от взгляда на восток
    assert_eq!(
        at(&[(IVec3::X, stairs_at(FACING_NEG_Z, false))]),
        StairShape::OuterLeft
    );
    assert_eq!(
        at(&[(IVec3::X, stairs_at(FACING_POS_Z, false))]),
        StairShape::OuterRight
    );
    assert_eq!(
        at(&[(IVec3::NEG_X, stairs_at(FACING_NEG_Z, false))]),
        StairShape::InnerLeft
    );
    assert_eq!(
        at(&[(IVec3::NEG_X, stairs_at(FACING_POS_Z, false))]),
        StairShape::InnerRight
    );
    assert_eq!(
        at(&[(IVec3::X, stairs_at(FACING_NEG_Z, true))]),
        StairShape::Straight
    );
    assert_eq!(
        at(&[(IVec3::X, stairs_at(FACING_POS_X, false))]),
        StairShape::Straight
    );
}

#[test]
fn stair_corner_boxes_have_the_expected_volume() {
    let data = stairs_at(FACING_POS_X, false);
    let volume_with =
        |neighbors: &[(IVec3, u8)]| volume(Block::OakStairs, data, stair_with(neighbors));
    let slab = 16 * 8 * 16;
    assert_eq!(volume_with(&[]), slab + 8 * 8 * 16);
    let outer = volume_with(&[(IVec3::X, stairs_at(FACING_NEG_Z, false))]);
    assert_eq!(outer, slab + 8 * 8 * 8);
    let inner = volume_with(&[(IVec3::NEG_X, stairs_at(FACING_NEG_Z, false))]);
    assert_eq!(inner, slab + 8 * 8 * 16 + 8 * 8 * 8);
}

#[test]
fn inner_corner_adds_the_front_quarter_on_the_neighbor_side() {
    let data = stairs_at(FACING_POS_X, false);
    let list = Block::OakStairs.boxes(
        data,
        stair_with(&[(IVec3::NEG_X, stairs_at(FACING_NEG_Z, false))]),
    );
    assert_eq!(list.len(), 3);
    assert_eq!(list.as_slice()[2], VoxelBox::new([0, 8, 0], [8, 16, 8]));
}

#[test]
fn slab_and_stairs_block_light_but_are_not_opaque() {
    for block in [Block::OakSlab, Block::OakStairs, Block::StoneSlab] {
        assert!(!block.is_opaque() && block.blocks_light(), "{block:?}");
        assert!(block.lights_from_neighbors());
    }
    assert!(!Block::Carpet.blocks_light());
    assert!(!Block::Glass.blocks_light() && !Block::Glass.is_opaque());
    assert!(Block::Stone.blocks_light() && !Block::Stone.lights_from_neighbors());
}

fn placement(normal: IVec3, hit_y: f32, look: IVec3) -> Placement {
    Placement {
        normal,
        hit_x: 0.5,
        hit_y,
        hit_z: 0.5,
        look,
        variant: 0,
    }
}

#[test]
fn slab_half_follows_the_clicked_face_and_height() {
    let state = |normal, hit_y| {
        Block::OakSlab
            .placement_state(&placement(normal, hit_y, IVec3::X))
            .unwrap()
    };
    assert_eq!(state(IVec3::Y, 0.0), SLAB_BOTTOM);
    assert_eq!(state(IVec3::Y, 0.99), SLAB_BOTTOM);
    assert_eq!(state(IVec3::NEG_Y, 0.0), SLAB_TOP);
    assert_eq!(state(IVec3::X, 0.5), SLAB_BOTTOM);
    assert_eq!(state(IVec3::X, 0.51), SLAB_TOP);
}

#[test]
fn stairs_face_where_the_player_looks() {
    let state = |look, normal, hit_y| {
        Block::OakStairs
            .placement_state(&placement(normal, hit_y, look))
            .unwrap()
    };
    assert_eq!(state(IVec3::X, IVec3::Y, 0.0), FACING_POS_X);
    assert_eq!(state(IVec3::NEG_X, IVec3::Y, 0.0), FACING_NEG_X);
    assert_eq!(state(IVec3::Z, IVec3::Y, 0.0), FACING_POS_Z);
    assert_eq!(state(IVec3::NEG_Z, IVec3::Y, 0.0), FACING_NEG_Z);
    assert_eq!(
        state(IVec3::Z, IVec3::NEG_Y, 0.0),
        FACING_POS_Z | STAIRS_TOP
    );
    assert_eq!(state(IVec3::Z, IVec3::X, 0.8), FACING_POS_Z | STAIRS_TOP);
}

#[test]
fn colored_blocks_place_with_the_item_color() {
    for block in [Block::Wool, Block::Carpet] {
        let state = |variant| {
            block.placement_state(&Placement {
                variant,
                ..placement(IVec3::Y, 0.0, IVec3::X)
            })
        };
        assert_eq!(state(14), Some(14));
        assert_eq!(state(3), Some(3));
    }
}

#[test]
fn colors_are_separate_stacks_with_their_own_tiles() {
    let stacks: Vec<_> = Block::stacks()
        .iter()
        .filter(|stack| stack.block == Block::Wool)
        .collect();
    assert_eq!(stacks.len(), 16);
    assert_eq!(stacks[14].name, "red_wool");
    assert_eq!(stacks[14].data, 14);
    let red = Block::stack_from_name("red_carpet").unwrap();
    assert_eq!((red.block, red.data), (Block::Carpet, 14));
    assert_eq!(red.label, "красный ковёр");
    assert!(Block::stack_from_name("wool").is_none());
    assert_eq!(Block::Wool.tile_for(Facing::Side, 14), Some(Tile::WoolRed));
    let names: std::collections::HashSet<_> =
        Block::stacks().iter().map(|stack| &stack.name).collect();
    assert_eq!(names.len(), Block::stacks().len());
}

#[test]
fn carpet_stands_on_anything_but_air() {
    assert!(Block::Carpet.stands_on(Block::OakSlab));
    assert!(Block::Carpet.stands_on(Block::Stone));
    assert!(!Block::Carpet.stands_on(Block::Air));
}

#[test]
fn bookshelf_and_crafting_table_use_planks_and_different_faces() {
    assert_eq!(Block::Bookshelf.tile(Facing::Top), Some(Tile::PlanksOak));
    assert_eq!(Block::Bookshelf.tile(Facing::Bottom), Some(Tile::PlanksOak));
    assert_eq!(Block::Bookshelf.tile(Facing::Side), Some(Tile::Bookshelf));
    let table = |normal| Block::CraftingTable.face_tile(Facing::Side, normal, 0);
    assert_eq!(table(IVec3::NEG_Z), Some(Tile::CraftingTableFront));
    assert_eq!(table(IVec3::Z), Some(Tile::CraftingTableSide));
    assert_eq!(
        Block::CraftingTable.tile(Facing::Top),
        Some(Tile::CraftingTableTop)
    );
}

#[test]
fn stone_slab_uses_its_own_top_and_side_tiles() {
    assert_eq!(Block::StoneSlab.tile(Facing::Top), Some(Tile::StoneSlabTop));
    assert_eq!(
        Block::StoneSlab.tile(Facing::Side),
        Some(Tile::StoneSlabSide)
    );
}

fn neighbors(around: &[(IVec3, Block, u8)]) -> impl Fn(IVec3) -> (Block, u8) + '_ {
    move |offset| {
        around
            .iter()
            .find(|(at, ..)| *at == offset)
            .map_or((Block::Air, 0), |&(_, block, data)| (block, data))
    }
}

#[test]
fn a_lone_fence_is_a_post_and_joins_fences_gates_and_full_blocks() {
    let alone = Block::OakFence.boxes(0, neighbors(&[]));
    assert_eq!(alone.len(), 1);
    let joined = Block::OakFence.boxes(
        0,
        neighbors(&[
            (IVec3::X, Block::SpruceFence, 0),
            (IVec3::NEG_X, Block::Stone, 0),
            (IVec3::Z, Block::Leaves, 0),
            (IVec3::NEG_Z, Block::Glowstone, 0),
        ]),
    );
    assert_eq!(joined.len(), 1 + 3 * 2);
    let (min, max) = joined.bounds();
    assert_eq!((min.x, max.x), (0.0, 1.0));
    assert_eq!((min.z, max.z), (0.0, 10.0 / 16.0));
}

#[test]
fn a_gate_joins_a_fence_only_along_its_own_line() {
    let gate = |data| neighbors_for(Block::OakFenceGate, data);
    let along = Block::OakFence.boxes(0, gate(FACING_POS_Z));
    let across = Block::OakFence.boxes(0, gate(FACING_POS_X));
    assert_eq!(along.len(), 3);
    assert_eq!(across.len(), 1);
}

fn neighbors_for(block: Block, data: u8) -> impl Fn(IVec3) -> (Block, u8) {
    move |offset| {
        if offset == IVec3::X {
            (block, data)
        } else {
            (Block::Air, 0)
        }
    }
}

#[test]
fn a_fence_collides_to_one_and_a_half_but_is_selected_to_one() {
    let around = neighbors(&[(IVec3::X, Block::OakFence, 0)]);
    let collision = Block::OakFence.collision_boxes(0, &around);
    assert_eq!(collision.top(), 1.5);
    assert_eq!(collision.len(), 2);
    let outline = Block::OakFence.boxes(0, &around);
    assert_eq!(outline.top(), 1.0);
}

#[test]
fn an_open_gate_does_not_collide_and_a_closed_one_is_one_and_a_half_high() {
    use crate::block::GATE_OPEN;
    let closed = Block::OakFenceGate.collision_boxes(FACING_NEG_Z, neighbors(&[]));
    assert_eq!(closed.top(), 1.5);
    let open = Block::OakFenceGate.collision_boxes(FACING_NEG_Z | GATE_OPEN, neighbors(&[]));
    assert!(open.is_empty());
    assert!(
        !Block::OakFenceGate
            .boxes(FACING_NEG_Z | GATE_OPEN, neighbors(&[]))
            .is_empty()
    );
}

#[test]
fn a_lone_pane_is_a_cross_and_joins_panes_glass_and_full_blocks() {
    assert_eq!(Block::GlassPane.boxes(0, neighbors(&[])).len(), 1 + 4);
    let list = Block::GlassPane.boxes(
        0,
        neighbors(&[
            (IVec3::X, Block::IronBars, 0),
            (IVec3::NEG_X, Block::Glass, 0),
            (IVec3::Z, Block::Stone, 0),
        ]),
    );
    assert_eq!(list.len(), 1 + 3);
    let none = Block::IronBars.boxes(0, neighbors(&[(IVec3::X, Block::Leaves, 0)]));
    assert_eq!(none.len(), 5);
}

#[test]
fn a_door_slab_follows_facing_opening_and_hinge() {
    use crate::block::DOOR_OPEN;
    let slab = |data, upper: u8| {
        let list = Block::OakDoor.boxes(data, neighbors(&[(IVec3::Y, Block::OakDoorUpper, upper)]));
        assert_eq!(list.len(), 1);
        list.bounds()
    };
    let (min, max) = slab(FACING_POS_X, 0);
    assert_eq!((min.x, max.x, max.y), (0.0, 3.0 / 16.0, 1.0));
    let (min, max) = slab(FACING_POS_X | DOOR_OPEN, 1);
    assert_eq!((min.z, max.z), (13.0 / 16.0, 1.0));
    let (min, max) = slab(FACING_POS_X | DOOR_OPEN, 0);
    assert_eq!((min.z, max.z), (0.0, 3.0 / 16.0));
    let (min, _) = slab(FACING_NEG_X, 0);
    assert_eq!(min.x, 13.0 / 16.0);
}

#[test]
fn the_upper_door_half_reads_the_lower_one() {
    use crate::block::DOOR_OPEN;
    let around = neighbors(&[(IVec3::NEG_Y, Block::OakDoor, FACING_POS_Z | DOOR_OPEN)]);
    let list = Block::OakDoorUpper.boxes(0, around);
    let (min, max) = list.bounds();
    assert_eq!((min.x, max.x), (13.0 / 16.0, 1.0));
}

#[test]
fn a_ladder_hugs_the_wall_it_is_placed_on() {
    assert_eq!(Block::Ladder.placement_data(IVec3::Y), None);
    assert_eq!(Block::Ladder.placement_data(IVec3::NEG_Y), None);
    assert_eq!(Block::Ladder.placement_data(IVec3::X), Some(FACING_POS_X));
    assert_eq!(Block::Ladder.support_dir(FACING_POS_X), Some(IVec3::NEG_X));
    let (min, max) = Block::Ladder.bounds(FACING_POS_X);
    assert_eq!((min.x, max.x), (0.0, 2.0 / 16.0));
    assert!(
        Block::Ladder
            .collision_boxes(FACING_POS_X, neighbors(&[]))
            .len()
            == 1
    );
}

#[test]
fn the_new_blocks_are_in_the_creative_inventory_with_russian_labels() {
    for name in [
        "oak_fence",
        "dark_oak_fence_gate",
        "glass_pane",
        "iron_bars",
        "oak_door",
        "iron_door",
        "ladder",
        "web",
        "rail",
    ] {
        let stack = Block::stack_from_name(name).unwrap_or_else(|| panic!("{name}"));
        assert!(
            stack.label.chars().any(|c| ('а'..='я').contains(&c)),
            "{name}"
        );
    }
    assert!(Block::stack_from_name("oak_door_upper").is_none());
    assert!(Block::stack_from_name("iron_door_upper").is_none());
}

#[test]
fn rails_need_a_full_block_below_and_webs_need_nothing() {
    assert!(Block::Rail.stands_on(Block::Stone));
    assert!(!Block::Rail.stands_on(Block::OakFence));
    assert!(!Block::Rail.stands_on(Block::Air));
    assert!(!Block::Web.needs_support());
    assert_eq!(Block::Web.collision_height(), 0.0);
    assert_eq!(Block::Rail.collision_height(), 0.0);
}

#[test]
fn double_doors_pair_up_like_double_plants() {
    for (lower, upper) in crate::block::DOORS {
        assert_eq!(lower.double_upper(), Some(upper));
        assert_eq!(upper.double_lower(), Some(lower));
    }
}

#[test]
fn furnace_front_turns_to_the_player() {
    let data = Block::Furnace
        .placement_state(&placement(IVec3::Y, 0.0, IVec3::NEG_Z))
        .unwrap();
    assert_eq!(Block::Furnace.facing_dir(data), IVec3::Z);
    for (normal, front) in [(IVec3::Z, true), (IVec3::NEG_Z, false), (IVec3::X, false)] {
        let tile = Block::Furnace.face_tile(Facing::Side, normal, data);
        assert_eq!(tile == Some(Tile::FurnaceFrontOff), front, "{normal}");
    }
    assert_eq!(
        Block::Furnace.face_tile(Facing::Top, IVec3::Y, data),
        Some(Tile::FurnaceTop)
    );
}

#[test]
fn lit_furnace_glows_and_shows_the_burning_front() {
    assert_eq!(Block::LitFurnace.emission(), 13);
    assert_eq!(Block::Furnace.emission(), 0);
    let front = Block::LitFurnace.face_tile(Facing::Side, IVec3::Z, 2);
    assert_eq!(front, Some(Tile::FurnaceFrontOn));
    assert!(!Block::LitFurnace.def().selectable);
    assert!(Block::stack_from_name("lit_furnace").is_none());
    assert!(Block::stack_from_name("furnace").is_some());
}

#[test]
fn cauldron_water_level_gives_the_surface_height() {
    assert_eq!(Block::cauldron_water_units(0), None);
    assert_eq!(Block::cauldron_water_units(1), Some(9));
    assert_eq!(Block::cauldron_water_units(2), Some(12));
    assert_eq!(Block::cauldron_water_units(3), Some(15));
    assert_eq!(Block::cauldron_water_units(9), Some(15));
}

#[test]
fn cauldron_is_an_open_pot_with_legs() {
    let boxes = Block::Cauldron.boxes(0, |_| (Block::Air, 0));
    assert_eq!(boxes.len(), 9);
    let (min, max) = boxes.bounds();
    assert_eq!((min, max), (Vec3::ZERO, Vec3::ONE));
    let tiles: Vec<_> = (0..9)
        .map(|i| Block::Cauldron.box_tile(i, Facing::Top, IVec3::Y, 0))
        .collect();
    assert_eq!(tiles[0], Some(Tile::CauldronInner));
    assert_eq!(tiles[1], Some(Tile::CauldronTop));
    assert_eq!(
        Block::Cauldron.box_tile(1, Facing::Side, IVec3::X, 0),
        Some(Tile::CauldronInner)
    );
    assert_eq!(
        Block::Cauldron.box_tile(1, Facing::Side, IVec3::NEG_X, 0),
        Some(Tile::CauldronSide)
    );
}

#[test]
fn pot_remembers_one_plant_from_the_list() {
    assert_eq!(Block::pot_plant(0), None);
    for (i, plant) in POT_PLANTS.iter().enumerate() {
        assert_eq!(Block::pot_code(*plant), Some(i as u8 + 1));
        assert_eq!(Block::pot_plant(i as u8 + 1), Some(*plant));
    }
    assert_eq!(Block::pot_code(Block::Stone), None);
    assert_eq!(POT_PLANTS.len(), 15);
}

#[test]
fn monster_eggs_look_like_their_variant_and_each_has_an_item() {
    for (data, &(name, _, tile)) in EGG_VARIANTS.iter().enumerate() {
        let data = data as u8;
        assert_eq!(Block::MonsterEgg.tile_for(Facing::Side, data), Some(tile));
        let stack = Block::stack_from_name(name).unwrap();
        assert_eq!((stack.block, stack.data), (Block::MonsterEgg, data));
    }
    assert_eq!(
        Block::MonsterEgg.tile_for(Facing::Top, 0),
        Some(Tile::Stone)
    );
    assert_eq!(
        Block::MonsterEgg.placement_state(&Placement {
            normal: IVec3::Y,
            hit_x: 0.5,
            hit_y: 1.0,
            hit_z: 0.5,
            look: IVec3::X,
            variant: 4,
        }),
        Some(4)
    );
}

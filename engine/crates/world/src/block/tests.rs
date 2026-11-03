use super::*;
use crate::tile::DEFAULT_TILE_SIZE;

fn tile_pixels(tile: Tile) -> Vec<u8> {
    TextureSet::standard().tile_pixels(tile)
}

fn icon_pixels(block: Block) -> Option<Vec<u8>> {
    block.icon_pixels(&TextureSet::standard())
}

const ALL: [Block; 50] = Block::ALL;

#[test]
fn byte_round_trip() {
    for (i, block) in ALL.into_iter().enumerate() {
        assert_eq!(block as usize, i, "save numbers must not move");
        assert_eq!(Block::try_from(block as u8), Ok(block));
    }
    assert_eq!(Block::try_from(50), Err(50));
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
        let plant = matches!(
            block,
            Block::TallGrass
                | Block::Poppy
                | Block::Dandelion
                | Block::Torch
                | Block::DeadBush
                | Block::Reeds
                | Block::Vine
        );
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
        ) && !plant;
        assert_eq!(
            block.is_targetable(),
            solid || plant || block == Block::Snow,
            "{block:?}"
        );
        assert_eq!(
            block.needs_support(),
            (plant && block != Block::Vine) || matches!(block, Block::Snow | Block::Cactus),
            "{block:?}"
        );
        assert_eq!(block.is_opaque(), opaque, "{block:?}");
        assert_eq!(block.is_water(), block == Block::Water, "{block:?}");
    }
}

#[test]
fn render_layers() {
    for block in ALL {
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
            | Block::Reeds => RenderLayer::Cutout,
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
            !(def.needs_support && def.solid) || def.shape == Shape::Cactus,
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
        let expected = match block {
            Block::Log
            | Block::BirchLog
            | Block::SpruceLog
            | Block::JungleLog
            | Block::AcaciaLog
            | Block::DarkOakLog => DataKind::Axis,
            Block::Vine => DataKind::Sides,
            Block::Torch => DataKind::Attachment,
            Block::Water | Block::Lava => DataKind::Fluid,
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
        assert_eq!(block.is_slippery(), block == Block::Ice, "{block:?}");
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
        assert_eq!(
            block.shape() == Shape::Cross,
            matches!(
                block,
                Block::TallGrass | Block::Poppy | Block::Dandelion | Block::DeadBush | Block::Reeds
            ),
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

use super::*;

const ALL: [Block; 13] = Block::ALL;

#[test]
fn byte_round_trip() {
    for (i, block) in ALL.into_iter().enumerate() {
        assert_eq!(block as usize, i, "save numbers must not move");
        assert_eq!(Block::try_from(block as u8), Ok(block));
    }
    assert_eq!(Block::try_from(13), Err(13));
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
    assert_eq!(Block::from_name("water"), None);
    assert_eq!(Block::from_name("Stone"), None);
}

#[test]
fn icon_is_side_face() {
    let pixels = |tile| Some(tile_pixels(tile));
    assert_eq!(Block::Grass.icon_pixels(), pixels(Tile::GrassSide));
    assert_eq!(Block::Log.icon_pixels(), pixels(Tile::LogSide));
    assert_eq!(Block::Stone.icon_pixels(), pixels(Tile::Stone));
    assert_eq!(Block::Air.icon_pixels(), None);
    // Зелёная кайма сверху и земля ниже: иконка травы не сплошная.
    let grass = Block::Grass.icon_pixels().unwrap();
    let (top, bottom) = (&grass[..3], &grass[grass.len() - 4..][..3]);
    assert_ne!(top, bottom);
}

#[test]
fn solid_and_opaque() {
    for block in ALL {
        let plant = matches!(
            block,
            Block::TallGrass | Block::Poppy | Block::Dandelion | Block::Torch
        );
        let solid = !matches!(block, Block::Air | Block::Water) && !plant;
        assert_eq!(block.is_solid(), solid, "{block:?}");
        let opaque = !matches!(block, Block::Air | Block::Leaves | Block::Water) && !plant;
        assert_eq!(block.is_targetable(), solid || plant, "{block:?}");
        assert_eq!(block.needs_support(), plant, "{block:?}");
        assert_eq!(block.is_opaque(), opaque, "{block:?}");
        assert_eq!(block.is_water(), block == Block::Water, "{block:?}");
    }
}

#[test]
fn render_layers() {
    for block in ALL {
        let layer = match block {
            Block::Leaves | Block::TallGrass | Block::Poppy | Block::Dandelion | Block::Torch => {
                RenderLayer::Cutout
            }
            Block::Water => RenderLayer::Translucent,
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
    assert_eq!(Block::Stone.bounds(), (Vec3::ZERO, Vec3::ONE));
    assert_eq!(Block::Poppy.bounds(), (Vec3::ZERO, Vec3::ONE));
    let (lo, hi) = Block::Torch.bounds();
    assert_eq!((lo.x, hi.x, hi.y), (7.0 / 16.0, 9.0 / 16.0, 10.0 / 16.0));
}

#[test]
fn replaceable_cells() {
    for block in ALL {
        let expected = matches!(block, Block::Air | Block::Water | Block::TallGrass);
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
        assert!(block.icon_pixels().is_some());
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
        let def = block.def();
        assert!(def.top.is_some() && def.side.is_some() && def.bottom.is_some());
    }
}

#[test]
fn support_is_for_non_solid_blocks() {
    for def in &BLOCKS {
        assert!(!(def.needs_support && def.solid), "{}", def.name);
        // Непрозрачный блок обязан быть твёрдым кубом.
        assert!(
            !def.opaque || def.solid && def.shape == Shape::Cube,
            "{}",
            def.name
        );
    }
}

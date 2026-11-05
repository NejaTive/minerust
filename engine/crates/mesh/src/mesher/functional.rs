use super::*;
use crate::templates::POT_CROSS_PARTS;

const AT: IVec3 = IVec3::new(5, 5, 5);

fn air(_: IVec3) -> Block {
    Block::Air
}

fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

fn mesh(block: Block, data: u8) -> SectionMeshes {
    let mut section = Section::new();
    section.set_with_data(AT, block, data);
    mesh_section(&section, air, full, MeshOptions::default())
}

fn top_planes(meshes: &SectionMeshes, tile: Tile) -> Vec<f32> {
    meshes
        .cutout
        .quads
        .iter()
        .filter(|q| q.layer() == tile.layer() as u16 && q.template() == 0)
        .map(|q| f32::from(q.box_rect().1))
        .collect()
}

#[test]
fn cauldron_water_surface_follows_the_level() {
    assert!(top_planes(&mesh(Block::Cauldron, 0), Tile::Water).is_empty());
    for (level, plane) in [(1, 9.0), (2, 12.0), (3, 15.0)] {
        assert_eq!(
            top_planes(&mesh(Block::Cauldron, level), Tile::Water),
            [plane]
        );
    }
}

#[test]
fn cauldron_inside_uses_the_inner_texture() {
    let meshes = mesh(Block::Cauldron, 0);
    let inner = Tile::CauldronInner.layer() as u16;
    assert!(meshes.cutout.quads.iter().any(|q| q.layer() == inner));
}

#[test]
fn pot_plant_is_two_crossed_planes_in_the_plant_texture() {
    assert!(
        mesh(Block::FlowerPot, 0)
            .cutout
            .quads
            .iter()
            .all(|q| q.template() < crate::templates::POT_CROSS)
    );
    let meshes = mesh(Block::FlowerPot, Block::pot_code(Block::Poppy).unwrap());
    let planes: Vec<_> = meshes
        .cutout
        .quads
        .iter()
        .filter(|q| q.template() >= crate::templates::POT_CROSS)
        .collect();
    assert_eq!(planes.len(), usize::from(POT_CROSS_PARTS));
    assert!(
        planes
            .iter()
            .all(|q| q.layer() == Tile::Poppy.layer() as u16)
    );
    let cactus = mesh(Block::FlowerPot, Block::pot_code(Block::Cactus).unwrap());
    assert!(
        cactus
            .cutout
            .quads
            .iter()
            .any(|q| q.layer() == Tile::CactusSide.layer() as u16)
    );
}

#[test]
fn chest_faces_show_their_entity_tiles() {
    let meshes = mesh(Block::Chest, 0);
    let layers: std::collections::HashSet<u16> =
        meshes.cutout.quads.iter().map(|q| q.layer()).collect();
    for tile in [
        Tile::ChestFront,
        Tile::ChestBack,
        Tile::ChestTop,
        Tile::ChestBottom,
    ] {
        assert!(layers.contains(&(tile.layer() as u16)), "{tile:?}");
    }
}

#[test]
fn double_chest_has_no_end_faces_between_the_halves() {
    let facing = world::chest::data(IVec3::NEG_Z, world::chest::HALF_RIGHT);
    let other = world::chest::data(IVec3::NEG_Z, world::chest::HALF_LEFT);
    let mut section = Section::new();
    section.set_with_data(AT, Block::Chest, facing);
    section.set_with_data(AT + IVec3::X, Block::Chest, other);
    let meshes = mesh_section(&section, air, full, MeshOptions::default());
    let ends = meshes
        .cutout
        .quads
        .iter()
        .filter(|q| {
            [Tile::ChestDoubleEndL, Tile::ChestDoubleEndR]
                .iter()
                .any(|t| q.layer() == t.layer() as u16)
        })
        .count();
    assert_eq!(ends, 6);
}

#[test]
fn flower_pot_is_a_six_pixel_ring_around_recessed_dirt() {
    let meshes = mesh(Block::FlowerPot, 0);
    let dirt_tops = top_planes(&meshes, Tile::Dirt);
    assert_eq!(dirt_tops, vec![4.0]);
    let list = Block::FlowerPot.boxes(0, |_| (Block::Air, 0));
    let (lo, hi) = list.iter().fold((16, 0), |(lo, hi), b| {
        (
            lo.min(b.min[0]).min(b.min[2]),
            hi.max(b.max[0]).max(b.max[2]),
        )
    });
    assert_eq!((lo, hi), (5, 11));
    assert!(list.iter().all(|b| b.max[1] <= 6));
}

fn quads_of(meshes: &SectionMeshes) -> Vec<crate::mesh::Quad> {
    meshes
        .solid
        .quads
        .iter()
        .chain(&meshes.cutout.quads)
        .copied()
        .collect()
}

#[test]
fn bed_top_is_turned_with_the_head_direction_and_its_sides_are_not() {
    for turns in 0..4u8 {
        let data = world::bed::data(world::block::facing_dir_of(turns), true, 14);
        let top = Tile::bed(14, true, true).layer() as u16;
        let side = Tile::bed(14, true, false).layer() as u16;
        let quads = quads_of(&mesh(Block::Bed, data));
        let tops: Vec<_> = quads.iter().filter(|q| q.layer() == top).collect();
        assert_eq!(tops.len(), 1);
        assert_eq!(tops[0].turn(), turns);
        let sides: Vec<_> = quads.iter().filter(|q| q.layer() == side).collect();
        assert!(!sides.is_empty());
        assert!(sides.iter().all(|q| q.turn() == 0));
    }
}

#[test]
fn bed_keeps_its_height_of_nine_sixteenths() {
    let quads = quads_of(&mesh(Block::Bed, world::bed::data(IVec3::Z, false, 0)));
    let top = Tile::bed(0, false, true).layer() as u16;
    let plane = quads
        .iter()
        .find(|q| q.layer() == top)
        .unwrap()
        .box_rect()
        .1;
    assert_eq!(plane, 9);
}

#[test]
fn redstone_torch_uses_its_own_tile_on_floor_and_wall() {
    for data in [0, world::block::ATTACH_POS_X] {
        let meshes = mesh(Block::RedstoneTorch, data);
        let quads = quads_of(&meshes);
        assert!(!quads.is_empty());
        assert!(
            quads
                .iter()
                .all(|q| q.layer() == Tile::RedstoneTorchOn.layer() as u16)
        );
    }
}

#[test]
fn brewing_stand_draws_a_rod_over_a_base() {
    let layers: std::collections::HashSet<u16> = quads_of(&mesh(Block::BrewingStand, 0))
        .iter()
        .map(|q| q.layer())
        .collect();
    assert!(layers.contains(&(Tile::BrewingStand.layer() as u16)));
    assert!(layers.contains(&(Tile::BrewingStandBase.layer() as u16)));
}

#[test]
fn a_pressure_plate_is_a_stone_box_inset_by_a_pixel() {
    let quads = quads_of(&mesh(Block::StonePressurePlate, 0));
    assert_eq!(quads.len(), 6);
    assert!(
        quads
            .iter()
            .all(|q| q.layer() == Tile::Stone.layer() as u16)
    );
    let (low, high) = Block::StonePressurePlate
        .boxes(0, |_| (Block::Air, 0))
        .bounds();
    assert_eq!(
        (low.x, high.x, high.y),
        (1.0 / 16.0, 15.0 / 16.0, 1.0 / 16.0)
    );
}

#[test]
fn a_trapdoor_is_a_slab_that_stands_up_when_open() {
    use world::block::{TRAPDOOR_OPEN, TRAPDOOR_TOP};
    let flat = Block::OakTrapdoor.boxes(0, |_| (Block::Air, 0));
    assert_eq!(flat.bounds().1.y, 3.0 / 16.0);
    let top = Block::OakTrapdoor.boxes(TRAPDOOR_TOP, |_| (Block::Air, 0));
    assert_eq!(top.bounds().0.y, 13.0 / 16.0);
    let open = Block::OakTrapdoor.boxes(TRAPDOOR_OPEN, |_| (Block::Air, 0));
    assert_eq!(open.bounds().1.y, 1.0);
    assert_eq!(open.bounds().0.z, 13.0 / 16.0);
    assert!(!quads_of(&mesh(Block::OakTrapdoor, TRAPDOOR_OPEN)).is_empty());
}

fn stone_faces_toward(neighbor: Block, data: u8, offset: IVec3) -> bool {
    let mut section = Section::new();
    section.set(AT, Block::Stone);
    section.set_with_data(AT + offset, neighbor, data);
    let meshes = mesh_section(&section, air, full, MeshOptions::default());
    let face = crate::templates::FACES
        .iter()
        .position(|f| f.normal == offset)
        .unwrap() as u8;
    meshes
        .solid
        .quads
        .iter()
        .any(|q| q.layer() == Tile::Stone.layer() as u16 && q.template() == face)
}

#[test]
fn a_wall_face_shows_behind_a_ladder_or_an_open_trapdoor_but_not_under_a_carpet() {
    use world::block::{FACING_POS_X, TRAPDOOR_OPEN};
    assert!(stone_faces_toward(Block::Ladder, FACING_POS_X, IVec3::X));
    assert!(stone_faces_toward(
        Block::OakTrapdoor,
        TRAPDOOR_OPEN | FACING_POS_X,
        IVec3::X
    ));
    assert!(stone_faces_toward(Block::IronBars, 0, IVec3::X));
    assert!(!stone_faces_toward(Block::Carpet, 0, IVec3::Y));
}

#[test]
fn lever_draws_a_cobblestone_base_and_a_leaning_handle() {
    use world::block::{FACING_POS_X, LEVER_POWERED};
    let meshes = mesh(Block::Lever, FACING_POS_X);
    let layers = |tile: Tile| {
        meshes
            .cutout
            .quads
            .iter()
            .filter(|q| q.layer() == tile.layer() as u16)
            .collect::<Vec<_>>()
    };
    assert!(!layers(Tile::Cobblestone).is_empty());
    let handle = layers(Tile::Lever);
    assert_eq!(handle.len(), 6);
    assert!(handle.iter().all(|q| q.is_extended() && !q.is_boxed()));
    let powered = mesh(Block::Lever, FACING_POS_X | LEVER_POWERED);
    let first = |m: &SectionMeshes| {
        m.cutout
            .quads
            .iter()
            .find(|q| q.layer() == Tile::Lever.layer() as u16)
            .unwrap()
            .template_index()
    };
    assert_ne!(first(&meshes), first(&powered));
}

#[test]
fn hook_draws_a_plank_plate_an_arm_and_a_ring_and_wire_two_faces() {
    let hook = mesh(Block::TripwireHook, world::block::FACING_POS_X);
    let count = |tile: Tile| {
        hook.cutout
            .quads
            .iter()
            .filter(|q| q.layer() == tile.layer() as u16)
            .count()
    };
    assert_eq!(count(Tile::TripwireHook), 2);
    assert_eq!(count(Tile::PlanksOak), 6 + 6);

    let wire = mesh(Block::Tripwire, 0);
    assert_eq!(wire.cutout.quads.len(), 2);
    assert!(
        wire.cutout
            .quads
            .iter()
            .all(|q| q.layer() == Tile::Tripwire.layer() as u16)
    );
}

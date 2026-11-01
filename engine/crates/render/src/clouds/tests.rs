use glam::{IVec2, Vec2, Vec3};
use world::clouds::{CLOUD_CELL, CLOUD_HEIGHT, CLOUD_Y, CloudMask};

use super::*;

const CLOUD_SHADOW_PLANE_Y: f32 = CLOUD_Y as f32 + CLOUD_HEIGHT as f32 * 0.5;

fn cloud_shadow_cell(world_pos: Vec3, light_dir: Vec3, drift: f32) -> Vec2 {
    let t = (CLOUD_SHADOW_PLANE_Y - world_pos.y) / light_dir.y;
    let xz = Vec2::new(world_pos.x + light_dir.x * t, world_pos.z + light_dir.z * t);
    (xz - Vec2::new(drift, 0.0)) / CLOUD_CELL as f32
}

fn cloud_shadow_cover(mask: &CloudMask, cell: Vec2) -> f32 {
    let q = cell - Vec2::splat(0.5);
    let base = q.floor();
    let f = q - base;
    let f = f * f * (Vec2::splat(3.0) - 2.0 * f);
    let origin = base.as_ivec2();
    let at = |offset: IVec2| {
        let c = origin + offset;
        f32::from(u8::from(mask.get(c.x, c.y)))
    };
    let top = at(IVec2::ZERO) + (at(IVec2::X) - at(IVec2::ZERO)) * f.x;
    let bottom = at(IVec2::Y) + (at(IVec2::ONE) - at(IVec2::Y)) * f.x;
    top + (bottom - top) * f.y
}

fn instance_cell(index: u32, camera_xz: Vec2, drift: f32) -> IVec2 {
    let base = ((camera_xz - Vec2::new(drift, 0.0)) / CLOUD_CELL as f32)
        .floor()
        .as_ivec2();
    base + IVec2::new((index % CLOUD_GRID) as i32, (index / CLOUD_GRID) as i32)
        - IVec2::splat(CLOUD_RADIUS_CELLS as i32)
}

fn cell_origin(cell: IVec2, drift: f32) -> Vec2 {
    cell.as_vec2() * CLOUD_CELL as f32 + Vec2::new(drift, 0.0)
}

const NEIGHBORS: [IVec2; 6] = [
    IVec2::ZERO,
    IVec2::ZERO,
    IVec2::new(1, 0),
    IVec2::new(-1, 0),
    IVec2::new(0, 1),
    IVec2::new(0, -1),
];

fn face_visible(mask: &CloudMask, cell: IVec2, face: usize) -> bool {
    mask.get(cell.x, cell.y)
        && (face < 2 || !mask.get(cell.x + NEIGHBORS[face].x, cell.y + NEIGHBORS[face].y))
}

#[test]
fn instance_count_matches_grid() {
    assert_eq!(CLOUD_INSTANCES, 64 * 64);
}

#[test]
fn grid_covers_camera_and_stays_within_radius() {
    let camera = Vec2::new(-1234.5, 777.7);
    for drift in [0.0, 5.3, 1000.0] {
        let origins: Vec<Vec2> = (0..CLOUD_INSTANCES)
            .map(|i| cell_origin(instance_cell(i, camera, drift), drift))
            .collect();
        let size = CLOUD_CELL as f32;
        assert!(origins.iter().any(
            |o| (o.x..o.x + size).contains(&camera.x) && (o.y..o.y + size).contains(&camera.y)
        ));
        let reach = (CLOUD_RADIUS_CELLS as f32 + 1.0) * size;
        assert!(origins.iter().all(|o| (o.x - camera.x).abs() <= reach));
        assert!(origins.iter().all(|o| (o.y - camera.y).abs() <= reach));
    }
}

#[test]
fn drift_moves_clouds_along_x_without_moving_grid() {
    let camera = Vec2::new(10.0, 20.0);
    let cell = instance_cell(100, camera, 0.0);
    let moved = instance_cell(100, camera, 12.0);
    assert_eq!(moved, cell - IVec2::X);
    assert_eq!(cell_origin(moved, 12.0), cell_origin(cell, 0.0));
    assert_eq!(
        cell_origin(instance_cell(100, camera, 3.0), 3.0).y,
        cell_origin(cell, 0.0).y
    );
}

#[test]
fn interior_faces_between_clouds_are_dropped() {
    let mask = CloudMask::new(9);
    let mut inner = 0;
    let mut outer = 0;
    for x in -20..20 {
        for z in -20..20 {
            let cell = IVec2::new(x, z);
            for (face, offset) in NEIGHBORS.iter().enumerate().skip(2) {
                let neighbor = cell + *offset;
                if mask.get(x, z) && mask.get(neighbor.x, neighbor.y) {
                    assert!(!face_visible(&mask, cell, face));
                    inner += 1;
                } else if mask.get(x, z) {
                    assert!(face_visible(&mask, cell, face));
                    outer += 1;
                }
            }
            assert_eq!(face_visible(&mask, cell, 0), mask.get(x, z));
            assert_eq!(face_visible(&mask, cell, 1), mask.get(x, z));
        }
    }
    assert!(inner > 0 && outer > 0);
}

#[test]
fn color_follows_day_phase() {
    assert_eq!(cloud_color(0.0, 0.0, 0.0, 0.0), Vec3::ONE);
    assert_eq!(cloud_color(1.0, 0.0, 0.0, 0.0), NIGHT_CLOUD);
    let dusk = cloud_color(0.0, 1.0, 0.0, 0.0);
    assert!(dusk.x > dusk.y && dusk.y > dusk.z);
    assert!(cloud_color(1.0, 0.0, 0.0, 0.0).max_element() < 0.2);
}

#[test]
fn rain_darkens_clouds_and_flash_lights_them() {
    let rainy = cloud_color(0.0, 0.0, 1.0, 0.0);
    assert!(rainy.max_element() < 0.5);
    assert!(cloud_color(0.0, 0.0, 1.0, 1.0).min_element() > rainy.max_element());
}

#[test]
fn shadow_cell_projects_along_light_to_cloud_plane_with_drift() {
    let light = Vec3::new(0.6, 0.8, 0.0);
    let point = Vec3::new(100.0, 100.0, 50.0);
    let rise = (CLOUD_SHADOW_PLANE_Y - point.y) / light.y;
    let hit_x = point.x + light.x * rise;
    let cell = cloud_shadow_cell(point, light, 0.0);
    assert!((cell.x - hit_x / CLOUD_CELL as f32).abs() < 1e-4);
    assert!((cell.y - point.z / CLOUD_CELL as f32).abs() < 1e-4);
    let drifted = cloud_shadow_cell(point, light, 24.0);
    assert!((drifted.x - (cell.x - 2.0)).abs() < 1e-4);
    assert_eq!(drifted.y, cell.y);
}

#[test]
fn shadow_cell_of_point_under_the_plane_with_vertical_light_is_its_own_column() {
    let cell = cloud_shadow_cell(Vec3::new(30.0, 70.0, -18.0), Vec3::Y, 0.0);
    assert_eq!(cell, Vec2::new(2.5, -1.5));
}

#[test]
fn shadow_cover_is_solid_inside_empty_outside_and_soft_on_the_edge() {
    let mask = CloudMask::new(9);
    let mut checked = 0;
    for x in -30..30 {
        for z in -30..30 {
            let near = |dx: i32, dz: i32| mask.get(x + dx, z + dz);
            let cells = [(0, 0), (1, 0), (0, 1), (1, 1)];
            let all = cells.iter().all(|&(dx, dz)| near(dx, dz));
            let none = cells.iter().all(|&(dx, dz)| !near(dx, dz));
            let mixed = !all && !none;
            let center = Vec2::new(x as f32 + 1.0, z as f32 + 1.0);
            let cover = cloud_shadow_cover(&mask, center);
            if all {
                assert_eq!(cover, 1.0);
            } else if none {
                assert_eq!(cover, 0.0);
            } else if mixed {
                assert!(cover > 0.0 && cover < 1.0, "{cover}");
                checked += 1;
            }
        }
    }
    assert!(checked > 0);
}

#[test]
fn cloud_shadow_strength_fades_with_rain_and_light() {
    assert!(cloud_shadow_strength(1.0, 0.0) > 0.3);
    assert!(cloud_shadow_strength(1.0, 0.0) < 0.5);
    assert_eq!(cloud_shadow_strength(1.0, 0.5), 0.0);
    assert!(cloud_shadow_strength(0.13, 0.0) < 0.07);
}

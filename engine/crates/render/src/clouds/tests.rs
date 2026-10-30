use glam::{IVec2, Vec2};
use world::clouds::{CLOUD_CELL, CloudMask};

use super::*;

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
    assert_eq!(cloud_color(0.0, 0.0), Vec3::ONE);
    assert_eq!(cloud_color(1.0, 0.0), NIGHT_CLOUD);
    let dusk = cloud_color(0.0, 1.0);
    assert!(dusk.x > dusk.y && dusk.y > dusk.z);
    assert!(cloud_color(1.0, 0.0).max_element() < 0.2);
}

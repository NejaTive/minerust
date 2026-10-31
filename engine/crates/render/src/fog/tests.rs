use glam::{Vec2, Vec3};
use world::pos::ColumnPos;

use super::*;

fn distance_to_column(point: Vec2, key: ColumnPos) -> f32 {
    let size = SECTION_SIZE as f32;
    let min = key.0.as_vec2() * size;
    point.clamp(min, min + size).distance(point)
}

#[test]
fn mesh_edge_is_fully_fogged_from_any_point_of_camera_column() {
    let size = SECTION_SIZE as f32;
    for radius in 4..=16 {
        let fog = Fog::for_radius(radius);
        let reach = radius + 2;
        let mut nearest = f32::INFINITY;
        for corner in [Vec2::ZERO, Vec2::X, Vec2::Y, Vec2::ONE] {
            for z in -reach..=reach {
                for x in -reach..=reach {
                    let key = ColumnPos::new(x, z);
                    if key.dist2(ColumnPos::ZERO) > radius * radius {
                        nearest = nearest.min(distance_to_column(corner * size, key));
                    }
                }
            }
        }
        assert!(
            fog.end <= nearest,
            "radius {radius}: {} > {nearest}",
            fog.end
        );
        assert!(
            nearest - fog.end < size,
            "radius {radius}: {} vs {nearest}",
            fog.end
        );
        assert!(fog.start < fog.end);
    }
}

#[test]
fn far_plane_covers_everything_before_fog_end() {
    let fog = Fog::for_radius(16);
    let height = WORLD_HEIGHT as f32;
    for camera_y in [-20.0, 0.0, 40.0, 64.0, 127.0, 300.0] {
        let far = fog.far_plane(camera_y);
        let camera = Vec3::new(0.0, camera_y, 0.0);
        for y in [0.0, height] {
            let edge = Vec3::new(fog.end, y, 0.0);
            assert!(
                camera.distance(edge) < far,
                "camera y {camera_y}, point y {y}"
            );
        }
    }
}

#[test]
fn rain_pulls_fog_closer() {
    let fog = Fog::for_radius(8);
    assert_eq!(fog.in_rain(0.0), fog);
    let rainy = fog.in_rain(1.0);
    assert!(rainy.end < fog.end * 0.6 && rainy.start < fog.start * 0.5);
    assert!(rainy.start < rainy.end);
}

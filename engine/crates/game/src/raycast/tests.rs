use std::collections::HashSet;

use super::*;

fn blocks(list: &[[i32; 3]]) -> impl Fn(IVec3) -> bool {
    let set: HashSet<IVec3> = list.iter().map(|&p| IVec3::from(p)).collect();
    move |p| set.contains(&p)
}

#[test]
fn axis_rays_hit_each_face() {
    let solid = blocks(&[[0, 0, 0]]);
    for normal in [
        IVec3::X,
        IVec3::NEG_X,
        IVec3::Y,
        IVec3::NEG_Y,
        IVec3::Z,
        IVec3::NEG_Z,
    ] {
        // Из центра блока на 3 блока в сторону нормали, смотрим обратно.
        let origin = Vec3::splat(0.5) + normal.as_vec3() * 3.0;
        let hit = raycast(origin, -normal.as_vec3(), 6.0, &solid).unwrap();
        assert_eq!(hit.block, IVec3::ZERO);
        assert_eq!(hit.normal, normal);
        assert!((hit.distance - 2.5).abs() < 1e-5, "{}", hit.distance);
    }
}

#[test]
fn diagonal_through_edge_does_not_slip_between_blocks() {
    // Два блока касаются ребром, луч идёт точно через это ребро.
    let solid = blocks(&[[1, 0, 0], [0, 1, 0]]);
    let hit = raycast(
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(1.0, 1.0, 0.0),
        6.0,
        solid,
    )
    .unwrap();
    assert!([IVec3::new(1, 0, 0), IVec3::new(0, 1, 0)].contains(&hit.block));
    assert!((hit.distance - 0.5 * 2f32.sqrt()).abs() < 1e-5);
}

#[test]
fn diagonal_through_corner_does_not_slip_between_blocks() {
    // Три блока вокруг угла (1, 1, 1) по соседству с исходным; луч точно в угол.
    let solid = blocks(&[[1, 0, 0], [0, 1, 0], [0, 0, 1]]);
    let hit = raycast(Vec3::splat(0.5), Vec3::ONE, 6.0, solid);
    assert!(hit.is_some());

    // Блок только по диагонали через угол: путь идёт через соседей, а не мимо.
    let far = blocks(&[[1, 1, 1]]);
    let hit = raycast(Vec3::splat(0.5), Vec3::ONE, 6.0, far).unwrap();
    assert_eq!(hit.block, IVec3::ONE);
}

#[test]
fn diagonal_visits_every_crossed_voxel() {
    // Пологий луч через стену: сколько бы шагов ни было, стена не пропускается.
    let solid = |p: IVec3| p.x == 5;
    let hit = raycast(
        Vec3::new(0.1, 0.9, 0.3),
        Vec3::new(1.0, 0.37, -0.21),
        20.0,
        solid,
    )
    .unwrap();
    assert_eq!(hit.block.x, 5);
    assert_eq!(hit.normal, IVec3::NEG_X);
}

#[test]
fn negative_coordinates() {
    let solid = blocks(&[[-3, 10, -7]]);
    // По диагонали точно через ребро: при ничьей шаг сначала по x, в воздух
    // (-3, 10, -6), потом по z — в блок через его грань +Z.
    let origin = Vec3::new(-1.5, 10.5, -5.5);
    let hit = raycast(origin, Vec3::new(-1.0, 0.0, -1.0), 6.0, &solid).unwrap();
    assert_eq!(hit.block, IVec3::new(-3, 10, -7));
    assert_eq!(hit.normal, IVec3::Z);

    let hit = raycast(Vec3::new(-0.5, 10.5, -6.5), Vec3::NEG_X, 6.0, &solid).unwrap();
    assert_eq!(hit.block, IVec3::new(-3, 10, -7));
    assert_eq!(hit.normal, IVec3::X);
    assert!((hit.distance - 1.5).abs() < 1e-5);
}

#[test]
fn beyond_max_distance_is_none() {
    let solid = blocks(&[[10, 0, 0]]);
    assert_eq!(raycast(Vec3::splat(0.5), Vec3::X, 6.0, &solid), None);
    assert!(raycast(Vec3::splat(0.5), Vec3::X, 9.5, &solid).is_some());
}

#[test]
fn origin_block_is_skipped() {
    let solid = blocks(&[[0, 0, 0], [2, 0, 0]]);
    let hit = raycast(Vec3::splat(0.5), Vec3::X, 6.0, solid).unwrap();
    assert_eq!(hit.block, IVec3::new(2, 0, 0));
}

#[test]
fn unnormalized_dir_gives_same_hit() {
    let solid = |p: IVec3| p.y < 0;
    let origin = Vec3::new(3.2, 2.7, -1.4);
    let dir = Vec3::new(0.4, -1.0, 0.25);
    let a = raycast(origin, dir, 6.0, solid);
    let b = raycast(origin, dir * 17.0, 6.0, solid);
    assert!(a.is_some());
    assert_eq!(a, b);
}

#[test]
fn degenerate_input_is_none() {
    let solid = |_: IVec3| true;
    assert_eq!(raycast(Vec3::ZERO, Vec3::ZERO, 6.0, solid), None);
    assert_eq!(raycast(Vec3::NAN, Vec3::X, 6.0, solid), None);
    assert_eq!(raycast(Vec3::ZERO, Vec3::X, f32::NAN, solid), None);
}

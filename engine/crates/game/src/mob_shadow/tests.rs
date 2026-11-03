use super::*;

const FLOOR: i32 = 63;

fn flat(pos: IVec3) -> Ground {
    if pos.y <= FLOOR {
        Ground::Solid(1.0)
    } else {
        Ground::Air
    }
}

fn pig_feet() -> Vec3 {
    Vec3::new(0.5, 64.0, 0.5)
}

fn tops(patches: &[ShadowPatch]) -> Vec<f32> {
    patches.iter().map(|p| p.y).collect()
}

#[test]
fn flat_floor_gets_a_patch_lifted_over_every_covered_block() {
    let patches = shadow_patches(pig_feet(), 0.7, flat);
    assert_eq!(patches.len(), 9);
    assert!(
        tops(&patches)
            .iter()
            .all(|y| (y - 64.0 - LIFT).abs() < 1e-4)
    );
    let area: f32 = patches
        .iter()
        .map(|p| (p.max - p.min).element_product())
        .sum();
    assert!((area - 1.4 * 1.4).abs() < 1e-4);
    assert!(patches.iter().all(|p| p.opacity == MAX_OPACITY));
}

#[test]
fn a_step_under_the_mob_gets_its_own_height() {
    let stepped = |pos: IVec3| {
        if pos.x >= 1 && pos.y == 64 && pos.z == 0 {
            Ground::Solid(1.0)
        } else {
            flat(pos)
        }
    };
    let feet = Vec3::new(1.0, 65.0, 0.5);
    let patches = shadow_patches(feet, 0.7, stepped);
    let mut heights = tops(&patches);
    heights.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
    assert!(heights.iter().any(|y| (y - 65.0 - LIFT).abs() < 1e-4));
    assert!(patches.iter().any(|p| p.y < 65.0));
}

#[test]
fn no_patch_over_a_deep_drop() {
    let cliff = |pos: IVec3| {
        if pos.y <= FLOOR - 2 {
            Ground::Solid(1.0)
        } else {
            Ground::Air
        }
    };
    assert!(shadow_patches(pig_feet(), 0.7, cliff).is_empty());
}

#[test]
fn no_patch_on_water_or_plants() {
    assert!(shadow_patches(pig_feet(), 0.7, |_| Ground::Blocked).is_empty());
    let flower_floor = |pos: IVec3| {
        if pos.y == FLOOR {
            Ground::Blocked
        } else {
            flat(pos)
        }
    };
    assert!(shadow_patches(pig_feet(), 0.7, flower_floor).is_empty());
}

#[test]
fn opacity_fades_with_height() {
    let low = shadow_patches(Vec3::new(0.5, 64.2, 0.5), 0.7, flat)[0].opacity;
    let high = shadow_patches(Vec3::new(0.5, 65.5, 0.5), 0.7, flat)[0].opacity;
    assert!(low > high && high > 0.0);
    assert_eq!(opacity_at(2.0), 0.0);
    assert!(shadow_patches(Vec3::new(0.5, 66.5, 0.5), 0.7, flat).is_empty());
}

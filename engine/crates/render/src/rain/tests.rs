use super::*;

fn alpha(pixels: &[u8], x: u32, y: u32) -> u8 {
    pixels[((y * TILE_WIDTH + x) * 4 + 3) as usize]
}

#[test]
fn one_instance_per_cover_cell() {
    assert_eq!(RAIN_INSTANCES, RAIN_GRID * RAIN_GRID);
}

#[test]
fn tile_has_the_expected_size_and_is_deterministic() {
    let tile = rain_tile_pixels();
    assert_eq!(tile.len(), (TILE_WIDTH * TILE_HEIGHT * 4) as usize);
    assert_eq!(tile, rain_tile_pixels());
}

#[test]
fn tile_is_sparse_with_translucent_streaks() {
    let tile = rain_tile_pixels();
    let filled = tile.as_chunks::<4>().0.iter().filter(|p| p[3] > 0).count() as f32;
    let share = filled / (TILE_WIDTH * TILE_HEIGHT) as f32;
    assert!((0.01..=0.10).contains(&share), "{share}");
    assert!(
        tile.as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[3] == 0 || (80..=170).contains(&p[3]))
    );
    assert!(
        tile.as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0)
            .all(|p| p[2] > p[0])
    );
}

#[test]
fn streaks_are_vertical_and_vary_in_length() {
    let tile = rain_tile_pixels();
    let mut lengths = Vec::new();
    for x in 0..TILE_WIDTH {
        let mut run = 0;
        for y in 0..TILE_HEIGHT {
            if alpha(&tile, x, y) > 0 {
                run += 1;
            } else if run > 0 {
                lengths.push(run);
                run = 0;
            }
        }
        assert_eq!(run, 0, "streak touches the tile edge in column {x}");
    }
    assert!(
        lengths.iter().all(|l| STREAK_LENGTH.contains(l)),
        "{lengths:?}"
    );
    assert!(lengths.iter().min() < lengths.iter().max());
    assert!(lengths.len() >= 4);
}

#[test]
fn streak_head_is_lighter_than_tail() {
    let tile = rain_tile_pixels();
    let x = (0..TILE_WIDTH)
        .find(|&x| (0..TILE_HEIGHT).any(|y| alpha(&tile, x, y) > 0))
        .expect("tile has streaks");
    let ys: Vec<u32> = (0..TILE_HEIGHT)
        .filter(|&y| alpha(&tile, x, y) > 0)
        .collect();
    let (first, last) = (ys[0], *ys.last().unwrap());
    let px = |y: u32| tile[((y * TILE_WIDTH + x) * 4) as usize];
    assert!(px(first) > px(last));
}

#[test]
fn streaks_are_at_most_two_texels_wide() {
    let tile = rain_tile_pixels();
    for y in 0..TILE_HEIGHT {
        let mut run = 0;
        for x in 0..TILE_WIDTH {
            run = if alpha(&tile, x, y) > 0 { run + 1 } else { 0 };
            assert!(run <= 2, "row {y}");
        }
    }
}

fn pcg(v: u32) -> u32 {
    let state = v.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

fn hash(cell: (i32, i32), salt: u32) -> [f32; 3] {
    let a = pcg((cell.0 as u32).wrapping_mul(2_654_435_761)
        ^ pcg((cell.1 as u32).wrapping_add(salt.wrapping_mul(2_246_822_519))));
    let b = pcg(a);
    let c = pcg(b);
    [a, b, c].map(|v| (v >> 8) as f32 / 16_777_216.0)
}

fn planes_around(camera: (f32, f32)) -> Vec<(f32, f32)> {
    let base = (camera.0.floor() as i32, camera.1.floor() as i32);
    let half = RAIN_GRID as i32 / 2;
    (0..RAIN_INSTANCES as i32)
        .map(|i| {
            let cell = (
                base.0 + i % RAIN_GRID as i32 - half,
                base.1 + i / RAIN_GRID as i32 - half,
            );
            (cell.0 as f32 + 0.5, cell.1 as f32 + 0.5)
        })
        .collect()
}

#[test]
fn hash_stays_in_unit_range_and_is_independent_per_cell() {
    let mut seen = std::collections::HashSet::new();
    for x in -20..20 {
        for z in -20..20 {
            let h = hash((x, z), 0);
            assert!(h.iter().all(|v| (0.0..1.0).contains(v)));
            assert_ne!(h, hash((x, z), 17));
            seen.insert(h[0].to_bits());
        }
    }
    assert!(seen.len() > 1500);
}

#[test]
fn planes_are_spread_evenly_around_the_camera_in_every_direction() {
    for camera in [(0.5, 0.5), (-37.2, 1001.9), (123.99, -0.01)] {
        let mut sectors = [0u32; 8];
        for (x, z) in planes_around(camera) {
            let (dx, dz) = (x - camera.0, z - camera.1);
            if dx.hypot(dz) > 10.0 {
                continue;
            }
            let angle = dz.atan2(dx).rem_euclid(std::f32::consts::TAU);
            sectors[(angle / (std::f32::consts::TAU / 8.0)) as usize % 8] += 1;
        }
        let mean = sectors.iter().sum::<u32>() as f32 / 8.0;
        for count in sectors {
            assert!(
                (count as f32 - mean).abs() < mean * 0.35,
                "{camera:?}: {sectors:?}"
            );
        }
    }
}

#[test]
fn grid_is_centered_on_the_camera_within_half_a_cell() {
    for camera in [(0.01, 0.99), (-5.5, 7.5), (99.9, -99.9)] {
        let planes = planes_around(camera);
        let n = planes.len() as f32;
        let mean = planes
            .iter()
            .fold((0.0, 0.0), |a, p| (a.0 + p.0 / n, a.1 + p.1 / n));
        assert!((mean.0 - camera.0).abs() < 1.0 && (mean.1 - camera.1).abs() < 1.0);
    }
}

#[test]
fn plane_faces_the_camera_for_eight_view_directions() {
    let camera = (10.3, -4.8);
    for k in 0..8 {
        let yaw = k as f32 * std::f32::consts::TAU / 8.0;
        let plane = (camera.0 + 5.0 * yaw.cos(), camera.1 + 5.0 * yaw.sin());
        let to_camera = (camera.0 - plane.0, camera.1 - plane.1);
        let right = (to_camera.1, -to_camera.0);
        let length = right.0.hypot(right.1);
        assert!(length > 1.0);
        assert!((right.0 * to_camera.0 + right.1 * to_camera.1).abs() < 1e-3);
    }
}

fn plane_alpha(center: (f32, f32), camera: (f32, f32)) -> f32 {
    let distance = (center.0 - camera.0).hypot(center.1 - camera.1);
    let t = ((distance - 6.0) / 4.0).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

#[test]
fn alpha_depends_only_on_horizontal_distance_and_is_symmetric() {
    let camera = (3.5, -2.5);
    for distance in [2.0f32, 6.0, 8.0, 10.0, 12.0] {
        let alphas: Vec<f32> = (0..8)
            .map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                plane_alpha(
                    (camera.0 + distance * a.cos(), camera.1 + distance * a.sin()),
                    camera,
                )
            })
            .collect();
        assert!(alphas.iter().all(|&a| (a - alphas[0]).abs() < 1e-6));
    }
    assert_eq!(plane_alpha((3.5, 3.5), camera), 1.0);
    assert_eq!(plane_alpha((13.5, -2.5), camera), 0.0);
}

#[test]
fn snow_tile_has_the_rain_tile_size_and_is_deterministic() {
    let tile = snow_tile_pixels();
    assert_eq!(tile.len(), (TILE_WIDTH * TILE_HEIGHT * 4) as usize);
    assert_eq!(tile, snow_tile_pixels());
}

#[test]
fn snowflakes_are_sparse_white_dots_of_one_or_two_texels() {
    let tile = snow_tile_pixels();
    let pixels = tile.as_chunks::<4>().0;
    let filled: Vec<&[u8; 4]> = pixels.iter().filter(|p| p[3] > 0).collect();
    let share = filled.len() as f32 / (TILE_WIDTH * TILE_HEIGHT) as f32;
    assert!((0.01..0.10).contains(&share), "{share}");
    assert!(filled.iter().all(|p| p[..3] == [255, 255, 255]));

    let mut seen = vec![false; pixels.len()];
    let mut sizes = Vec::new();
    for start in 0..pixels.len() {
        if pixels[start][3] == 0 || seen[start] {
            continue;
        }
        let mut stack = vec![start];
        seen[start] = true;
        let (mut count, mut min, mut max) = (0, (u32::MAX, u32::MAX), (0, 0));
        while let Some(at) = stack.pop() {
            let (x, y) = ((at as u32) % TILE_WIDTH, (at as u32) / TILE_WIDTH);
            count += 1;
            min = (min.0.min(x), min.1.min(y));
            max = (max.0.max(x), max.1.max(y));
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx < TILE_WIDTH && ny < TILE_HEIGHT {
                    let next = (ny * TILE_WIDTH + nx) as usize;
                    if pixels[next][3] > 0 && !seen[next] {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
        }
        assert!(
            max.0 - min.0 < 2 && max.1 - min.1 < 2,
            "flake {min:?}-{max:?}"
        );
        sizes.push(count);
    }
    assert!(sizes.len() >= 10);
    assert!(sizes.contains(&1) && sizes.contains(&4));
}

#[test]
fn snow_shader_falls_slower_with_sway_and_picks_its_half_of_the_texture() {
    let shader = crate::shaders::RAIN_SHADER;
    assert!(shader.contains("const SNOW_FALL_SPEED: f32 = 2.5;"));
    assert!(shader.contains("const FALL_SPEED: f32 = 12.0;"));
    assert!(shader.contains("snow: f32,"));
    assert!(shader.contains("select(0.0, 0.5, globals.snow > 0.5)"));
}

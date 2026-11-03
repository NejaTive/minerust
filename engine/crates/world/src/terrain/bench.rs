use super::*;

#[test]
#[ignore]
fn generate_timing() {
    let terrain = Terrain::new(1337);
    let keys: Vec<ColumnPos> = (-8..8)
        .flat_map(|z| (-8..8).map(move |x| ColumnPos::new(x, z)))
        .collect();
    let start = std::time::Instant::now();
    for &key in &keys {
        std::hint::black_box(terrain.generate(key));
    }
    let per_column = start.elapsed() / keys.len() as u32;
    println!("generate: {per_column:?} per column");
}

#[test]
#[ignore]
fn bench_points() {
    let terrain = Terrain::new(1337);
    let (mut plains, mut forest) = (Vec::new(), Vec::new());
    for cz in (-3000..3000).step_by(64) {
        for cx in (-3000..3000).step_by(64) {
            let (mut n, mut pl, mut fo, mut wet) = (0, 0, 0, 0);
            let (mut lo, mut hi) = (i32::MAX, i32::MIN);
            for dz in (-192..=192).step_by(16) {
                for dx in (-192..=192).step_by(16) {
                    if dx * dx + dz * dz > 192 * 192 {
                        continue;
                    }
                    let (x, z) = (cx + dx, cz + dz);
                    let h = terrain.surface_height(x, z);
                    n += 1;
                    lo = lo.min(h);
                    hi = hi.max(h);
                    wet += i32::from(h < WATER_LEVEL);
                    match terrain.biome_at(x, z) {
                        Biome::Plains => pl += 1,
                        Biome::Forest => fo += 1,
                        Biome::Desert | Biome::Taiga | Biome::Tundra => {}
                    }
                }
            }
            let f = |k: i32| k as f32 / n as f32;
            if f(wet) < 0.2 && f(pl) > 0.6 {
                plains.push((hi - lo, cx, cz, f(pl), f(wet)));
            }
            if f(fo) > 0.6 && (0.08..0.25).contains(&f(wet)) {
                forest.push(((f(wet) - 0.15).abs(), cx, cz, f(fo)));
            }
        }
    }
    plains.sort_by_key(|p| p.0);
    forest.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (x, z) in [(-248, 264), (-1016, 1416)] {
        println!("surface {x} {z}: {}", terrain.surface_height(x, z));
    }
    println!(
        "plains (range, x, z, share, wet): {:?}",
        &plains[..plains.len().min(5)]
    );
    println!(
        "forest (|wet-0.15|, x, z, share): {:?}",
        &forest[..forest.len().min(5)]
    );
}

#[test]
#[ignore]
fn climate_quantiles() {
    for seed in [1337, 42, 7, 5] {
        let terrain = Terrain::new(seed);
        let (mut t, mut h) = (Vec::new(), Vec::new());
        for z in (-20000..20000).step_by(100) {
            for x in (-20000..20000).step_by(100) {
                let (a, b) = terrain.climate(x, z);
                t.push(a);
                h.push(b);
            }
        }
        t.sort_by(f32::total_cmp);
        h.sort_by(f32::total_cmp);
        let q = |v: &Vec<f32>| {
            [
                0.07, 0.15, 0.3, 0.45, 0.55, 0.7, 0.78, 0.85, 0.93, 0.33, 0.67,
            ]
            .map(|p| v[(p * v.len() as f32) as usize])
        };
        println!("seed {seed} t {:.3?}\n h {:.3?}", q(&t), q(&h));
    }
}

#[test]
#[ignore]
fn share_report() {
    for seed in [1337, 42, 7, 5, 99, 2024] {
        let terrain = Terrain::new(seed);
        let s = |b| super::tests::biome_share(&terrain, b);
        println!(
            "seed {seed} plains {:.3} forest {:.3} desert {:.3} taiga {:.3} tundra {:.3}",
            s(Biome::Plains),
            s(Biome::Forest),
            s(Biome::Desert),
            s(Biome::Taiga),
            s(Biome::Tundra)
        );
    }
}

#[test]
#[ignore]
fn run_report() {
    for seed in [1337, 42, 7, 99] {
        let terrain = Terrain::new(seed);

        println!(
            "run {seed} all {:.0} snow {:.0} desert {:.0}",
            super::tests::mean_run_length(&terrain, |b| b),
            super::tests::mean_run_length(&terrain, Biome::is_snowy),
            super::tests::mean_run_length(&terrain, |b| b == Biome::Desert)
        );
    }
}

use super::*;
use crate::biome_color::BiomePalette;

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
fn tint_timing() {
    let terrain = Terrain::new(1337);
    let palette = BiomePalette::constants();
    let start = std::time::Instant::now();
    for i in 0..200 {
        std::hint::black_box(terrain.column_tint_bytes(ColumnPos::new(i, i), &palette));
    }
    println!("column_tint_bytes: {:?} per column", start.elapsed() / 200);
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
            super::tests::mean_run_length(&terrain, Biome::is_desert)
        );
    }
}

#[test]
#[ignore]
fn share_report() {
    for seed in [1337, 42, 7, 5, 99, 2024] {
        let terrain = Terrain::new(seed);
        let mut shares = std::collections::BTreeMap::new();
        let mut total = 0;
        for z in (-8000..8000).step_by(160) {
            for biome in terrain
                .map
                .blocks(-8000, z, 16_000, 1)
                .into_iter()
                .step_by(160)
            {
                *shares.entry(biome.name()).or_insert(0) += 1;
                total += 1;
            }
        }
        let report: Vec<String> = shares
            .iter()
            .map(|(name, n)| format!("{name} {:.1}%", *n as f32 * 100.0 / total as f32))
            .collect();
        println!("seed {seed}: {}", report.join(", "));
    }
}

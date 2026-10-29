use super::*;
use world::block::Block;

#[test]
fn blocks_follow_the_table_order() {
    let list = blocks();
    assert_eq!(list.len(), BLOCKS.len());
    assert_eq!(list.len(), Block::ALL.len());
    for (i, info) in list.iter().enumerate() {
        assert_eq!(info.name, BLOCKS[i].name);
        assert_eq!(info.label, BLOCKS[i].label);
    }
    assert_eq!(list[0].name, "air");
}

#[test]
fn placeable_matches_select_block_names() {
    for info in blocks() {
        assert_eq!(
            info.placeable,
            Block::from_name(&info.name).is_some(),
            "{}",
            info.name
        );
    }
    let placeable: Vec<_> = blocks().into_iter().filter(|b| b.placeable).collect();
    assert!(
        placeable
            .iter()
            .all(|b| b.name != "air" && b.name != "water")
    );
    assert!(!placeable.is_empty());
}

/// Фикстура для Vitest (`src/settings/blocks.fixture.json`): клиентские тесты не грузят
/// wasm, но сверяют хотбар с настоящим списком блоков. `UPDATE_FIXTURES=1 cargo test`
/// перезаписывает файл. Формат совпадает с выводом Prettier, чтобы `format:check` молчал.
#[test]
fn js_fixture_matches_blocks() {
    let rows: Vec<String> = blocks()
        .iter()
        .map(|b| {
            assert!(!b.name.contains(['"', '\\']) && !b.label.contains(['"', '\\']));
            format!(
                "  {{ \"name\": \"{}\", \"label\": \"{}\", \"placeable\": {} }}",
                b.name, b.label, b.placeable
            )
        })
        .collect();
    let expected = format!("[\n{}\n]\n", rows.join(",\n"));
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../src/settings/blocks.fixture.json"
    );
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(path, &expected).unwrap();
    }
    let actual = std::fs::read_to_string(path).unwrap_or_default();
    assert_eq!(
        actual, expected,
        "фикстура устарела: UPDATE_FIXTURES=1 cargo test -p engine js_fixture"
    );
}

#[test]
fn limits_match_engine_constants() {
    let l = settings_limits();
    assert_eq!(
        (l.render_distance_min, l.render_distance_max),
        (MIN_RADIUS, MAX_RADIUS)
    );
    assert_eq!((l.fov_min, l.fov_max), (MIN_FOV, MAX_FOV));
    assert_eq!(
        (l.mouse_sensitivity_min, l.mouse_sensitivity_max),
        (MIN_SENSITIVITY, MAX_SENSITIVITY)
    );
}

#[test]
fn defaults_are_within_limits() {
    let d = default_settings();
    let l = settings_limits();
    assert!((l.render_distance_min..=l.render_distance_max).contains(&d.render_distance));
    assert!((l.mouse_sensitivity_min..=l.mouse_sensitivity_max).contains(&d.mouse_sensitivity));
    assert!((l.fov_min..=l.fov_max).contains(&d.fov));
    assert!(d.ambient_occlusion);
}

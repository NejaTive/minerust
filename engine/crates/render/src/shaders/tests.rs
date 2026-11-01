use super::*;

fn module() -> naga::Module {
    naga::front::wgsl::parse_str(BLOCK_SHADER).expect("wgsl parses")
}

#[test]
fn block_shader_validates() {
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module())
    .expect("wgsl validates");
}

#[test]
fn shape_table_size_matches_rust_table() {
    let module = module();
    let (_, table) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("ShapeTable"))
        .expect("ShapeTable");
    let naga::TypeInner::Struct { span, .. } = table.inner else {
        panic!("ShapeTable is not a struct");
    };
    assert_eq!(span as usize, mesh::templates::TABLE_BYTES);
    assert_eq!(
        mesh::templates::table_bytes().len(),
        mesh::templates::TABLE_BYTES
    );
}

#[test]
fn animation_table_size_matches_layer_count() {
    let module = module();
    let (_, var) = module
        .global_variables
        .iter()
        .find(|(_, v)| v.name.as_deref() == Some("animation"))
        .expect("animation");
    let naga::TypeInner::Array { size, .. } = module.types[var.ty].inner else {
        panic!("animation is not an array");
    };
    assert_eq!(
        size,
        naga::ArraySize::Constant(std::num::NonZeroU32::new(world::tile::LAYER_COUNT).unwrap())
    );
}

#[test]
fn clouds_shader_validates() {
    let module = naga::front::wgsl::parse_str(CLOUDS_SHADER).expect("wgsl parses");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .expect("wgsl validates");
}

#[test]
fn bloom_shader_validates() {
    let module = naga::front::wgsl::parse_str(BLOOM_SHADER).expect("wgsl parses");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .expect("wgsl validates");
}

#[test]
fn every_shader_validates_with_the_full_globals_layout() {
    for (name, source) in [
        ("block", BLOCK_SHADER),
        ("sky", SKY_SHADER),
        ("clouds", CLOUDS_SHADER),
        ("rain", RAIN_SHADER),
        ("post", POST_SHADER),
        ("rays", RAYS_SHADER),
    ] {
        let module = naga::front::wgsl::parse_str(source).expect(name);
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let (_, globals) = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some("Globals"))
            .expect(name);
        let naga::TypeInner::Struct { span, .. } = globals.inner else {
            panic!("{name}: Globals is not a struct");
        };
        assert_eq!(span as usize, crate::globals::Globals::SIZE, "{name}");
    }
}

fn mixes_bitwise_with_arithmetic(line: &str) -> bool {
    let expr = line.split_once(" = ").map_or(line, |(_, rhs)| rhs);
    let (mut depth, mut bitwise, mut arithmetic) = (0, false, false);
    let chars: Vec<char> = expr.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let spaced = i > 0 && chars.get(i + 1) == Some(&' ') && chars[i - 1] == ' ';
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '^' | '|' | '&' if depth == 0 && spaced => bitwise = true,
            '*' | '+' | '-' | '/' if depth == 0 && spaced => arithmetic = true,
            _ => {}
        }
    }
    bitwise && arithmetic
}

#[test]
fn bitwise_operators_are_not_mixed_with_arithmetic_without_parentheses() {
    for (name, source) in [
        ("block", BLOCK_SHADER),
        ("sky", SKY_SHADER),
        ("clouds", CLOUDS_SHADER),
        ("rain", RAIN_SHADER),
        ("post", POST_SHADER),
        ("rays", RAYS_SHADER),
    ] {
        for line in source.lines() {
            let code = line.split("//").next().unwrap();
            assert!(
                !mixes_bitwise_with_arithmetic(code),
                "{name}: WGSL forbids mixing bitwise and arithmetic operators without parentheses: {code}"
            );
        }
    }
}

#[test]
fn bitwise_lint_catches_the_bad_pattern() {
    assert!(mixes_bitwise_with_arithmetic("let a = x * 3u ^ y;"));
    assert!(!mixes_bitwise_with_arithmetic("let a = (x * 3u) ^ y;"));
}

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

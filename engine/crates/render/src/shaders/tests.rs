use super::*;

fn module() -> naga::Module {
    naga::front::wgsl::parse_str(BLOCK_SHADER).expect("wgsl parses")
}

/// Шейдер блоков разбирается и проходит валидацию naga: опечатка в раскладке записи
/// иначе всплыла бы только при создании модуля в браузере.
#[test]
fn block_shader_validates() {
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module())
    .expect("wgsl validates");
}

/// Структура `ShapeTable` в шейдере занимает ровно столько, сколько байт отдаёт
/// `mesh::templates::table_bytes`: иначе uniform-буфер не совпал бы с привязкой.
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

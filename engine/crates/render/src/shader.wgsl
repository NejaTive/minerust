// Раскладка байтов — в globals.rs.
struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    // Туман — по горизонтальному расстоянию в блоках: иначе при полёте на высоте
    // затуманилась бы земля прямо под камерой.
    fog_start: f32,
    fog_color: vec3<f32>,
    fog_end: f32,
    // 1 — глаза под водой: туман по полному расстоянию и синий тон.
    underwater: f32,
    // Уровней солнечного света, которые отнимает ночь (0 днём); блочный свет не гаснет.
    darkness: f32,
};

// Множитель цвета под водой, линейный: гасит красный и зелёный сильнее синего.
// Яркость уровня света — как `light::level_brightness`: 0.8^(15 − уровень), не ниже
// минимума. Менять вместе с light.rs; константы сверяет тест в mesh.rs.
const LIGHT_BASE: f32 = 0.8;
const MIN_BRIGHTNESS: f32 = 0.05;
const MAX_LIGHT: f32 = 15.0;

const UNDERWATER_TINT: vec3<f32> = vec3<f32>(0.35, 0.6, 1.0);

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(0) @binding(1)
var block_textures: texture_2d_array<f32>;
@group(0) @binding(2)
var block_sampler: sampler;

// Вершина — два u32, раскладка в mesh.rs (`Vertex`): позиция локальная в 1/16 блока,
// UV в 1/16 тайла, слой и сторона × AO по байту, свет солнца и блоков по 5 бит в
// половинках уровня. `origin` — мировой угол секции, один на
// весь вызов (буфер с шагом по экземплярам).
struct VertexIn {
    @location(0) packed: vec2<u32>,
    @location(1) origin: vec3<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) shade: f32,
    // Целые varying нельзя интерполировать.
    @location(2) @interpolate(flat) layer: u32,
    @location(3) world_pos: vec3<f32>,
};

// Яркость вершины от света: больший из «солнце минус тьма» и блоков, в половинках.
fn vertex_brightness(sun_halves: u32, block_halves: u32) -> f32 {
    let level = max(f32(sun_halves) * 0.5 - globals.darkness, f32(block_halves) * 0.5);
    return max(pow(LIGHT_BASE, MAX_LIGHT - level), MIN_BRIGHTNESS);
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let a = in.packed.x;
    let b = in.packed.y;
    let local = vec3<f32>(
        f32(a & 0x1FFu),
        f32((a >> 9u) & 0x1FFu),
        f32((a >> 18u) & 0x1FFu),
    ) / 16.0;
    let world_pos = in.origin + local;
    var out: VertexOut;
    out.position = globals.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = vec2<f32>(f32(a >> 27u), f32(b & 0x1Fu)) / 16.0;
    out.shade = f32((b >> 13u) & 0xFFu) / 255.0
        * vertex_brightness((b >> 21u) & 0x1Fu, (b >> 26u) & 0x1Fu);
    out.layer = (b >> 5u) & 0xFFu;
    out.world_pos = world_pos;
    return out;
}

// Цвет грани с тоном и туманом.
fn shaded(in: VertexOut, texel: vec3<f32>) -> vec3<f32> {
    var color = texel * in.shade;
    var distance = length(in.world_pos.xz - globals.camera_pos.xz);
    if globals.underwater > 0.5 {
        color *= UNDERWATER_TINT;
        // Под водой дно в паре блоков под камерой уже должно тонуть в мути.
        distance = length(in.world_pos - globals.camera_pos);
    }
    let fog = smoothstep(globals.fog_start, globals.fog_end, distance);
    return mix(color, globals.fog_color, fog);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, in.uv, in.layer);
    // Дырки листвы. Альфа в текстуре двоичная на каждом мип-уровне (texture.rs),
    // поэтому порог не зависит от расстояния, а смешение пайплайну не нужно.
    if texel.a < 0.5 {
        discard;
    }
    return vec4<f32>(shaded(in, texel.rgb), 1.0);
}

// Вода: полупрозрачная, смешивается пайплайном; `discard` не нужен.
@fragment
fn fs_water(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, in.uv, in.layer);
    return vec4<f32>(shaded(in, texel.rgb), texel.a);
}

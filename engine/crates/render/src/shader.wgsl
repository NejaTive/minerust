// Раскладка байтов — в globals.rs.
struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    fog_start: f32,
    fog_color: vec3<f32>,
    fog_end: f32,
    eye_fluid: f32,
    darkness: f32,
    anim_time: f32,
    cloud_drift: f32,
    view_dir: mat4x4<f32>,
    sun_dir: vec3<f32>,
    night: f32,
    zenith: vec3<f32>,
    warm: f32,
    horizon_tilt: f32,
    star_cos: f32,
    star_sin: f32,
    cloud_color: vec3<f32>,
    rain: f32,
    rain_drops: f32,
    flash: f32,
    cloud_threshold: f32,
};

// Яркость уровня света — как `light::level_brightness`: 0.8^(15 − уровень), не ниже
// минимума. Менять вместе с light.rs; константы сверяет тест в mesh.rs.
const LIGHT_BASE: f32 = 0.8;
const MIN_BRIGHTNESS: f32 = 0.05;
const MAX_LIGHT: f32 = 15.0;

const UNDERWATER_TINT: vec3<f32> = vec3<f32>(0.35, 0.6, 1.0);
const LAVA_TINT: vec3<f32> = vec3<f32>(1.0, 0.45, 0.2);
const RAIN_DIM: f32 = 0.35;
const FLASH_GAIN: f32 = 1.5;

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(0) @binding(1)
var block_textures: texture_2d_array<f32>;
@group(0) @binding(2)
var block_sampler: sampler;

// Запись на слой: (кадры, кадров в секунду, прокрутка боковых граней в блоках в секунду, 0).
// Строит `world::tile::animation_table`; размер — `LAYER_COUNT`.
@group(0) @binding(3)
var<uniform> animation: array<vec4<f32>, 48>;

@group(1) @binding(0)
var<storage, read> quads: array<u32>;

// Таблица шаблонов квадов, байты даёт `mesh::templates::table_bytes` (размер сверяет тест
// рендера): на шаблон четыре упакованных угла (x, y, z, u, v по 5 бит, в 1/16) и яркость
// стороны в `.x`, флаг боковой грани жидкости (прокрутка) в `.y`; ao.x..w — яркость
// вершины по уровню AO.
struct Shape {
    corners: vec4<u32>,
    shade: vec4<f32>,
};
struct ShapeTable {
    shapes: array<Shape, 58>,
    ao: vec4<f32>,
};
@group(1) @binding(1)
var<uniform> table: ShapeTable;

// Слот меша — `first_instance`: мировой угол секции и начало записей квадов меша в
// арене (в словах u32), один на весь вызов (буфер с шагом по экземплярам).
struct VertexIn {
    @builtin(vertex_index) vertex: u32,
    @location(0) origin: vec3<f32>,
    @location(1) first_word: u32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) shade: f32,
    // Целые varying нельзя интерполировать.
    @location(2) @interpolate(flat) layer: u32,
    @location(3) world_pos: vec3<f32>,
};

fn vertex_brightness(sun_halves: u32, block_halves: u32) -> f32 {
    let level = max(f32(sun_halves) * 0.5 - globals.darkness, f32(block_halves) * 0.5);
    return max(pow(LIGHT_BASE, MAX_LIGHT - level), MIN_BRIGHTNESS);
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let at = in.first_word + (in.vertex / 4u) * 3u;
    let w0 = quads[at];
    let w1 = quads[at + 1u];
    let w2 = quads[at + 2u];
    let flip = (w0 >> 17u) & 1u;
    let corner = ((in.vertex % 4u) + flip) & 3u;
    let shape = ((w0 >> 12u) & 0x1Fu) | (((w0 >> 26u) & 1u) << 5u);
    let packed = table.shapes[shape].corners[corner];
    let cell = vec3<f32>(f32(w0 & 0xFu), f32((w0 >> 4u) & 0xFu), f32((w0 >> 8u) & 0xFu));
    var offset = vec3<f32>(
        f32(packed & 0x1Fu),
        f32((packed >> 5u) & 0x1Fu),
        f32((packed >> 10u) & 0x1Fu),
    );
    var v = f32((packed >> 20u) & 0x1Fu);
    // y = 30 и 31 — маркеры SIDE_TOP_CORNER / TOP_CORNER из templates.rs: высота угла берётся из квада
    if offset.y >= 30.0 {
        var stored = (w1 >> 28u) & 0xFu;
        if corner > 0u {
            stored = (w2 >> (16u + 4u * corner)) & 0xFu;
        }
        let top = f32(stored + 1u);
        if offset.y == 30.0 {
            v = 16.0 - top;
        }
        offset.y = top;
    }
    let world_pos = in.origin + cell + offset / 16.0;
    let ao = (w1 >> (2u * corner)) & 3u;
    let sun = (w1 >> (8u + 5u * corner)) & 0x1Fu;
    let block = (w2 >> (5u * corner)) & 0x1Fu;
    var out: VertexOut;
    out.position = globals.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = vec2<f32>(f32((packed >> 15u) & 0x1Fu), v) / 16.0;
    out.uv.y -= table.shapes[shape].shade.y * fract(globals.anim_time * animation[(w0 >> 18u) & 0xFFu].z);
    out.shade = table.shapes[shape].shade.x * table.ao[ao] * vertex_brightness(sun, block);
    out.layer = (w0 >> 18u) & 0xFFu;
    out.world_pos = world_pos;
    return out;
}

fn animated_layer(base: u32) -> u32 {
    let entry = animation[base];
    return base + u32(floor(globals.anim_time * entry.y)) % u32(entry.x);
}

fn shaded(in: VertexOut, texel: vec3<f32>) -> vec3<f32> {
    var color = texel * in.shade;
    color *= (1.0 - RAIN_DIM * globals.rain * (1.0 - globals.night)) * (1.0 + FLASH_GAIN * globals.flash);
    var distance = length(in.world_pos.xz - globals.camera_pos.xz);
    if globals.eye_fluid > 0.5 {
        color *= select(UNDERWATER_TINT, LAVA_TINT, globals.eye_fluid > 1.5);
        distance = length(in.world_pos - globals.camera_pos);
    }
    let fog = smoothstep(globals.fog_start, globals.fog_end, distance);
    return mix(color, globals.fog_color, fog);
}

// Непрозрачное: без `discard`, чтобы GPU мог отсекать скрытые пиксели до шейдера
// (TBDR отключает это для любого пайплайна с `discard`).
@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, in.uv, animated_layer(in.layer));
    return vec4<f32>(shaded(in, texel.rgb), 1.0);
}

// Листва, кресты, факел: дырки через `discard`. Альфа в текстуре двоичная на каждом
// мип-уровне (texture.rs), поэтому порог не зависит от расстояния, а смешение пайплайну
// не нужно.
@fragment
fn fs_cutout(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, in.uv, animated_layer(in.layer));
    if texel.a < 0.5 {
        discard;
    }
    return vec4<f32>(shaded(in, texel.rgb), 1.0);
}

@fragment
fn fs_water(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, in.uv, animated_layer(in.layer));
    return vec4<f32>(shaded(in, texel.rgb), texel.a);
}

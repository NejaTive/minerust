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
    effects: u32,
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

// Биты `Globals.effects`, как `Effects::bits` в effects.rs.
const FX_COLORED_LIGHT: u32 = 1u;
const FX_WATER_REFLECTIONS: u32 = 2u;
const FX_GROUND_FOG: u32 = 4u;
const FX_PLANT_SWAY: u32 = 8u;

fn effect(bit: u32) -> bool {
    return (globals.effects & bit) != 0u;
}

const UNDERWATER_TINT: vec3<f32> = vec3<f32>(0.35, 0.6, 1.0);
const LAVA_TINT: vec3<f32> = vec3<f32>(1.0, 0.45, 0.2);
const RAIN_DIM: f32 = 0.35;
const FLASH_GAIN: f32 = 1.5;

const BLOCK_LIGHT_TINT: vec3<f32> = vec3<f32>(1.25, 0.85, 0.5);
const DAY_LIGHT_TINT: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
const NIGHT_LIGHT_TINT: vec3<f32> = vec3<f32>(0.7, 0.82, 1.15);
// Как DUSK_SKY в daytime.rs и DUSK_COLOR в sky.wgsl.
const DUSK_COLOR: vec3<f32> = vec3<f32>(0.78, 0.32, 0.14);
// Уровень моря — SEA_LEVEL в terrain.rs.
const FOG_BASE_Y: f32 = 64.0;
const FOG_DENSITY: f32 = 0.7;
const FOG_FALLOFF: f32 = 0.03;
const FOG_RAIN_DENSITY: f32 = 1.0;
const FOG_GLOW_HEIGHT: f32 = 0.05;
const LEAF_SWAY: f32 = 0.025;
const GRASS_SWAY: f32 = 0.065;
const RAIN_SWAY_GAIN: f32 = 1.5;
const WATER_F0: f32 = 0.02;
const WATER_TOP_ALPHA: f32 = 0.8;
const RIPPLE_STRENGTH: f32 = 0.05;
const SPECULAR_POWER: f32 = 220.0;
const MOON_SPECULAR: f32 = 0.3;
const WARM_LIGHT_TINT: vec3<f32> = vec3<f32>(1.2, 0.95, 0.7);

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(0) @binding(1)
var block_textures: texture_2d_array<f32>;
@group(0) @binding(2)
var block_sampler: sampler;

// Запись на слой: (кадры, кадров в секунду, прокрутка боковых граней в блоках в секунду,
// колыхание: 0 нет, 1 листва, 2 трава и цветы).
// Строит `world::tile::animation_table`; размер — `LAYER_COUNT`.
@group(0) @binding(3)
var<uniform> animation: array<vec4<f32>, 58>;

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
    shapes: array<Shape, 63>,
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
    @location(4) sun_light: f32,
    @location(5) block_light: f32,
    @location(6) sky_exposure: f32,
    // Целые varying нельзя интерполировать.
    @location(2) @interpolate(flat) layer: u32,
    @location(3) world_pos: vec3<f32>,
};

fn level_brightness(level: f32) -> f32 {
    return max(pow(LIGHT_BASE, MAX_LIGHT - level), MIN_BRIGHTNESS);
}

// Только от мировой позиции: общие вершины соседних блоков смещаются одинаково.
fn sway_offset(p: vec3<f32>, kind: f32, free: bool) -> vec3<f32> {
    if !effect(FX_PLANT_SWAY) {
        return vec3<f32>(0.0);
    }
    let gust = 1.0 + RAIN_SWAY_GAIN * globals.rain;
    let t = globals.anim_time;
    let wave = vec3<f32>(
        sin(t * 1.9 + p.x * 0.9 + p.z * 0.7 + p.y * 0.5),
        sin(t * 2.3 + p.x * 0.6 + p.y * 0.8 - p.z * 0.4 + 1.7),
        sin(t * 1.7 - p.x * 0.5 + p.z * 0.9 + p.y * 0.6 + 3.1),
    );
    if kind > 1.5 && free {
        return vec3<f32>(wave.x, 0.0, wave.z) * GRASS_SWAY * gust;
    }
    if kind > 0.5 && kind < 1.5 {
        return wave * LEAF_SWAY * gust;
    }
    return vec3<f32>(0.0);
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
    var world_pos = in.origin + cell + offset / 16.0;
    let layer = (w0 >> 18u) & 0xFFu;
    world_pos += sway_offset(world_pos, animation[layer].w, offset.y > 0.0);
    let ao = (w1 >> (2u * corner)) & 3u;
    let sun = (w1 >> (8u + 5u * corner)) & 0x1Fu;
    let block = (w2 >> (5u * corner)) & 0x1Fu;
    var out: VertexOut;
    out.position = globals.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = vec2<f32>(f32((packed >> 15u) & 0x1Fu), v) / 16.0;
    out.uv.y -= table.shapes[shape].shade.y * fract(globals.anim_time * animation[layer].z);
    out.shade = table.shapes[shape].shade.x * table.ao[ao];
    out.sun_light = level_brightness(f32(sun) * 0.5 - globals.darkness);
    out.sky_exposure = level_brightness(f32(sun) * 0.5);
    out.block_light = level_brightness(f32(block) * 0.5);
    out.layer = layer;
    out.world_pos = world_pos;
    return out;
}

fn animated_layer(base: u32) -> u32 {
    let entry = animation[base];
    return base + u32(floor(globals.anim_time * entry.y)) % u32(entry.x);
}

fn luma_normalized(tint: vec3<f32>) -> vec3<f32> {
    return tint / dot(tint, vec3<f32>(0.299, 0.587, 0.114));
}

fn light_tint(in: VertexOut) -> vec3<f32> {
    if !effect(FX_COLORED_LIGHT) {
        return vec3<f32>(1.0);
    }
    var sun_tint = mix(DAY_LIGHT_TINT, NIGHT_LIGHT_TINT, globals.night);
    sun_tint = mix(sun_tint, WARM_LIGHT_TINT, globals.warm);
    let block = in.block_light - MIN_BRIGHTNESS;
    let sun = in.sun_light - MIN_BRIGHTNESS;
    let share = smoothstep(0.3, 0.7, block / (block + sun + 1e-4));
    return luma_normalized(mix(luma_normalized(sun_tint), luma_normalized(BLOCK_LIGHT_TINT), share));
}

fn apply_fog(color: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    let to_pos = world_pos - globals.camera_pos;
    if globals.eye_fluid > 0.5 {
        return mix(color, globals.fog_color, smoothstep(globals.fog_start, globals.fog_end, length(to_pos)));
    }
    let distance = length(to_pos.xz);
    let thickness = FOG_DENSITY * (1.0 + FOG_RAIN_DENSITY * globals.rain)
        * exp(-FOG_FALLOFF * max(world_pos.y - FOG_BASE_Y, 0.0)) / globals.fog_end;
    let linear_fog = smoothstep(globals.fog_start, globals.fog_end, distance);
    var fog = linear_fog;
    if effect(FX_GROUND_FOG) {
        fog = max(1.0 - exp(-thickness * distance), linear_fog);
    }
    let tint = mix(globals.fog_color, DUSK_COLOR * 1.3, dusk_glow(to_pos, FOG_GLOW_HEIGHT));
    return mix(color, tint, fog);
}

fn weather_exposure() -> f32 {
    return (1.0 - RAIN_DIM * globals.rain * (1.0 - globals.night)) * (1.0 + FLASH_GAIN * globals.flash);
}

fn shaded(in: VertexOut, texel: vec3<f32>) -> vec3<f32> {
    var color = texel * in.shade * max(in.sun_light, in.block_light) * light_tint(in);
    color *= weather_exposure();
    if globals.eye_fluid > 0.5 {
        color *= select(UNDERWATER_TINT, LAVA_TINT, globals.eye_fluid > 1.5);
    }
    return apply_fog(color, in.world_pos);
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

// Одинаково в sky.wgsl и shader.wgsl: туман сцены сходится с горизонтом неба.
fn dusk_glow(dir: vec3<f32>, height: f32) -> f32 {
    let sun_flat = normalize(vec2<f32>(globals.sun_dir.x, globals.sun_dir.z) + vec2<f32>(1e-4, 0.0));
    let side = max(dot(normalize(dir.xz + vec2<f32>(1e-5)), sun_flat), 0.0);
    let glow = globals.warm * pow(side, 3.0) * exp(-6.0 * height) * smoothstep(0.0, 0.07, height);
    return clamp(glow, 0.0, 1.0);
}

// Небо по лучу — та же формула, что в sky.wgsl (горизонт → зенит и закатное пятно).
fn sky_color(dir: vec3<f32>) -> vec3<f32> {
    let tilt = globals.horizon_tilt;
    let edge = -tilt / sqrt(1.0 + tilt * tilt);
    let height = clamp((dir.y - edge) / (1.0 - edge), 0.0, 1.0);
    var color = mix(globals.fog_color, globals.zenith, pow(smoothstep(0.0, 1.0, height), 0.6));
    return mix(color, DUSK_COLOR * 1.3, dusk_glow(dir, height));
}

fn ripple_normal(p: vec2<f32>) -> vec3<f32> {
    let t = globals.anim_time;
    let a = vec2<f32>(sin(p.x * 2.3 + p.y * 1.7 + t * 1.1), cos(p.x * 1.9 - p.y * 2.9 + t * 0.9));
    let b = vec2<f32>(sin(p.y * 5.1 - t * 1.7 + p.x * 1.3), cos(p.x * 4.7 + t * 1.4 - p.y * 0.8));
    let r = (a + 0.5 * b) * RIPPLE_STRENGTH;
    return normalize(vec3<f32>(r.x, 1.0, r.y));
}

@fragment
fn fs_water(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, in.uv, animated_layer(in.layer));
    let flat_normal = normalize(cross(dpdx(in.world_pos), dpdy(in.world_pos)));
    if globals.eye_fluid > 0.5 || !effect(FX_WATER_REFLECTIONS) {
        return vec4<f32>(shaded(in, texel.rgb), texel.a);
    }
    let view = normalize(globals.camera_pos - in.world_pos);
    var normal = select(-flat_normal, flat_normal, dot(flat_normal, view) >= 0.0);
    if normal.y > 0.95 {
        normal = ripple_normal(in.world_pos.xz);
    }
    let cos_view = clamp(dot(normal, view), 0.0, 1.0);
    let fresnel = WATER_F0 + (1.0 - WATER_F0) * pow(1.0 - cos_view, 5.0);
    let reflected = reflect(-view, normal);
    let exposure = weather_exposure();
    let sky = sky_color(vec3<f32>(reflected.x, abs(reflected.y), reflected.z)) * in.sky_exposure * exposure;

    let sun_up = smoothstep(-0.05, 0.1, globals.sun_dir.y);
    let moon_up = MOON_SPECULAR * smoothstep(-0.05, 0.1, -globals.sun_dir.y);
    let glint = sun_up * pow(max(dot(reflected, globals.sun_dir), 0.0), SPECULAR_POWER)
        + moon_up * pow(max(dot(reflected, -globals.sun_dir), 0.0), SPECULAR_POWER);
    let specular = glint * (1.0 - globals.rain) * smoothstep(0.4, 1.0, in.sky_exposure);

    let body = texel.rgb * in.shade * max(in.sun_light, in.block_light) * light_tint(in) * exposure;
    var color = mix(body, sky, fresnel) + vec3<f32>(specular);
    color = apply_fog(color, in.world_pos);
    let alpha = max(mix(texel.a * WATER_TOP_ALPHA, 1.0, smoothstep(0.0, 0.5, fresnel)), clamp(specular, 0.0, 1.0));
    return vec4<f32>(color, alpha);
}

@fragment
fn fs_ice(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, in.uv, animated_layer(in.layer));
    return vec4<f32>(shaded(in, texel.rgb), texel.a);
}

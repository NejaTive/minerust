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
    snow: f32,
    resolution: vec2<f32>,
    near: f32,
    far: f32,
    sun_screen: vec2<f32>,
    sun_ray: f32,
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
const FX_HDR: u32 = 16u;
const FX_WATER_DEPTH: u32 = 64u;
const FX_SHADOWS: u32 = 256u;

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
const RIPPLE_CALM: f32 = 0.12;
const RIPPLE_RAIN_GAIN: f32 = 1.2;
const RIPPLE_FADE_START: f32 = 24.0;
const RIPPLE_FADE_END: f32 = 90.0;
const RIPPLE_WIND_ANGLE: f32 = 0.6;
const RIPPLE_WIND_SWING: f32 = 0.15;
const RIPPLE_PHASE_WARP: f32 = 5.0;
const RIPPLE_WAVE_SPEED: f32 = 0.5;
const RIPPLE_DETAIL: f32 = 0.3;
const RIPPLE_WIND_SCALE: f32 = 0.025;
const SPECULAR_POWER: f32 = 220.0;
const MOON_SPECULAR: f32 = 0.3;
// Светящиеся тайлы (лава, светокамень, факел) в HDR ярче 1.0 и дают свечение.
const HDR_EMISSIVE_LEVEL: f32 = 2.0;
const EMISSIVE_FLAG: f32 = 4.0;
const HDR_GLINT_GAIN: f32 = 4.0;
const REFRACT_STRENGTH: f32 = 0.06;
const MAX_WATER_THICKNESS: f32 = 48.0;
const WATER_ABSORPTION: vec3<f32> = vec3<f32>(0.5, 0.14, 0.07);
const WATER_SHALLOW_TINT: vec3<f32> = vec3<f32>(0.55, 1.0, 0.9);
const WATER_DEEP_DARKEN: f32 = 0.3;
const SHORE_FADE: f32 = 0.35;
const WARM_LIGHT_TINT: vec3<f32> = vec3<f32>(1.2, 0.95, 0.7);

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(0) @binding(1)
var block_textures: texture_2d_array<f32>;
@group(0) @binding(2)
var block_sampler: sampler;

// Запись на слой: (кадры, кадров в секунду, прокрутка боковых граней в блоках в секунду,
// колыхание: 0 нет, 1 листва, 2 трава и цветы; плюс 4 — светящийся тайл, EMISSIVE_FLAG в tile.rs).
// Строит `world::tile::animation_table`; размер — `LAYER_COUNT`.
@group(0) @binding(3)
var<uniform> animation: array<vec4<f32>, 58>;

// Каскадные тени от солнца (или луны). Раскладка байтов — `ShadowData` в shadow.rs.
// matrices — мировая позиция в клип каскада; splits — дальняя граница каскада в линейной
// глубине камеры; light — (направление на источник, сила тени); texel_world — размер
// текселя карты в блоках; params — (каскадов, отсчётов PCF на сторону, 1 / разрешение).
struct ShadowData {
    matrices: array<mat4x4<f32>, 3>,
    splits: vec4<f32>,
    light: vec4<f32>,
    texel_world: vec4<f32>,
    params: vec4<f32>,
};
@group(0) @binding(7)
var<uniform> shadows: ShadowData;
@group(0) @binding(8)
var shadow_map: texture_depth_2d_array;
@group(0) @binding(9)
var shadow_sampler: sampler_comparison;

// Только проход теней: матрица каскада, отдельный буфер на каскад.
struct ShadowPass {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(10)
var<uniform> shadow_pass: ShadowPass;

// Сцена без воды: цвет и глубина копируются перед проходом воды (renderer.rs).
@group(2) @binding(0)
var scene_color: texture_2d<f32>;
@group(2) @binding(1)
var scene_depth: texture_depth_2d;
@group(2) @binding(2)
var scene_sampler: sampler;

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

fn is_emissive(flags: f32) -> bool {
    return flags >= EMISSIVE_FLAG - 0.5;
}

fn sway_kind(flags: f32) -> f32 {
    return select(flags, flags - EMISSIVE_FLAG, is_emissive(flags));
}

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

fn block_vertex(in: VertexIn) -> VertexOut {
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
    world_pos += sway_offset(world_pos, sway_kind(animation[layer].w), offset.y > 0.0);
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

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    return block_vertex(in);
}

struct ShadowOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
};

// Те же вершины, что в vs_main (вместе с колыханием), но с матрицей каскада.
@vertex
fn vs_shadow(in: VertexIn) -> ShadowOut {
    let v = block_vertex(in);
    var out: ShadowOut;
    out.position = shadow_pass.view_proj * vec4<f32>(v.world_pos, 1.0);
    out.uv = v.uv;
    out.layer = v.layer;
    return out;
}

@fragment
fn fs_shadow_cutout(in: ShadowOut) {
    let texel = textureSample(block_textures, block_sampler, in.uv, animated_layer(in.layer));
    if texel.a < 0.5 {
        discard;
    }
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

const SHADOW_FLOOR: f32 = 0.45;
const SHADOW_NORMAL_OFFSET: f32 = 1.5;
const SHADOW_BLEND: f32 = 0.9;
const TWO_SIDED_LIFT: f32 = 0.03;

// Плоская нормаль по производным, лицом к камере. Только в однородном потоке управления.
fn flat_normal(world_pos: vec3<f32>) -> vec3<f32> {
    let normal = normalize(cross(dpdx(world_pos), dpdy(world_pos)));
    return select(-normal, normal, dot(normal, globals.camera_pos - world_pos) >= 0.0);
}

// lift — сдвиг приёмника к свету в блоках: колышущаяся листва неплоская, и без него её
// квады затеняют сами себя пятнами по треугольникам.
fn shadow_cascade(c: u32, world_pos: vec3<f32>, normal: vec3<f32>, facing: f32, lift: f32) -> f32 {
    let offset = normal * (shadows.texel_world[c] * SHADOW_NORMAL_OFFSET * (1.0 - facing))
        + shadows.light.xyz * lift;
    let clip = shadows.matrices[c] * vec4<f32>(world_pos + offset, 1.0);
    let uv = vec2<f32>(clip.x * 0.5 + 0.5, 0.5 - clip.y * 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || clip.z > 1.0 {
        return 1.0;
    }
    let taps = i32(shadows.params.y);
    let start = f32(taps - 1) * 0.5;
    var lit = 0.0;
    for (var i = 0; i < taps; i++) {
        for (var j = 0; j < taps; j++) {
            let shift = (vec2<f32>(f32(i), f32(j)) - vec2<f32>(start)) * shadows.params.z;
            lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + shift, c, clip.z);
        }
    }
    return lit / f32(taps * taps);
}

// 1 — солнце видно, меньше — тень (с учётом силы тени).
// Не использует производные: нормаль считается снаружи.
fn sun_visibility(in: VertexOut, geometric: vec3<f32>, cutout: bool) -> f32 {
    if !effect(FX_SHADOWS) {
        return 1.0;
    }
    let strength = shadows.light.w;
    var normal = geometric;
    var lift = 0.0;
    var facing = dot(geometric, shadows.light.xyz);
    let kind = sway_kind(animation[in.layer].w);
    let sways = effect(FX_PLANT_SWAY);
    let gust = 1.0 + RAIN_SWAY_GAIN * globals.rain;
    // Кресты, трава, факел: тонкая двусторонняя геометрия, света «от солнца» у них нет.
    // Освещаются с обеих сторон, приёмник сдвигается к свету, а не по нормали.
    let two_sided = cutout && (kind < 0.5 || kind > 1.5);
    if two_sided {
        facing = 1.0;
        lift = TWO_SIDED_LIFT;
        if sways && kind > 1.5 {
            lift += 2.0 * GRASS_SWAY * gust;
        }
    } else if sways && kind > 0.5 && kind < 1.5 {
        lift = 2.0 * LEAF_SWAY * gust;
        // Грани листвы осевые; нормаль по производным у неплоского квада разная на треугольниках.
        let a = abs(geometric);
        if a.x >= a.y && a.x >= a.z {
            normal = vec3<f32>(sign(geometric.x), 0.0, 0.0);
        } else if a.y >= a.z {
            normal = vec3<f32>(0.0, sign(geometric.y), 0.0);
        } else {
            normal = vec3<f32>(0.0, 0.0, sign(geometric.z));
        }
        facing = dot(normal, shadows.light.xyz);
    }
    var visibility = 0.0;
    var fade = 0.0;
    if facing > 0.0 {
        let depth = linear_depth(in.position.z);
        let count = u32(shadows.params.x);
        var c = 0u;
        while c + 1u < count && depth > shadows.splits[c] {
            c++;
        }
        visibility = shadow_cascade(c, in.world_pos, normal, facing, lift);
        let edge = shadows.splits[c];
        let blend = smoothstep(edge * SHADOW_BLEND, edge, depth);
        if blend > 0.0 {
            if c + 1u < count {
                visibility = mix(visibility, shadow_cascade(c + 1u, in.world_pos, normal, facing, lift), blend);
            } else {
                fade = blend;
            }
        }
    }
    return 1.0 - strength * (1.0 - mix(visibility, 1.0, fade));
}

// Тень ограничивает солнечный свет сверху уровнем полного неба: где CPU-свет уже ниже
// (под кроной, в пещере), min ничего не меняет и не осветляет.
fn shadowed_sun(sun_light: f32, sun_vis: f32) -> f32 {
    let full_sky = max(level_brightness(MAX_LIGHT - globals.darkness), sun_light);
    return min(sun_light, full_sky * mix(SHADOW_FLOOR, 1.0, sun_vis));
}

fn shaded(in: VertexOut, texel: vec3<f32>, sun_vis: f32) -> vec3<f32> {
    let sun_light = shadowed_sun(in.sun_light, sun_vis);
    var color = texel * in.shade * max(sun_light, in.block_light) * light_tint(in);
    color *= weather_exposure();
    if effect(FX_HDR) && is_emissive(animation[in.layer].w) {
        color = max(color, texel * HDR_EMISSIVE_LEVEL);
    }
    if globals.eye_fluid > 0.5 {
        color *= select(UNDERWATER_TINT, LAVA_TINT, globals.eye_fluid > 1.5);
    }
    return apply_fog(color, in.world_pos);
}

// Непрозрачное: без `discard`, чтобы GPU мог отсекать скрытые пиксели до шейдера
// (TBDR отключает это для любого пайплайна с `discard`).
@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let normal = flat_normal(in.world_pos);
    let texel = textureSample(block_textures, block_sampler, in.uv, animated_layer(in.layer));
    return vec4<f32>(shaded(in, texel.rgb, sun_visibility(in, normal, false)), 1.0);
}

// Листва, кресты, факел: дырки через `discard`. Альфа в текстуре двоичная на каждом
// мип-уровне (texture.rs), поэтому порог не зависит от расстояния, а смешение пайплайну
// не нужно.
@fragment
fn fs_cutout(in: VertexOut) -> @location(0) vec4<f32> {
    let normal = flat_normal(in.world_pos);
    let texel = textureSample(block_textures, block_sampler, in.uv, animated_layer(in.layer));
    if texel.a < 0.5 {
        discard;
    }
    return vec4<f32>(shaded(in, texel.rgb, sun_visibility(in, normal, true)), 1.0);
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

fn cell_hash(c: vec2<i32>) -> f32 {
    var h = (u32(c.x) * 0x27d4eb2du) ^ (u32(c.y) * 0x165667b1u);
    h = (h ^ (h >> 15u)) * 0x85ebca6bu;
    h = h ^ (h >> 13u);
    return f32(h & 0xffffffu) / 16777215.0;
}

// Value-шум с аналитическими производными: (значение, d/dx, d/dy).
fn value_noise(p: vec2<f32>) -> vec3<f32> {
    let i = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let du = 30.0 * f * f * (f - 1.0) * (f - 1.0);
    let a = cell_hash(i);
    let b = cell_hash(i + vec2<i32>(1, 0));
    let c = cell_hash(i + vec2<i32>(0, 1));
    let d = cell_hash(i + vec2<i32>(1, 1));
    let k = a - b - c + d;
    let value = a + (b - a) * u.x + (c - a) * u.y + k * u.x * u.y;
    return vec3<f32>(value, du.x * ((b - a) + k * u.y), du.y * ((c - a) + k * u.x));
}

// Наклон одной октавы: масштаб, поворот (угол), скорость дрейфа, вес; footprint — размер пикселя в блоках.
fn ripple_octave(q: vec2<f32>, scale: f32, angle: f32, drift: vec2<f32>, weight: f32, footprint: f32) -> vec2<f32> {
    let rot = vec2<f32>(cos(angle), sin(angle));
    let r = vec2<f32>(q.x * rot.x - q.y * rot.y, q.x * rot.y + q.y * rot.x);
    let n = value_noise(r * scale + drift * globals.anim_time);
    let slope = vec2<f32>(n.y * rot.x + n.z * rot.y, n.z * rot.x - n.y * rot.y) * scale;
    let resolved = 1.0 - smoothstep(0.25, 0.6, footprint * scale);
    return slope * (weight * resolved / scale);
}

// Направленная волна с острым гребнем; шум сдвигает фазу и ломает гребни.
// phase_noise — (значение, d/dx, d/dy) в координатах шума, noise_scale — его масштаб.
fn ripple_wave(
    p: vec2<f32>,
    angle: f32,
    wavelength: f32,
    weight: f32,
    phase_noise: vec3<f32>,
    noise_scale: f32,
    footprint: f32,
) -> vec2<f32> {
    let dir = vec2<f32>(cos(angle), sin(angle));
    let k = 6.2831853 / wavelength;
    let phase = k * dot(dir, p) - k * RIPPLE_WAVE_SPEED * sqrt(wavelength) * globals.anim_time
        + (phase_noise.x - 0.5) * RIPPLE_PHASE_WARP;
    let dphase = k * dir + phase_noise.yz * (noise_scale * RIPPLE_PHASE_WARP);
    let crest = exp(sin(phase) - 1.0) * cos(phase);
    let resolved = 1.0 - smoothstep(0.25, 0.6, footprint / wavelength);
    return dphase * (crest * weight * resolved / k);
}

// footprint = length(fwidth(p)) считается снаружи: производные нельзя брать в неоднородном ветвлении.
fn ripple_normal(p: vec2<f32>, dist: f32, footprint: f32) -> vec3<f32> {
    let t = globals.anim_time;
    let wind = value_noise(p * RIPPLE_WIND_SCALE + vec2<f32>(t * 0.004, t * 0.006)).x;
    let gust = mix(RIPPLE_CALM, 1.0, smoothstep(0.3, 0.7, wind));
    let distance_fade = 1.0 - smoothstep(RIPPLE_FADE_START, RIPPLE_FADE_END, dist);
    let strength = RIPPLE_STRENGTH * gust * distance_fade * (1.0 + RIPPLE_RAIN_GAIN * globals.rain);
    let heading = RIPPLE_WIND_ANGLE + RIPPLE_WIND_SWING * sin(t * 0.02);
    let coarse = value_noise(p * 0.35 + vec2<f32>(t * 0.05, -t * 0.03));
    let fine = value_noise(p * 0.8 + vec2<f32>(-t * 0.08, t * 0.06) + vec2<f32>(31.7, 12.3));
    let slope = ripple_wave(p, heading - 0.65, 3.1, 1.0, coarse, 0.35, footprint)
        + ripple_wave(p, heading - 0.28, 2.1, 0.8, fine, 0.8, footprint)
        + ripple_wave(p, heading + 0.08, 1.4, 0.6, coarse, 0.35, footprint)
        + ripple_wave(p, heading + 0.4, 0.95, 0.45, fine, 0.8, footprint)
        + ripple_wave(p, heading + 0.66, 0.62, 0.3, coarse, 0.35, footprint)
        + ripple_octave(p, 6.0, 1.3, vec2<f32>(0.2, -0.1), RIPPLE_DETAIL, footprint);
    let r = slope * strength * 2.0;
    return normalize(vec3<f32>(-r.x, 1.0, -r.y));
}

fn linear_depth(raw: f32) -> f32 {
    return globals.near * globals.far / (globals.far - raw * (globals.far - globals.near));
}

fn scene_depth_at(pixel: vec2<f32>) -> f32 {
    let size = vec2<i32>(globals.resolution) - vec2<i32>(1);
    return linear_depth(textureLoad(scene_depth, clamp(vec2<i32>(pixel), vec2<i32>(0), size), 0));
}

@fragment
fn fs_water(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, in.uv, animated_layer(in.layer));
    var normal = flat_normal(in.world_pos);
    let sun_vis = sun_visibility(in, normal, false);
    if globals.eye_fluid > 0.5 || !effect(FX_WATER_REFLECTIONS) {
        return vec4<f32>(shaded(in, texel.rgb, sun_vis), texel.a);
    }
    let view = normalize(globals.camera_pos - in.world_pos);
    let ripple_footprint = length(fwidth(in.world_pos.xz));
    if normal.y > 0.95 {
        normal = ripple_normal(in.world_pos.xz, length(globals.camera_pos - in.world_pos), ripple_footprint);
    }
    let cos_view = clamp(dot(normal, view), 0.0, 1.0);
    let fresnel = WATER_F0 + (1.0 - WATER_F0) * pow(1.0 - cos_view, 5.0);
    let reflected = reflect(-view, normal);
    let exposure = weather_exposure();
    let sky = sky_color(vec3<f32>(reflected.x, abs(reflected.y), reflected.z)) * min(in.sky_exposure, mix(SHADOW_FLOOR, 1.0, sun_vis)) * exposure;

    let sun_up = smoothstep(-0.05, 0.1, globals.sun_dir.y);
    let moon_up = MOON_SPECULAR * smoothstep(-0.05, 0.1, -globals.sun_dir.y);
    let glint = sun_up * pow(max(dot(reflected, globals.sun_dir), 0.0), SPECULAR_POWER)
        + moon_up * pow(max(dot(reflected, -globals.sun_dir), 0.0), SPECULAR_POWER);
    var specular = glint * (1.0 - globals.rain) * smoothstep(0.4, 1.0, in.sky_exposure) * sun_vis;
    let alpha_glint = clamp(specular, 0.0, 1.0);
    if effect(FX_HDR) {
        specular *= HDR_GLINT_GAIN;
    }

    let lit = in.shade * max(shadowed_sun(in.sun_light, sun_vis), in.block_light) * light_tint(in) * exposure;
    let body = texel.rgb * lit;
    if effect(FX_WATER_DEPTH) {
        let pixel = in.position.xy;
        let own = linear_depth(in.position.z);
        let straight = clamp(scene_depth_at(pixel) - own, 0.0, MAX_WATER_THICKNESS);
        let depth_scale = clamp(straight * 0.5, 0.1, 1.0);
        let max_uv = (globals.resolution - vec2<f32>(1.0)) / globals.resolution;
        let shifted_uv = clamp(pixel / globals.resolution + normal.xz * (REFRACT_STRENGTH * depth_scale), vec2<f32>(0.0), max_uv);
        let shifted_depth = scene_depth_at(shifted_uv * globals.resolution);
        let behind = shifted_depth > own;
        let sample_uv = select(pixel / globals.resolution, shifted_uv, behind);
        let thickness = clamp(select(scene_depth_at(pixel), shifted_depth, behind) - own, 0.0, MAX_WATER_THICKNESS);
        let bed = textureSampleLevel(scene_color, scene_sampler, sample_uv, 0.0).rgb;
        let transmit = exp(-thickness * WATER_ABSORPTION);
        let tint = mix(vec3<f32>(1.0), WATER_SHALLOW_TINT, 1.0 - exp(-thickness * 0.6));
        let deep = texel.rgb * lit * WATER_DEEP_DARKEN;
        let through = bed * tint * transmit + deep * (vec3<f32>(1.0) - transmit);
        var lit_color = mix(through, sky, fresnel) + vec3<f32>(specular);
        lit_color = apply_fog(lit_color, in.world_pos);
        let edge = smoothstep(0.0, SHORE_FADE, straight);
        let cover = max(edge, alpha_glint);
        return vec4<f32>(lit_color, cover);
    }
    var color = mix(body, sky, fresnel) + vec3<f32>(specular);
    color = apply_fog(color, in.world_pos);
    let alpha = max(mix(texel.a * WATER_TOP_ALPHA, 1.0, smoothstep(0.0, 0.5, fresnel)), alpha_glint);
    return vec4<f32>(color, alpha);
}

@fragment
fn fs_ice(in: VertexOut) -> @location(0) vec4<f32> {
    let normal = flat_normal(in.world_pos);
    let texel = textureSample(block_textures, block_sampler, in.uv, animated_layer(in.layer));
    return vec4<f32>(shaded(in, texel.rgb, sun_visibility(in, normal, false)), texel.a);
}

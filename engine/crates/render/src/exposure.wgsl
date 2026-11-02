// Автоэкспозиция: средний log2 яркости кадра (сетка 64×64 → 8×8 → 1×1) и адаптация. Константы — как в exposure.rs.
struct Params {
    dt: f32,
    reset: f32,
    pad0: f32,
    pad1: f32,
};

const KEY: f32 = 0.48;
const MIN_EXPOSURE: f32 = 0.7;
const MAX_EXPOSURE: f32 = 2.4;
const TAU_TO_DARK: f32 = 2.5;
const TAU_TO_LIGHT: f32 = 0.7;
const MIN_LUMA: f32 = 0.001;
const MAX_LUMA: f32 = 4.0;
const GRID: i32 = 64;
const SAMPLES: i32 = 6;
const BLOCK: i32 = 8;
const CENTER_FLOOR: f32 = 0.3;
const CENTER_FALLOFF: f32 = 2.5;

@group(0) @binding(0)
var src: texture_2d<f32>;
@group(0) @binding(1)
var src_sampler: sampler;

@group(0) @binding(2)
var prev_exposure: texture_2d<f32>;
@group(0) @binding(3)
var<uniform> params: Params;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    let xy = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOut;
    out.position = vec4<f32>(xy * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(xy.x, 1.0 - xy.y);
    return out;
}

@fragment
fn fs_sample(in: VertexOut) -> @location(0) vec4<f32> {
    let cell = 1.0 / f32(GRID);
    var sum = 0.0;
    for (var y = 0; y < SAMPLES; y++) {
        for (var x = 0; x < SAMPLES; x++) {
            let offset = ((vec2<f32>(f32(x), f32(y)) + vec2<f32>(0.5)) / f32(SAMPLES) - vec2<f32>(0.5)) * cell;
            let color = textureSampleLevel(src, src_sampler, in.uv + offset, 0.0).rgb;
            let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
            sum += log2(clamp(luma, MIN_LUMA, MAX_LUMA));
        }
    }
    return vec4<f32>(sum / f32(SAMPLES * SAMPLES), 0.0, 0.0, 1.0);
}

@fragment
fn fs_reduce(in: VertexOut) -> @location(0) vec4<f32> {
    let base = vec2<i32>(in.position.xy) * BLOCK;
    var sum = 0.0;
    for (var y = 0; y < BLOCK; y++) {
        for (var x = 0; x < BLOCK; x++) {
            sum += textureLoad(src, base + vec2<i32>(x, y), 0).r;
        }
    }
    return vec4<f32>(sum / f32(BLOCK * BLOCK), 0.0, 0.0, 1.0);
}

fn adapt(current: f32, target_value: f32, dt: f32) -> f32 {
    let tau = select(TAU_TO_LIGHT, TAU_TO_DARK, target_value > current);
    let blend = 1.0 - exp(-max(dt, 0.0) / tau);
    return exp2(mix(log2(current), log2(target_value), blend));
}

@fragment
fn fs_adapt(in: VertexOut) -> @location(0) vec4<f32> {
    var sum = 0.0;
    var total = 0.0;
    for (var y = 0; y < BLOCK; y++) {
        for (var x = 0; x < BLOCK; x++) {
            let p = (vec2<f32>(f32(x), f32(y)) + vec2<f32>(0.5)) / f32(BLOCK) * 2.0 - vec2<f32>(1.0);
            let weight = mix(CENTER_FLOOR, 1.0, exp(-dot(p, p) * CENTER_FALLOFF));
            sum += textureLoad(src, vec2<i32>(x, y), 0).r * weight;
            total += weight;
        }
    }
    let target_value = clamp(KEY / exp2(sum / total), MIN_EXPOSURE, MAX_EXPOSURE);
    let current = textureLoad(prev_exposure, vec2<i32>(0), 0).r;
    let next = select(adapt(current, target_value, params.dt), target_value, params.reset > 0.5);
    return vec4<f32>(next, 0.0, 0.0, 1.0);
}

// Свечение: порог яркости и цепочка даунсемплов (13 выборок), затем апсемпл с tent-фильтром (аддитивно).
@group(0) @binding(0)
var src: texture_2d<f32>;
@group(0) @binding(1)
var src_sampler: sampler;
@group(0) @binding(2)
var exposure_tex: texture_2d<f32>;

const DISPLAY_THRESHOLD: f32 = 1.26;
const MIN_THRESHOLD: f32 = 0.9;
const KNEE_RATIO: f32 = 0.2857143;

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

fn tap(uv: vec2<f32>, offset: vec2<f32>, texel: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(src, src_sampler, uv + offset * texel, 0.0).rgb;
}

fn luma(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn prefilter(color: vec3<f32>, threshold: f32) -> vec3<f32> {
    let knee_width = threshold * KNEE_RATIO;
    let brightness = max(color.r, max(color.g, color.b));
    let soft = clamp(brightness - threshold + knee_width, 0.0, 2.0 * knee_width);
    let knee = soft * soft / (4.0 * knee_width + 1e-4);
    return color * (max(knee, brightness - threshold) / max(brightness, 1e-4));
}

fn karis(color: vec3<f32>) -> f32 {
    return 1.0 / (1.0 + luma(color));
}

@fragment
fn fs_down_first(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(src));
    let exposure = textureLoad(exposure_tex, vec2<i32>(0), 0).r;
    let threshold = max(DISPLAY_THRESHOLD / exposure, MIN_THRESHOLD);
    let a = prefilter(tap(in.uv, vec2<f32>(-2.0, 2.0), texel), threshold);
    let b = prefilter(tap(in.uv, vec2<f32>(0.0, 2.0), texel), threshold);
    let c = prefilter(tap(in.uv, vec2<f32>(2.0, 2.0), texel), threshold);
    let d = prefilter(tap(in.uv, vec2<f32>(-2.0, 0.0), texel), threshold);
    let e = prefilter(tap(in.uv, vec2<f32>(0.0, 0.0), texel), threshold);
    let f = prefilter(tap(in.uv, vec2<f32>(2.0, 0.0), texel), threshold);
    let g = prefilter(tap(in.uv, vec2<f32>(-2.0, -2.0), texel), threshold);
    let h = prefilter(tap(in.uv, vec2<f32>(0.0, -2.0), texel), threshold);
    let i = prefilter(tap(in.uv, vec2<f32>(2.0, -2.0), texel), threshold);
    let j = prefilter(tap(in.uv, vec2<f32>(-1.0, 1.0), texel), threshold);
    let k = prefilter(tap(in.uv, vec2<f32>(1.0, 1.0), texel), threshold);
    let l = prefilter(tap(in.uv, vec2<f32>(-1.0, -1.0), texel), threshold);
    let m = prefilter(tap(in.uv, vec2<f32>(1.0, -1.0), texel), threshold);
    let center = (j + k + l + m) * 0.25;
    let g0 = (a + b + d + e) * 0.25;
    let g1 = (b + c + e + f) * 0.25;
    let g2 = (d + e + g + h) * 0.25;
    let g3 = (e + f + h + i) * 0.25;
    let w0 = 0.5 * karis(center);
    let w1 = 0.125 * karis(g0);
    let w2 = 0.125 * karis(g1);
    let w3 = 0.125 * karis(g2);
    let w4 = 0.125 * karis(g3);
    let sum = center * w0 + g0 * w1 + g1 * w2 + g2 * w3 + g3 * w4;
    return vec4<f32>(sum / (w0 + w1 + w2 + w3 + w4), 1.0);
}

@fragment
fn fs_down(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(src));
    let a = tap(in.uv, vec2<f32>(-2.0, 2.0), texel);
    let b = tap(in.uv, vec2<f32>(0.0, 2.0), texel);
    let c = tap(in.uv, vec2<f32>(2.0, 2.0), texel);
    let d = tap(in.uv, vec2<f32>(-2.0, 0.0), texel);
    let e = tap(in.uv, vec2<f32>(0.0, 0.0), texel);
    let f = tap(in.uv, vec2<f32>(2.0, 0.0), texel);
    let g = tap(in.uv, vec2<f32>(-2.0, -2.0), texel);
    let h = tap(in.uv, vec2<f32>(0.0, -2.0), texel);
    let i = tap(in.uv, vec2<f32>(2.0, -2.0), texel);
    let j = tap(in.uv, vec2<f32>(-1.0, 1.0), texel);
    let k = tap(in.uv, vec2<f32>(1.0, 1.0), texel);
    let l = tap(in.uv, vec2<f32>(-1.0, -1.0), texel);
    let m = tap(in.uv, vec2<f32>(1.0, -1.0), texel);
    let sum = e * 0.125 + (a + c + g + i) * 0.03125 + (b + d + f + h) * 0.0625 + (j + k + l + m) * 0.125;
    return vec4<f32>(sum, 1.0);
}

@fragment
fn fs_up(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(src));
    var sum = tap(in.uv, vec2<f32>(0.0, 0.0), texel) * 4.0;
    sum += (tap(in.uv, vec2<f32>(-1.0, 0.0), texel) + tap(in.uv, vec2<f32>(1.0, 0.0), texel)
        + tap(in.uv, vec2<f32>(0.0, -1.0), texel) + tap(in.uv, vec2<f32>(0.0, 1.0), texel)) * 2.0;
    sum += tap(in.uv, vec2<f32>(-1.0, -1.0), texel) + tap(in.uv, vec2<f32>(1.0, -1.0), texel)
        + tap(in.uv, vec2<f32>(-1.0, 1.0), texel) + tap(in.uv, vec2<f32>(1.0, 1.0), texel);
    return vec4<f32>(sum / 16.0, 1.0);
}

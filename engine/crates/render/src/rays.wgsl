// Солнечные лучи: радиальное размытие маски «небо» от экранной позиции солнца, половинное разрешение. Раскладка Globals — в globals.rs.
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

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(1) @binding(1)
var scene_depth: texture_depth_2d;

const STEPS: i32 = 40;
const DENSITY: f32 = 0.92;
const DECAY: f32 = 0.985;
const SKY_DEPTH: f32 = 0.9999;
const GLOW_RADIUS: f32 = 1.3;

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
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let to_sun = globals.sun_screen - in.uv;
    let march = to_sun * (DENSITY / f32(STEPS));
    let limit = globals.resolution - vec2<f32>(1.0);
    var uv = in.uv;
    var weight = 1.0;
    var lit = 0.0;
    var total = 0.0;
    for (var i = 0; i < STEPS; i++) {
        uv += march;
        weight *= DECAY;
        let pixel = vec2<i32>(clamp(uv * globals.resolution, vec2<f32>(0.0), limit));
        let depth = textureLoad(scene_depth, pixel, 0);
        lit += select(0.0, weight, depth >= SKY_DEPTH);
        total += weight;
    }
    let glow = pow(saturate(1.0 - length((to_sun) * vec2<f32>(globals.resolution.x / globals.resolution.y, 1.0)) / GLOW_RADIUS), 2.0);
    return vec4<f32>(lit / total * glow, 0.0, 0.0, 1.0);
}

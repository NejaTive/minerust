// Облака: коробки 12x4x12 инстансами по сетке вокруг камеры. Раскладка Globals — в globals.rs;
// константы сетки и маски сверяет тест в web/shader_constants.rs.
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

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(0) @binding(4)
var cloud_mask: texture_2d<u32>;

const CLOUD_MAP: i32 = 256;
const CLOUD_CELL: f32 = 12.0;
const CLOUD_Y: f32 = 160.0;
const CLOUD_HEIGHT: f32 = 4.0;
const GRID_RADIUS: i32 = 32;
const GRID_SIZE: u32 = 64u;
const FADE_START: f32 = 200.0;
const FADE_END: f32 = 360.0;
const CLOUD_ALPHA: f32 = 0.8;

var<private> CORNERS: array<vec3<f32>, 24> = array<vec3<f32>, 24>(
    vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 1.0, 1.0), vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(1.0, 1.0, 0.0),
    vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(1.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, 1.0),
    vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(1.0, 0.0, 1.0),
    vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 1.0), vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 1.0), vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.0, 1.0, 1.0),
    vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0),
);
// Порядок граней: +Y, -Y, +X, -X, +Z, -Z.
var<private> NEIGHBOR: array<vec2<i32>, 6> = array<vec2<i32>, 6>(
    vec2<i32>(0, 0), vec2<i32>(0, 0), vec2<i32>(1, 0), vec2<i32>(-1, 0), vec2<i32>(0, 1), vec2<i32>(0, -1),
);
var<private> SHADE: array<f32, 6> = array<f32, 6>(1.0, 0.65, 0.8, 0.8, 0.8, 0.8);
var<private> QUAD: array<u32, 6> = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) shade: f32,
    @location(1) distance: f32,
};

fn is_cloud(cell: vec2<i32>) -> bool {
    return f32(textureLoad(cloud_mask, cell & vec2<i32>(CLOUD_MAP - 1), 0).x) >= globals.cloud_threshold;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> VertexOut {
    let face = vertex / 6u;
    let drift = globals.cloud_drift;
    let base = vec2<i32>(floor((globals.camera_pos.xz - vec2<f32>(drift, 0.0)) / CLOUD_CELL));
    let cell = base + vec2<i32>(i32(instance % GRID_SIZE), i32(instance / GRID_SIZE)) - vec2<i32>(GRID_RADIUS);
    var visible = is_cloud(cell);
    if face >= 2u {
        visible = visible && !is_cloud(cell + NEIGHBOR[face]);
    }
    var out: VertexOut;
    out.shade = SHADE[face];
    if !visible {
        out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        out.distance = 0.0;
        return out;
    }
    let corner = CORNERS[face * 4u + QUAD[vertex % 6u]];
    let origin = vec2<f32>(cell) * CLOUD_CELL + vec2<f32>(drift, 0.0);
    let world = vec3<f32>(origin.x + corner.x * CLOUD_CELL, CLOUD_Y + corner.y * CLOUD_HEIGHT, origin.y + corner.z * CLOUD_CELL);
    let clip = globals.view_proj * vec4<f32>(world, 1.0);
    // Облака дальше far plane мира: прижимаем глубину к дальней плоскости, иначе их режет по прямой.
    out.position = vec4<f32>(clip.xy, min(clip.z, clip.w * 0.99999), clip.w);
    out.distance = length(world.xz - globals.camera_pos.xz);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let fade = smoothstep(FADE_START, FADE_END, in.distance);
    let color = mix(globals.cloud_color * in.shade, globals.fog_color, fade);
    return vec4<f32>(color, (CLOUD_ALPHA + 0.15 * globals.rain) * (1.0 - fade));
}

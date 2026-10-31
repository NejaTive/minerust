// Дождь: вертикальные плоскости к камере по одной на колонку, текстура штрихов прокручивается вниз. Раскладка Globals — в globals.rs;
// размер сетки и карты укрытий сверяет тест в web/shader_constants.rs.
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
@group(0) @binding(2)
var tile_sampler: sampler;
@group(0) @binding(5)
var rain_cover: texture_2d<u32>;
@group(0) @binding(6)
var rain_tile: texture_2d<f32>;

const GRID_SIZE: u32 = 32u;
const GRID_RADIUS: i32 = 16;
const HALF_SPAN: f32 = 16.0;
const TILE_BLOCKS: f32 = 4.0;
const FALL_SPEED: f32 = 12.0;
const FADE_START: f32 = 6.0;
const FADE_END: f32 = 10.0;
const MIN_DISTANCE: f32 = 0.3;
const HEIGHT_FADE_START: f32 = 12.0;
const HEIGHT_FADE_END: f32 = 16.0;
const NIGHT_LIGHT: f32 = 0.25;
var<private> QUAD: array<u32, 6> = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
    @location(2) height: f32,
};

fn pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

// Три независимых значения [0, 1) на клетку; зеркало в rain/tests.rs.
fn hash(cell: vec2<i32>, salt: u32) -> vec3<f32> {
    let a = pcg((u32(cell.x) * 2654435761u) ^ pcg(u32(cell.y) + salt * 2246822519u));
    let b = pcg(a);
    let c = pcg(b);
    return vec3<f32>(f32(a >> 8u), f32(b >> 8u), f32(c >> 8u)) / 16777216.0;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> VertexOut {
    let base = vec2<i32>(floor(globals.camera_pos.xz));
    let cell = base + vec2<i32>(i32(instance % GRID_SIZE), i32(instance / GRID_SIZE)) - vec2<i32>(GRID_RADIUS);
    let h = hash(cell, 0u);
    let h2 = hash(cell, 17u);
    var out: VertexOut;
    out.uv = vec2<f32>(0.0);
    out.alpha = 0.0;
    out.height = 0.0;
    out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    if h.z >= globals.rain_drops {
        return out;
    }
    let cover = f32(textureLoad(rain_cover, cell & vec2<i32>(i32(GRID_SIZE) - 1), 0).x);
    let top_y = floor(globals.camera_pos.y) + HALF_SPAN;
    let bottom_y = max(cover, floor(globals.camera_pos.y) - HALF_SPAN);
    if top_y <= bottom_y {
        return out;
    }
    let xz = vec2<f32>(cell) + vec2<f32>(0.5);
    let distance = length(xz - globals.camera_pos.xz);
    if distance < MIN_DISTANCE {
        return out;
    }
    let to_camera = vec3<f32>(globals.camera_pos.x - xz.x, 0.0, globals.camera_pos.z - xz.y);
    let right = vec3<f32>(to_camera.z, 0.0, -to_camera.x) / max(length(to_camera), MIN_DISTANCE);
    let corner = QUAD[vertex];
    let side = select(-0.5, 0.5, corner == 1u || corner == 2u);
    let y = select(bottom_y, top_y, corner >= 2u);
    let point = vec3<f32>(xz.x, y, xz.y) + right * side;
    let scroll = fract(globals.anim_time * FALL_SPEED / TILE_BLOCKS);
    out.uv = vec2<f32>(select(0.0, 1.0, corner == 1u || corner == 2u) + h.x, -y / TILE_BLOCKS - scroll + h2.y);
    out.alpha = 1.0 - smoothstep(FADE_START, FADE_END, distance);
    out.height = y - globals.camera_pos.y;
    out.position = globals.view_proj * vec4<f32>(point, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(rain_tile, tile_sampler, in.uv);
    let light = mix(1.0, NIGHT_LIGHT, globals.night) * (1.0 + globals.flash);
    let color = mix(texel.rgb, globals.fog_color * 1.5, 0.25) * light;
    return vec4<f32>(color, texel.a * in.alpha * (1.0 - smoothstep(HEIGHT_FADE_START, HEIGHT_FADE_END, abs(in.height))));
}

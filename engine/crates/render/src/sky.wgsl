// Небо: один полноэкранный треугольник на дальней плоскости. Раскладка Globals — в globals.rs.
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

const SUN_HALF: f32 = 0.05;
const MOON_HALF: f32 = 0.04;
const SUN_COLOR: vec3<f32> = vec3<f32>(1.6, 1.45, 1.1);
const SUN_LOW_COLOR: vec3<f32> = vec3<f32>(1.7, 0.7, 0.3);
const MOON_COLOR: vec3<f32> = vec3<f32>(0.75, 0.8, 0.9);
// Как DUSK_SKY в daytime.rs (сверяет тест).
const DUSK_COLOR: vec3<f32> = vec3<f32>(0.78, 0.32, 0.14);
const STAR_GRID: f32 = 48.0;
const STAR_CHANCE: f32 = 0.12;
// Радиус звезды в ячейках: меньше 0.25, иначе диск вылезет из своей ячейки и обрежется.
const STAR_RADIUS: f32 = 0.12;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    let xy = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    var out: VertexOut;
    // z = w = 1: глубина 1.0, тест LessEqual проходит только там, где рельефа нет.
    out.position = vec4<f32>(xy, 1.0, 1.0);
    out.ndc = xy;
    return out;
}

fn hash(cell: vec3<i32>) -> vec3<f32> {
    var h = bitcast<vec3<u32>>(cell) * vec3<u32>(747796405u, 2891336453u, 277803737u);
    h = h ^ (h.yzx >> vec3<u32>(15u));
    h = h * vec3<u32>(2246822519u, 3266489917u, 668265263u);
    h = h ^ (h.zxy >> vec3<u32>(13u));
    return vec3<f32>(h & vec3<u32>(0xffffffu)) / 16777216.0;
}

fn moon_surface(uv: vec2<f32>) -> f32 {
    let cell = vec2<i32>(floor((clamp(uv, vec2<f32>(-1.0), vec2<f32>(0.999)) * 0.5 + 0.5) * 8.0));
    let h = hash(vec3<i32>(cell, 31)).x;
    return select(select(0.0, 0.4, h > 0.55), 0.9, h > 0.82);
}

fn stars(dir: vec3<f32>) -> f32 {
    let c = globals.star_cos;
    let s = globals.star_sin;
    let d = vec3<f32>(c * dir.x + s * dir.y, -s * dir.x + c * dir.y, dir.z);
    let p = d * STAR_GRID;
    let cell = vec3<i32>(floor(p));
    // Производные считаются до ветвлений: в неоднородном потоке WGSL их запрещает.
    let footprint = length(fwidth(p));
    let h = hash(cell);
    let h2 = hash(cell + vec3<i32>(7919, 104729, 1299709));
    let center = vec3<f32>(cell) + 0.25 + 0.5 * h2;
    let radius = max(STAR_RADIUS, 0.8 * footprint);
    let falloff = 1.0 - smoothstep(0.0, radius, distance(p, center));
    let dim = (STAR_RADIUS * STAR_RADIUS) / (radius * radius);
    return select(0.0, falloff * falloff * dim * (0.5 + 1.5 * h.y), h.x <= STAR_CHANCE);
}

fn square_body(dir: vec3<f32>, light: vec3<f32>, half_size: f32) -> vec4<f32> {
    let right = normalize(cross(vec3<f32>(0.0, 0.0, 1.0), light));
    let up = cross(light, right);
    let forward = dot(dir, light);
    // Деление на forward <= 0 даёт мусор, поэтому маска ниже; ветвления нет из-за fwidth.
    let u = dot(dir, right) / max(forward, 1e-3) / half_size;
    let v = dot(dir, up) / max(forward, 1e-3) / half_size;
    let m = max(abs(u), abs(v));
    let aa = max(fwidth(m), 1e-4);
    let disc = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, m);
    let halo = exp(-1.4 * m) * 0.35 / (1.0 + 0.1 * m * m);
    let front = select(0.0, 1.0, forward > 0.0);
    return vec4<f32>(disc * front, halo * front, u, v);
}

// Одинаково в sky.wgsl и shader.wgsl: туман сцены сходится с горизонтом неба.
fn dusk_glow(dir: vec3<f32>, height: f32) -> f32 {
    let sun_flat = normalize(vec2<f32>(globals.sun_dir.x, globals.sun_dir.z) + vec2<f32>(1e-4, 0.0));
    let side = max(dot(normalize(dir.xz + vec2<f32>(1e-5)), sun_flat), 0.0);
    let glow = globals.warm * pow(side, 3.0) * exp(-6.0 * height) * smoothstep(0.0, 0.07, height);
    return clamp(glow, 0.0, 1.0);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let dir = normalize((globals.view_dir * vec4<f32>(in.ndc, 1.0, 0.0)).xyz);
    let tilt = globals.horizon_tilt;
    let edge = -tilt / sqrt(1.0 + tilt * tilt);
    let height = clamp((dir.y - edge) / (1.0 - edge), 0.0, 1.0);
    var color = mix(globals.fog_color, globals.zenith, pow(smoothstep(0.0, 1.0, height), 0.6));

    color = mix(color, DUSK_COLOR * 1.3, dusk_glow(dir, height));

    let above = smoothstep(edge - 0.06, edge - 0.03, dir.y);
    let clear = 1.0 - globals.rain;
    let sun = square_body(dir, globals.sun_dir, SUN_HALF) * clear;
    var moon = square_body(dir, -globals.sun_dir, MOON_HALF);
    let moon_uv = moon.zw;
    moon = moon * vec4<f32>(clear, clear, 1.0, 1.0);
    let sun_tint = mix(SUN_LOW_COLOR, SUN_COLOR, smoothstep(0.0, 0.4, globals.sun_dir.y));
    color += sun_tint * sun.y * 0.5 * above;
    color = mix(color, sun_tint, sun.x * above);
    let moon_color = MOON_COLOR * (1.0 - 0.5 * moon_surface(moon_uv));
    color += MOON_COLOR * moon.y * 0.15 * above;
    color = mix(color, moon_color, moon.x * above * (1.0 - sun.x));

    let star_fade = globals.night * clear * above * (0.4 + 0.6 * smoothstep(0.0, 0.3, dir.y));
    color += vec3<f32>(0.9, 0.95, 1.0) * stars(dir) * star_fade * clamp(1.0 - moon.x - 0.5 * moon.y, 0.0, 1.0);
    let noise = fract(52.9829189 * fract(dot(in.position.xy, vec2<f32>(0.06711056, 0.00583715))));
    let encoded = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2)) + (noise - 0.5) / 255.0;
    return vec4<f32>(pow(max(encoded, vec3<f32>(0.0)), vec3<f32>(2.2)), 1.0);
}

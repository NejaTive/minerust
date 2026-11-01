// Постобработка: HDR-сцена → экспозиция → ACES → цветокоррекция → виньетка. Раскладка Globals — в globals.rs.
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
@group(1) @binding(0)
var scene: texture_2d<f32>;
@group(1) @binding(1)
var bloom: texture_2d<f32>;
@group(1) @binding(2)
var post_sampler: sampler;
@group(1) @binding(3)
var rays: texture_2d<f32>;

const FX_BLOOM: u32 = 32u;
const BLOOM_STRENGTH: f32 = 0.45;
const FX_SUN_RAYS: u32 = 128u;
const RAY_STRENGTH: f32 = 0.35;
const RAY_DAY_COLOR: vec3<f32> = vec3<f32>(1.0, 0.95, 0.8);
const RAY_DUSK_COLOR: vec3<f32> = vec3<f32>(1.0, 0.55, 0.25);

const EXPOSURE: f32 = 1.2;
const NIGHT_GRADE: vec3<f32> = vec3<f32>(0.85, 0.95, 1.2);
const NIGHT_GRADE_MIX: f32 = 0.25;
const WARM_GRADE: vec3<f32> = vec3<f32>(1.06, 1.0, 0.9);
const VIGNETTE_STRENGTH: f32 = 0.3;

const ACES_INPUT: mat3x3<f32> = mat3x3<f32>(
    vec3<f32>(0.59719, 0.07600, 0.02840),
    vec3<f32>(0.35458, 0.90834, 0.13383),
    vec3<f32>(0.04823, 0.01566, 0.83777),
);
const ACES_OUTPUT: mat3x3<f32> = mat3x3<f32>(
    vec3<f32>(1.60475, -0.10208, -0.00327),
    vec3<f32>(-0.53108, 1.10813, -0.07276),
    vec3<f32>(-0.07367, -0.00605, 1.07602),
);

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

fn aces_fitted(color: vec3<f32>) -> vec3<f32> {
    let v = ACES_INPUT * color;
    let a = v * (v + vec3<f32>(0.0245786)) - vec3<f32>(0.000090537);
    let b = v * (0.983729 * v + vec3<f32>(0.432951)) + vec3<f32>(0.238081);
    return clamp(ACES_OUTPUT * (a / b), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn grade(color: vec3<f32>) -> vec3<f32> {
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    var graded = mix(color, vec3<f32>(luma) * NIGHT_GRADE, NIGHT_GRADE_MIX * globals.night);
    graded *= mix(vec3<f32>(1.0), WARM_GRADE, globals.warm);
    return graded;
}

fn vignette(uv: vec2<f32>) -> f32 {
    let r = length((uv - vec2<f32>(0.5)) * 2.0);
    return 1.0 - VIGNETTE_STRENGTH * smoothstep(0.6, 1.5, r);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var hdr = textureLoad(scene, vec2<i32>(in.position.xy), 0).rgb;
    if (globals.effects & FX_BLOOM) != 0u {
        hdr += textureSampleLevel(bloom, post_sampler, in.uv, 0.0).rgb * BLOOM_STRENGTH;
    }
    if (globals.effects & FX_SUN_RAYS) != 0u {
        let shafts = textureSampleLevel(rays, post_sampler, in.uv, 0.0).r;
        hdr += mix(RAY_DAY_COLOR, RAY_DUSK_COLOR, globals.warm) * (shafts * globals.sun_ray * RAY_STRENGTH);
    }
    var color = aces_fitted(hdr * EXPOSURE);
    color = grade(color) * vignette(in.uv);
    let noise = fract(52.9829189 * fract(dot(in.position.xy, vec2<f32>(0.06711056, 0.00583715))));
    let encoded = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2)) + (noise - 0.5) / 255.0;
    return vec4<f32>(pow(max(encoded, vec3<f32>(0.0)), vec3<f32>(2.2)), 1.0);
}

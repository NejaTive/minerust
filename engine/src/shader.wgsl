struct Globals {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(0) @binding(1)
var block_textures: texture_2d_array<f32>;
@group(0) @binding(2)
var block_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) shade: f32,
    @location(3) layer: u32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) shade: f32,
    // Целые varying нельзя интерполировать.
    @location(2) @interpolate(flat) layer: u32,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = globals.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.shade = in.shade;
    out.layer = in.layer;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(block_textures, block_sampler, in.uv, in.layer).rgb;
    return vec4<f32>(color * in.shade, 1.0);
}

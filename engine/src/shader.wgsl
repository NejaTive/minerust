struct Globals {
    time: f32,
    aspect: f32,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.5),
        vec2<f32>(-0.5, -0.4),
        vec2<f32>(0.5, -0.4),
    );
    var colors = array<vec3<f32>, 3>(
        vec3<f32>(0.36, 0.62, 0.23),
        vec3<f32>(0.55, 0.35, 0.17),
        vec3<f32>(0.50, 0.50, 0.50),
    );

    let p = positions[index];
    let c = cos(globals.time);
    let s = sin(globals.time);
    let rotated = vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);

    var out: VertexOut;
    out.position = vec4<f32>(rotated.x / globals.aspect, rotated.y, 0.0, 1.0);
    out.color = colors[index];
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

// light.wgsl
// Vertex shader

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct Light {
    proj: mat4x4<f32>,
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
    radius: f32,
    is_spotlight: i32,
    limitcos_inner: f32,
    limitcos_outer: f32,
}
@group(1) @binding(0)
var<storage> lights: array<Light>;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    let scale = 0.25;
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position * scale + lights[0].position, 1.0);
    out.color = lights[0].color;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

 

 
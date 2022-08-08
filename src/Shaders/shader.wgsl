// Vertex shader

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: Camera;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(2) @binding(0)
var<uniform> light: Light;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
}
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.world_normal = normal_matrix * model.normal;
    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = camera.view_proj * world_position;
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var t_normal: texture_2d<f32>;
@group(0) @binding(3)
var s_normal: sampler;
@group(0) @binding(4)
var depth_texture: texture_2d<f32>;

struct MaterialUniform {
    @location(0) use_texture: i32,
    @location(1) u_ambient: vec3<f32>,
    @location(2) u_diffuse: vec3<f32>,
    @location(3) u_specular: vec3<f32>,
}

@group(0) @binding(4)
var<uniform> materialUniform: MaterialUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var object_color: vec4<f32> = vec4(0.0,0.0,0.0,0.0);
    var object_normal: vec4<f32> = vec4(0.0,0.0,0.0,0.0);
    if (materialUniform.use_texture == 1) {
        object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
        object_normal = textureSample(t_normal, s_normal, in.tex_coords);
    } else {
        object_color = vec4(materialUniform.u_diffuse, 1.0);
        object_normal = vec4(0.0,0.0,0.0,0.0);
    }
    
    var result = vec3(0.0,0.0,0.0);
    result = object_color.xyz;

    var light_hit: f32 = 0.0;

    var final_result = vec4<f32>(result, object_color.a);

    return final_result;
}
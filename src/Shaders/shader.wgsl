// Vertex shader

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
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
    limitdir: vec3<f32>,
}
@group(2) @binding(0)
var<storage> lights: array<Light>;

@group(2) @binding(1)
var<uniform> light_num: i32;

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
    @location(3) full_world_pos: vec4<f32>,
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
    out.full_world_pos = world_position;
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

@group(3)
@binding(0)
var t_shadow: texture_depth_2d_array;
@group(3)
@binding(1)
var sampler_shadow: sampler_comparison;

@group(4) @binding(0)
var<uniform> render_target: i32;

@group(4) @binding(1)
var t_depth: texture_depth_2d;
@group(4) @binding(2)
var s_depth: sampler_comparison;
@group(4) @binding(3)
var shadow_view: texture_depth_2d_array;

fn fetch_shadow(light_id: u32, homogeneous_coords: vec4<f32>) -> f32 {
    if (homogeneous_coords.w <= 0.0) {
        return 1.0;
    }
    // compensate for the Y-flip difference between the NDC and texture coordinates
    let flip_correction = vec2<f32>(0.5, -0.5);
    // compute texture coordinates for shadow lookup
    let proj_correction = 1.0 / homogeneous_coords.w;
    let light_local = homogeneous_coords.xy * flip_correction * proj_correction + vec2<f32>(0.5, 0.5);
    // do the lookup, using HW PCF and comparison
    return textureSampleCompareLevel(t_shadow, sampler_shadow, light_local, i32(light_id), homogeneous_coords.z * proj_correction);
}


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
    
    //result = object_color.xyz; // Static Colors no Shadow
        
    var light_hit: f32 = 0.0;

    for (var i: i32 = 0; i < light_num; i=i+1) {
        var l_position = lights[i].position;
        var l_color = lights[i].color;
        var l_intensity = lights[i].intensity;
        var l_radius = lights[i].radius;
        var l_is_spotlight: f32 = f32(lights[i].is_spotlight);

        var surface_to_light = normalize(l_position - in.world_position);

        var spot_target_check: f32 = dot(surface_to_light, -lights[i].limitdir);

        var in_light = max(1.0 - l_is_spotlight, smoothstep(
                lights[i].limitcos_outer,
                lights[i].limitcos_inner,
                spot_target_check
            ));

        l_radius = max(l_radius, 0.00001);
        
        var shadow = fetch_shadow(u32(i), lights[i].proj * in.full_world_pos);

        var ambient_color = l_color * l_radius / max(l_radius, distance(l_position, in.world_position));
        ambient_color = ambient_color * in_light;
        
        var normal = normalize(in.world_normal);
        var light_dir = normalize(l_position - in.world_position);

        var diffuse_strength = max(dot(normal, light_dir), 0.0);
        var diffuse_color = diffuse_strength * in_light * l_color;

        var view_dir = normalize(camera.view_pos.xyz - in.world_position);
        var half_dir = normalize(view_dir + light_dir);

        var specular_strength = pow(max(dot(normal, half_dir), 0.0), 32.0);
        var specular_color = specular_strength * in_light * l_color;
        
        var lig = l_intensity * (ambient_color + diffuse_color + specular_color) * object_color.xyz;
        if (render_target == 3) {
            result = result + lig;
        } else {
            result = result + lig * shadow;
        }
        
    }
    var final_result = vec4<f32>(result, object_color.a);
    
    if (render_target == 1) {
        final_result = vec4(textureSampleCompare(t_depth, s_depth, in.tex_coords, 0.0));
    } else if (render_target == 2) {
        final_result = vec4(fetch_shadow(u32(0), lights[0].proj * in.full_world_pos));
    }
    
    return final_result;
}
use super::{light, instances::{self, Instance}, model};
use std::{mem, num::NonZeroU32};

pub struct Shadow {
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,

    lights: Vec<light::Light>,
    light_target_views: Vec<Option<wgpu::TextureView>>,
    pub shadow_view: wgpu::TextureView,

    pub ext_bind_group: wgpu::BindGroup,
    pub ext_bind_group_layout: wgpu::BindGroupLayout,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalUniforms {
    proj: [[f32; 4]; 4],
    num_lights: [u32; 4],
}

impl Shadow {
    pub fn new(
        device: &wgpu::Device, 
        shader: &wgpu::ShaderModule,
        lights: Vec<light::Light>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        shadow_width: u32,
        shadow_height: u32,
    ) -> Self {
        let uniform_size = mem::size_of::<GlobalUniforms>() as wgpu::BufferAddress;
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Internal Shadow Bind Group"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, // global
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(uniform_size),
                },
                count: None,
            }],
        });        

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow Pipeline Layout"),
            bind_group_layouts: &[
                &bind_group_layout,
                camera_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: uniform_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
            label: None,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_bake",
                buffers: vertex_layouts,
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: device
                    .features()
                    .contains(wgpu::Features::DEPTH_CLIP_CONTROL),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2, // corresponds to bilinear filtering
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let shadow_size = wgpu::Extent3d {
            width: shadow_width,
            height: shadow_height,
            depth_or_array_layers: (lights.len() + 10) as u32,
        };

        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: shadow_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let pub_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("External Shadow Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        let pub_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("External Shadow Bind Group"),
            layout: &pub_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ]
        });

        let shadow_target_views = (0..lights.len())
            .map(|i| {
                Some(shadow_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Shadow Views"),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: i as u32,
                    array_layer_count: NonZeroU32::new(1),
                    format: None,
                    ..Default::default()
                }))
            })
            .collect::<Vec<_>>();

        Self { 
            bind_group: bind_group, 
            render_pipeline: pipeline, 
            uniform_buf: uniform_buf, 
            lights: lights, 
            light_target_views: shadow_target_views,
            shadow_view,
            ext_bind_group: pub_bind_group,
            ext_bind_group_layout: pub_bind_group_layout,
        }
    }
    pub fn render(
        &mut self, 
        encoder: &mut wgpu::CommandEncoder, 
        light_storage_buf: &wgpu::Buffer,
        instance_buf: &instances::InstanceBuffer,
        instances: &Vec<Instance>,
        model: &model::Model,
        camera_bind_group: &wgpu::BindGroup,
    ) -> bool {
        encoder.push_debug_group("shadow passes");
        for (i, light) in self.lights.iter().enumerate() {
            let light_target_view = &self.light_target_views[i].as_ref().unwrap();
            encoder.push_debug_group(&format!(
                "shadow pass {} (light at position {:?})",
                i, light.position
            ));

            // The light uniform buffer already has the projection,
            // let's just copy it over to the shadow uniform buffer.
            encoder.copy_buffer_to_buffer(
                light_storage_buf,
                (i * mem::size_of::<light::LightRaw>()) as wgpu::BufferAddress,
                &self.uniform_buf,
                0,
                64,
            );

            encoder.insert_debug_marker("render entities");
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: light_target_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                });
                pass.set_pipeline(&self.render_pipeline);
                pass.set_vertex_buffer(1, instance_buf.buffer.slice(..));
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_bind_group(1, camera_bind_group, &[]);

                for mesh in &model.meshes {
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.draw_indexed(0..mesh.num_elements, 0, 0..instances.len() as u32)
                }
            }

            encoder.pop_debug_group();
        }
        encoder.pop_debug_group();
        true
    }
}
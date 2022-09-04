// Local Imports, had to do it this way idk why
pub mod texture;
pub mod instances;
pub mod camera;
pub mod vertex;
pub mod model;
pub mod resources;
pub mod light;
pub mod render_pipeline;
pub mod ui;
pub mod shadow;

use wgpu::util::DeviceExt;
// winit Imports
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    window::Window,
};

use crate::window::model::{Vertex};


// All of the states needed for running the game
struct State {
    // Standard renderer stuff
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    // UI rendering
    ui: ui::UI,
    
    // Render Pipelin
    render_pipeline: wgpu::RenderPipeline,

    // Camera stuff
    camera: camera::Camera,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: camera::CameraController,

    // Instancing
    instances: Vec<instances::Instance>,
    instance_buffer: instances::InstanceBuffer,

    //Depth buffer
    depth_texture: texture::Texture,

    // Model testing stuff
    obj_model: model::Model,
    cube_model: model::Model,

    // Light stuff
    light0: light::Light,
    light_buffer: light::LightBuffer,
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,
    lights_are_dirty: bool,

    // Shadow Stuff
    shadow_config: shadow::Shadow,

    // Render Overlay stuff
    render_texture_bind_group: wgpu::BindGroup,
    render_target_buffer: wgpu::Buffer,
}

fn features() -> wgpu::Features {
    wgpu::Features::DEPTH_CLIP_CONTROL |
    wgpu::Features::MULTIVIEW
}


impl State {

    // After creating a window, initializing wgpu and other stuff for tha rendering
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let hidpi_factor = window.scale_factor();

        // Backends:all => Vulkan + Metal + DX12
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: features(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits {
                        max_bind_groups: 8,
                        ..Default::default()
                    }
                },
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let ui = ui::UI::new(&window, hidpi_factor, &device, &queue, &config);

        let camera = camera::Camera::new((10.0,5.0,10.0).into(), 45.0, config.width as f32 / config.height as f32, 45.0);

        let (camera_uniform, camera_buffer, camera_bind_group_layout, camera_bind_group) = camera.create_camera_buffers_and_uniform(&device);

        let camera_controller = camera::CameraController::new(0.2, 2.0);

        let light0 = light::Light::new(0, [2.0, 2.1, 2.0].into(), [1.0, 1.0, 1.0].into(), 1.0, 1.0);

        let lights_vec = vec![light0];
        
        let light_buffer = light::LightBuffer::new(&device, &lights_vec);

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Buffer { 
                        ty: wgpu::BufferBindingType::Uniform, 
                        has_dynamic_offset: false, 
                        min_binding_size: None 
                    },
                    count: None,
                }],
                label: Some("Light Bind group layout"),
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: light_buffer.light_num_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let shadow_config = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Shadow Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../Shaders/shadow.wgsl").into()),
            };
            let shader = device.create_shader_module(shader);
            shadow::Shadow::new(&device, &shader, lights_vec, &[model::ModelVertex::desc(), instances::InstanceRaw::desc()], 8192, 8192)
        };
        
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry { // Standard diffuse Texture
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry { // Standard Diffuse Sampler
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry { // Standard Normal texture
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry { // standard Normal Sampler
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry { // MaterialUniform
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }                    
                ],
                label: Some("texture_bind_group_layout"),
        });
        
        let render_textures_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Depth and Shadow Texture BindGroup"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { // Standard Depth Texture
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Depth,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { // Standard Depth Sampler
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &texture_bind_group_layout,
                &camera_bind_group_layout,
                &light_bind_group_layout,
                &shadow_config.ext_bind_group_layout,
                &render_textures_bind_layout,
            ],
            push_constant_ranges: &[],
        });

        
        let render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../Shaders/shader.wgsl").into()),
            };
            render_pipeline::create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc(), instances::InstanceRaw::desc()],
                shader,
            )
        };
        
        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Light Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../Shaders/light.wgsl").into()),
            };
            render_pipeline::create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc()],
                shader,
            )
        };

        let obj_model = resources::load_model(
            "Models1/test.obj",
            &device,
            &queue,
            &texture_bind_group_layout,
        ).await.unwrap();       

        let cube_model = resources::load_model(
            "Models/cube.obj",
            &device,
            &queue,
            &texture_bind_group_layout,
        ).await.unwrap();    

        let instance_vec = vec![instances::Instance {
            position: cgmath::Vector3::new(0.0,0.0,0.0),
            rotation: cgmath::Quaternion::new(0.0,0.0,0.0,0.0),
        }];

        let instance_buffer = instances::InstanceBuffer::new(&device, &instance_vec);

        let render_target_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Render target num"),
            contents: bytemuck::cast_slice(&[ui.render_target as i32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let render_textures_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render texture Bind group"),
            layout: &render_textures_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(render_target_buffer.as_entire_buffer_binding())
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&depth_texture.sampler),
                }
            ]
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            ui,
            render_pipeline,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            instances: instance_vec,
            instance_buffer: instance_buffer,
            depth_texture,
            obj_model,
            cube_model,
            light0,
            light_buffer,
            light_bind_group,
            light_render_pipeline,
            lights_are_dirty: true,
            shadow_config,
            render_texture_bind_group: render_textures_bind_group,
            render_target_buffer
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.camera.resize(new_size.width, new_size.height);
        }
    }

    fn input(&mut self, event: &WindowEvent, window: &Window) -> bool {
        if self.camera_controller.process_event(event, window) {return true}
        false
    }
    fn mouse_input(&mut self, event: &DeviceEvent) {
        self.camera_controller.process_mouse_event(event);
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
        self.queue.write_buffer(&self.render_target_buffer, 0, bytemuck::cast_slice(&[self.ui.render_target as i32]));
        // let old_position: cgmath::Vector3<_> = self.light0.position.into();
        // self.light0.position = (cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(1.0)) * old_position).into();
        // self.light_buffer.repopulate_lights(&self.queue, &vec![self.light0]);
        // self.shadow_config.update_lights(vec![self.light0]);
    }

    fn render(&mut self, window: &Window) -> Result<(), wgpu::SurfaceError> {
        if self.lights_are_dirty {
            self.lights_are_dirty = false;
            self.light_buffer.repopulate_lights(&self.queue, &vec![self.light0])
        }
        
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        self.shadow_config.render(&mut encoder, &self.instance_buffer, &self.instances, &self.obj_model, &self.queue);
        {   
             
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            use model::DrawLight;
            render_pass.set_vertex_buffer(1, self.instance_buffer.buffer.slice(..));
            render_pass.set_pipeline(&&self.light_render_pipeline);
            render_pass.draw_light_model(&self.cube_model, &self.camera_bind_group, &self.light_bind_group);
            
            use model::DrawModel;
            render_pass.set_pipeline(&&self.render_pipeline);
            render_pass.set_bind_group(3, &self.shadow_config.ext_bind_group, &[]);
            render_pass.set_bind_group(4, &self.render_texture_bind_group, &[]);
            render_pass.draw_model_instanced(&self.obj_model, 0..self.instances.len() as u32, &self.camera_bind_group, &self.light_bind_group);
            
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        self.ui.draw(window, &self.device, &self.queue, &view);
        output.present();

        Ok(())
    }
}

 


pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                state.update();
                match state.render(&window) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => if !state.input(event, &window) {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            },
            Event::DeviceEvent { ref event, .. } => state.mouse_input(&event),
            _ => {}
        }
        state.ui.handle_input(&window, &event);
    });
        
}

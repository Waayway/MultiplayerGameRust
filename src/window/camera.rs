use std::time::Instant;

use cgmath::{InnerSpace, Vector3};
use cgmath_dolly::prelude::*;
use wgpu::util::DeviceExt;
use winit::{event::{WindowEvent, ElementState, VirtualKeyCode, KeyboardInput, DeviceEvent}, window::Window};

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub rot: (f32, f32),
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);


impl Camera {
    pub fn new(pos: cgmath::Vector3<f32>, rotation: f32, aspect: f32, fovy: f32) -> Self {
        let mut target = pos;
        let rotation = rotation.to_radians();
        target.x -= rotation.sin();
        target.z -= rotation.cos();
        Self {
            aspect: aspect,
            fovy: 45.0,
            zfar: 100.0,
            znear: 0.1,
            eye: cgmath::Point3::new(pos.x,pos.y,pos.z),
            rot: (rotation, 0.0),
            target: cgmath::Point3::new(target.x,target.y,target.z),
            up: cgmath::Vector3::unit_y(),
        }
    }
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);

        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        return OPENGL_TO_WGPU_MATRIX * proj * view
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }
    pub fn create_camera_buffers_and_uniform(&self, device: &wgpu::Device) -> (CameraUniform, wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&self);
        
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: None,
        });
        
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });
        return (camera_uniform, camera_buffer, camera_bind_group_layout, camera_bind_group);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_position: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_position: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = camera.eye.to_homogeneous().into();
        self.view_proj = (OPENGL_TO_WGPU_MATRIX * camera.build_view_projection_matrix()).into();
    }
}

pub struct CameraController {
    camera_rig: CameraRig,
    mouse_speed: f32,
    is_forward_pressed: bool,
    is_backwards_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    mouse_delta: (f64, f64),
    mouse_event: bool,
    last_frame: Instant,
} 
impl CameraController {
    pub fn new(speed: f32, mouse_speed: f32) -> Self {
        let mut camera: CameraRig = CameraRig::builder()
            .with(YawPitch::new().yaw_degrees(45.0).pitch_degrees(-30.0))
            .with(Smooth::new_rotation(1.5))
            .with(Arm::new(Vector3::unit_z() * 20.0))
            .build();

        Self { 
            camera_rig: camera,
            mouse_speed: mouse_speed,
            is_forward_pressed: false,
            is_backwards_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            mouse_delta: (0.0,0.0),
            mouse_event: false,
            last_frame: Instant::now(),
        }
    }

    pub fn process_event(&mut self, event: &WindowEvent, window: &Window) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state,
                    virtual_keycode: Some(keycode),
                    ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.is_backwards_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn process_mouse_event(&mut self, event: &DeviceEvent) -> bool {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.mouse_delta = *delta;
                self.mouse_event = true;
                true
            },
            _ => {false}
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        let delta_s = self.last_frame.elapsed();
        self.last_frame = Instant::now();
        

        let camera_driver = self.camera_rig.driver_mut::<YawPitch>();
        if self.mouse_event {
            self.mouse_event = false;
            let mut delta: cgmath::Vector2<f64> = self.mouse_delta.into();
            camera_driver.rotate_yaw_pitch(delta.x as f32, delta.y as f32);
        }
        
        if self.is_forward_pressed {
            
        }
        if self.is_backwards_pressed {
            
        }
        if self.is_left_pressed {
            
        }
        if self.is_right_pressed {
            
        }
        let camera_transform = self.camera_rig.update(delta_s.as_secs_f32());

        camera.eye = cgmath::Point3::new(camera_transform.position.x, camera_transform.position.y, camera_transform.position.z);
        let target = camera_transform.position + camera_transform.forward();
        camera.target = cgmath::Point3::new(target.x, target.y, target.z);
    }
}
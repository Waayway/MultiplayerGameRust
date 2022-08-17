use std::{mem, f32::consts};

use wgpu::util::DeviceExt;

use super::camera;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Light {
    pub id: usize,
    pub position: cgmath::Vector3<f32>,
    pub color: cgmath::Vector3<f32>,
    pub intensity: f32,
    pub radius: f32,
    pub is_spotlight: bool,
    pub limitcos_inner: f32,
    pub limitcos_outer: f32,
    pub limitdir: cgmath::Vector3<f32>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightRaw {
    pub proj: [[f32; 4]; 4],
    pub position: [f32; 3],
    pub _padding: u32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub is_spotlight: u32,
    pub limitcos_inner: f32,
    pub limitcos_outer: f32,
    pub limitdir: [f32; 3],
    pub _padding1: u32,
}

pub struct LightBuffer {
    pub buffer: wgpu::Buffer,
    pub light_num_buffer: wgpu::Buffer,
}

impl Light {
    pub fn new(
        id: usize,
        position: cgmath::Vector3<f32>,
        color: cgmath::Vector3<f32>,
        intensity: f32,
        radius: f32,
    ) -> Self {
        Self {
            id,
            position,
            color,
            intensity,
            radius,
            is_spotlight: false,
            limitcos_inner: 0.9,
            limitcos_outer: 1.0,
            limitdir: (0.0, 0.0, 0.0).into(),
        }
    }
    pub fn to_raw(&self) -> LightRaw {
        let pos = cgmath::point3(self.position.x, self.position.y, self.position.z);
        let view = cgmath::Matrix4::look_at_rh(
            pos,
            cgmath::Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            cgmath::Vector3::new(0.0, 0.0, 1.0),
        );
        let projection = cgmath::perspective(cgmath::Deg(160.), 1.0, 0.1, 100.0);
        let view_proj = projection * view;
        let view_proj: [[f32;4]; 4] = [
            view_proj.x.into(),
            view_proj.y.into(),
            view_proj.z.into(),
            view_proj.w.into(),
        ];
        LightRaw {
            position: self.position.into(),
            _padding: 0,
            color: self.color.into(),
            intensity: self.intensity,
            radius: self.radius,
            is_spotlight: if self.is_spotlight { 1 } else { 0 },
            limitcos_inner: self.limitcos_inner,
            limitcos_outer: self.limitcos_outer,
            limitdir: self.limitdir.into(),
            proj: view_proj.into(),
            _padding1: 0,
        }
    }
}

impl LightBuffer {
    pub fn new(device: &wgpu::Device, lights: &Vec<Light>) -> Self {
        let light_raws = lights
            .iter()
            .map(|light| light.to_raw())
            .collect::<Vec<_>>();
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&light_raws),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });
        let light_num_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: &[light_raws.len() as u8],
            usage: wgpu::BufferUsages::UNIFORM,
        });
        Self {
            buffer: light_buffer,
            light_num_buffer: light_num_buffer,
        }
    }
    pub fn repopulate_lights(&mut self, queue: &wgpu::Queue, lights: &Vec<Light>) {
        for (i, light) in lights.iter().enumerate() {
            queue.write_buffer(
                &self.buffer,
                (i * mem::size_of::<LightRaw>()) as wgpu::BufferAddress,
                bytemuck::bytes_of(&light.to_raw()),
            );
        }
    }
}

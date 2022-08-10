use wgpu::util::DeviceExt;

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
    pub position: [f32; 3],
    pub _padding: u32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub is_spotlight: u32,
    pub limitcos_inner: f32,
    pub limitcos_outer: f32,
    pub limitdir: [f32; 3],
    pub _padding2: u32,
}

pub struct LightBuffer {
    pub buffer: wgpu::Buffer,
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
            _padding2: 0,
        }
    }
}
impl LightBuffer {
    pub fn new(device: &wgpu::Device, lights: &Vec<Light>) -> Self{
        let light_raws = lights.iter().map(|light| light.to_raw()).collect::<Vec<_>>();
        let light_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&light_raws),
                usage: wgpu::BufferUsages::UNIFORM,
            }
        );
        Self { buffer: light_buffer }
    }
}
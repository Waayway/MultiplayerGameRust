#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub id: usize,
    pub position: [f32; 3],
    pub _padding: u32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub is_spotlight: u32,
    pub limitcos_inner: f32,
    pub limitcos_outer: f32,
}
impl LightUniform {
    pub fn new(
        id: usize,
        position: [f32; 3],
        color: [f32; 3],
        intensity: f32,
        radius: f32,
    ) -> Self {
        Self {
            id,
            position,
            color,
            intensity,
            radius,
            is_spotlight: 0,
            limitcos_inner: 0.9,
            limitcos_outer: 1.0,
            _padding: 0,
        }
    }
}
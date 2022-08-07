use std::time::{Instant};
use imgui_wgpu::Renderer;
use winit::{
    event::{Event},
    window::Window,
};


pub struct UI {
    imgui: imgui::Context,
    imgui_platform: imgui_winit_support::WinitPlatform,
    renderer: Renderer,
    demo_open: bool,
    last_frame: Instant,
    last_cursor: Option<imgui::MouseCursor>,
}

impl UI {
    pub fn new(window: &Window, hidpi_factor: f64, device: &wgpu::Device, queue: &wgpu::Queue, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        let mut imgui = imgui::Context::create();
        let mut imgui_platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        imgui_platform.attach_window(
            imgui.io_mut(), 
            window, 
            imgui_winit_support::HiDpiMode::Default,
        );
        imgui.set_ini_filename(None);

        let font_size = (13.0 * hidpi_factor) as f32;

        imgui.fonts().add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);
   
        let renderer_config = imgui_wgpu::RendererConfig {
            texture_format: surface_config.format,
            ..Default::default()
        };
    
        let renderer = Renderer::new(&mut imgui, &device, &queue, renderer_config);
    
        let last_frame = Instant::now();
        let demo_open = true;
    
        let last_cursor: Option<imgui::MouseCursor> = None;

        Self { 
            imgui, 
            imgui_platform,
            renderer,
            demo_open,
            last_frame,
            last_cursor,
        }
    }
    pub fn draw(&mut self, window: &Window ,device: &wgpu::Device, queue: &wgpu::Queue, surface_view: &wgpu::TextureView) {
        let delta_s = self.last_frame.elapsed();
        let now = Instant::now();
        self.imgui.io_mut().update_delta_time(now - self.last_frame);
        self.last_frame = now;

        self.imgui_platform.prepare_frame(self.imgui.io_mut(), &window).expect("Failed to prepare frame");
        let ui = self.imgui.frame();
        {
            let window = imgui::Window::new("Information");
            window
                .size([300.0, 100.0], imgui::Condition::FirstUseEver)
                .build(&ui, || {
                    let mouse_pos = ui.io().mouse_pos;
                    let fps = (1000.0 / delta_s.as_millis() as f32).round() as i32;
                    ui.text(format!("Mouse Position: ({:.1},{:.1})", mouse_pos[0], mouse_pos[1]));
                    ui.text(format!("FPS: {:?}", fps));
                    ui.text(format!("Frametime: {:?}", delta_s));
                });
        }

        let mut encoder: wgpu::CommandEncoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Imgui Encoder"), 
        });

        if self.last_cursor != ui.mouse_cursor() {
            self.last_cursor = ui.mouse_cursor();
            self.imgui_platform.prepare_render(&ui, &window);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            self.renderer
                    .render(ui.render(), &queue, &device, &mut render_pass)
                    .expect("Rendering failed");
        }

        queue.submit(Some(encoder.finish()));
    }
    pub fn handle_input<T>(&mut self, window: &Window, event: &Event<T>) -> bool{
        self.imgui_platform.handle_event(self.imgui.io_mut(), window, event);
        return true;
    }
}
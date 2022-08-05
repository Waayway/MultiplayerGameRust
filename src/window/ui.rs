use std::time::{Instant, Duration};
use imgui_wgpu::Renderer;
use winit::window::Window;


pub struct UI {
    imgui: imgui::Context,
    imgui_platform: imgui_winit_support::WinitPlatform,
    renderer: Renderer,
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
        imgui.io_mut().font_global_scale = (1.0 / font_size) as f32;

        imgui.fonts().add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);
        
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };
    
        let renderer_config = imgui_wgpu::RendererConfig {
            texture_format: surface_config.format,
            ..Default::default()
        };
    
        let mut renderer = Renderer::new(&mut imgui, &device, &queue, renderer_config);
    
        let mut last_frame = Instant::now();
        let mut demo_open = true;
    
        let mut last_cursor: Option<imgui::MouseCursor> = None;

        Self { 
            imgui, 
            imgui_platform,
            renderer,
            last_frame,
            last_cursor,
        }
    }
    pub fn draw<'a>(&'a mut self, window: &Window ,device: &wgpu::Device, queue: &wgpu::Queue, surface: &wgpu::Surface, render_pass: &mut wgpu::RenderPass<'a>) -> &'a bool {
        let delta_s = self.last_frame.elapsed();
        let now = Instant::now();
        self.imgui.io_mut().update_delta_time(now - self.last_frame);
        self.last_frame = now;

        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("dropped frame: {:?}", e);
                return &false;
            }
        };
        self.imgui_platform.prepare_frame(self.imgui.io_mut(), &window).expect("Failed to prepare frame");
        let ui = self.imgui.frame();
        self.build_windows(&ui, delta_s, true);

        if self.last_cursor != ui.mouse_cursor() {
            self.last_cursor = ui.mouse_cursor();
            self.imgui_platform.prepare_render(&ui, &window);
        }
        self.renderer
                    .render(ui.render(), &queue, &device, render_pass)
                    .expect("Rendering failed");
                
        return &true;
    }
    
    fn build_windows(&mut self, ui: &imgui::Ui, delta_s: Duration, mut demo_open: bool) {
        let window = imgui::Window::new("Hello world");
        window
            .size([300.0, 100.0], imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.text("Hello world!");
                ui.text("This...is...imgui-rs on WGPU!");
                ui.separator();
                let mouse_pos = ui.io().mouse_pos;
                ui.text(format!(
                    "Mouse Position: ({:.1},{:.1})",
                    mouse_pos[0], mouse_pos[1]
                ));
            });
        let window = imgui::Window::new("Hello too");
        window
            .size([400.0, 200.0], imgui::Condition::FirstUseEver)
            .position([400.0, 200.0], imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.text(format!("Frametime: {:?}", delta_s));
            });

        ui.show_demo_window(&mut demo_open);
    }
}
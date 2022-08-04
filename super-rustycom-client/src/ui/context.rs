use imgui::{FontConfig, FontSource};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};

use super::UI;
use crate::expect;

pub struct Context {
    pub ui: UI,
    pub context: imgui::Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
}

impl Context {
    pub fn new(display: &glium::Display) -> Self {
        let mut context = imgui::Context::create();

        context.set_ini_filename(None);

        let mut platform = WinitPlatform::init(&mut context);

        // This is where highdpi would go, but we always use physical size
        let font_size = 13.0 as f32;
        context.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                size_pixels: font_size,
                ..FontConfig::default()
            }),
        }]);
        context.io_mut().font_global_scale = 1.0;

        {
            let style = context.style_mut();
            // Do rectangular elements
            style.window_rounding = 0.0;
            style.child_rounding = 0.0;
            style.popup_rounding = 0.0;
            style.grab_rounding = 0.0;
            style.tab_rounding = 0.0;
            style.frame_rounding = 0.0;
            style.scrollbar_rounding = 0.0;
            // No border line
            style.window_border_size = 0.0;
        }

        let renderer = expect!(
            Renderer::init(&mut context, display),
            "Failed to initialize renderer"
        );

        platform.attach_window(
            context.io_mut(),
            display.gl_window().window(),
            // This with the font settings appears to force the scaling off
            HiDpiMode::Rounded,
        );

        Self {
            ui: UI::default(),
            context,
            platform,
            renderer,
        }
    }
}

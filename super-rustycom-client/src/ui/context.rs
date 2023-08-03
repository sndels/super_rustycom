use egui::FontFamily::Proportional;
use egui::FontId;
use egui::TextStyle::*;
use egui_glium;

use super::Ui;

pub struct Context {
    pub ui: Ui,
    pub egui_glium: egui_glium::EguiGlium,
}

impl Context {
    pub fn new(display: &glium::Display) -> Self {
        let mut egui_glium = egui_glium::EguiGlium::new(display);

        let ctx = &egui_glium.egui_ctx;

        let mut style = (*ctx.style()).clone();

        style.text_styles = [
            (Heading, FontId::new(13.0, Proportional)),
            (Name("Heading2".into()), FontId::new(13.0, Proportional)),
            (Name("Context".into()), FontId::new(13.0, Proportional)),
            (Body, FontId::new(13.0, Proportional)),
            (Monospace, FontId::new(13.0, Proportional)),
            (Button, FontId::new(13.0, Proportional)),
            (Small, FontId::new(13.0, Proportional)),
        ]
        .into();

        // Mutate global style with above changes
        ctx.set_style(style);

        // context.set_ini_filename(None);

        // let mut platform = WinitPlatform::init(&mut context);

        // // This is where highdpi would go, but we always use physical size
        // let font_size = 13.0;
        // context.fonts().add_font(&[FontSource::DefaultFontData {
        //     config: Some(FontConfig {
        //         size_pixels: font_size,
        //         ..FontConfig::default()
        //     }),
        // }]);
        // context.io_mut().font_global_scale = 1.0;

        // {
        //     let style = context.style_mut();
        //     // Do rectangular elements
        //     style.window_rounding = 0.0;
        //     style.child_rounding = 0.0;
        //     style.popup_rounding = 0.0;
        //     style.grab_rounding = 0.0;
        //     style.tab_rounding = 0.0;
        //     style.frame_rounding = 0.0;
        //     style.scrollbar_rounding = 0.0;
        //     // No border line
        //     style.window_border_size = 0.0;
        // }

        // let mut renderer = expect!(
        //     Renderer::init(&mut context, display),
        //     "Failed to initialize renderer"
        // );

        // platform.attach_window(
        //     context.io_mut(),
        //     display.gl_window().window(),
        //     // This with the font settings appears to force the scaling off
        //     HiDpiMode::Rounded,
        // );

        Self {
            ui: Ui::new(),
            egui_glium,
        }
    }
}

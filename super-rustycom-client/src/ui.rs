use crate::config::Config;
use crate::debugger::{disassemble_current, status_str};
use crate::draw_data::DrawData;
use crate::framebuffer::Framebuffer;
use crate::text::TextRenderer;
use std::time::Instant;
use super_rustycom_core::snes::SNES;

pub struct UI {
    text_renderer: TextRenderer,
    fb: Framebuffer,
}

impl UI {
    pub fn new(config: &Config) -> UI {
        UI {
            text_renderer: TextRenderer::new(),
            fb: Framebuffer::new(&config),
        }
    }

    pub fn buffer(&self) -> &Vec<u32> {
        self.fb.buffer()
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.fb.resize(width, height)
    }

    pub fn draw(&mut self, data: &DrawData, snes: &mut SNES, config: &Config) {
        let t_debug_draw = Instant::now();

        self.fb.clear(0x00000000);

        self.draw_history(data, snes, config);
        self.draw_cpu(snes, config);

        let debug_draw_millis = t_debug_draw.elapsed().as_nanos() as f32 * 1e-6;

        self.draw_perf(data, debug_draw_millis, config);
    }

    fn draw_history(&mut self, data: &DrawData, snes: &mut SNES, config: &Config) {
        let current_disassembly = [[
            String::from("> "),
            disassemble_current(&snes.cpu, &mut snes.abus),
        ]
        .join("")];
        let disassembly_iter = data.disassembled_history.iter().chain(&current_disassembly);

        self.text_renderer.draw(
            disassembly_iter,
            0xFFFFFFFF,
            self.fb.window(
                2,
                2,
                config.resolution.width - 2 - 1,
                config.resolution.height - 2 - 1,
            ),
        );
    }

    fn draw_cpu(&mut self, snes: &SNES, config: &Config) {
        self.text_renderer.draw(
            &status_str(&snes.cpu),
            0xFFFFFFFF,
            self.fb.window(config.resolution.width - 79, 2, 79, 85),
        );
    }

    fn draw_perf(&mut self, data: &DrawData, debug_draw_millis: f32, config: &Config) {
        {
            let row_pos = config.resolution.height - 2 * self.text_renderer.line_height() - 1;
            self.text_renderer.draw(
                &[format!("Debug draw took {:.2}ms!", debug_draw_millis)],
                0xFFFFFFFF,
                self.fb.window(
                    2,
                    row_pos,
                    config.resolution.width,
                    self.text_renderer.line_height(),
                ),
            );
        }

        {
            let row_pos = config.resolution.height - self.text_renderer.line_height() - 1;
            let (message, color) = if data.missing_nanos > 0 {
                (
                    format!("Lagged {:2}ms behind!", data.missing_nanos as f32 * 1e-6),
                    0xFFFF0000,
                )
            } else {
                (
                    format!(
                        "Emulation is {:.2}ms ahead!",
                        data.extra_nanos as f32 * 1e-6
                    ),
                    0xFFFFFFFF,
                )
            };
            self.text_renderer.draw(
                &[message],
                color,
                self.fb.window(
                    2,
                    row_pos,
                    config.resolution.width,
                    self.text_renderer.line_height(),
                ),
            );
        }
    }
}

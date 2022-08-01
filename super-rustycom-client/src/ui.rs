use crate::config::Config;
use crate::debugger::{cpu_status_str, disassemble_current, smp_status_str};
use crate::draw_data::DrawData;
use crate::framebuffer::Framebuffer;
use crate::text::TextRenderer;
use itertools::Itertools;
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
            fb: Framebuffer::new(config),
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

        self.draw_history(data, snes);
        self.draw_mem(snes);
        self.draw_cpu(snes, config);
        self.draw_smp(snes, config);

        let debug_draw_millis = t_debug_draw.elapsed().as_nanos() as f32 * 1e-6;

        self.draw_perf(data, debug_draw_millis, config);
    }

    fn draw_history(&mut self, data: &DrawData, snes: &mut SNES) {
        // Top left corner
        let window = self.fb.relative_window(0, 0, 80, 40);
        let history_len = data.disassembled_history().len();
        let window_lines = window.len() / self.text_renderer.line_height();
        let disassembly = data
            .disassembled_history()
            .iter()
            // Drop history that doesn't fit while leaving room for current pointer
            .skip(history_len.saturating_sub(window_lines) + 1)
            .cloned()
            .chain(
                [format!(
                    "> {}",
                    disassemble_current(&snes.cpu, &mut snes.abus)
                )]
                .iter()
                .cloned(),
            )
            .collect::<Vec<String>>();
        self.text_renderer.draw(&disassembly, 0xFFFFFFFF, window);
    }

    fn draw_mem(&mut self, snes: &mut SNES) {
        // Below instruction history
        let window = self.fb.relative_window(0, 40, 80, 40);
        let window_lines = window.len() / self.text_renderer.line_height();

        let start_byte = 0x0000;
        // Drop one line since we have the column header
        let end_byte = start_byte + window_lines.saturating_sub(1) * 0x0010;

        let wram = snes.apu.bus.ram();
        let wram_text = {
            let mut wram_text = vec![String::from(
                "      00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F",
            )];
            wram_text.extend(
                wram[start_byte..end_byte]
                    .chunks(0x10)
                    .into_iter()
                    // Zip line addrs with lines
                    .zip((start_byte..wram.len()).step_by(0x0010))
                    // Create line string with space between bytes
                    .map(|(line, addr)| {
                        format!(
                            "${:04X} {}",
                            addr,
                            line.iter()
                                .format_with(" ", |elt, f| f(&format_args!("{:02X}", elt)))
                        )
                    })
                    .collect_vec(),
            );
            wram_text
        };

        self.text_renderer.draw(&wram_text, 0xFFFFFFFF, window);
    }

    fn draw_cpu(&mut self, snes: &SNES, config: &Config) {
        let text = cpu_status_str(&snes.cpu);
        let (w, h) = self.text_renderer.window_size(text.iter());
        let start_x = config.resolution.width.saturating_sub(w);
        // Top right corner
        let window = self.fb.absolute_window(start_x, 0, w, h);
        self.text_renderer.draw(&text, 0xFFFFFFFF, window);
    }

    fn draw_smp(&mut self, snes: &SNES, config: &Config) {
        let text = smp_status_str(&snes.apu.smp);
        let (w, h) = self.text_renderer.window_size(text.iter());
        let start_x = config.resolution.width.saturating_sub(w);
        let start_y = config.resolution.height.saturating_sub(h);
        // Top right corner
        let window = self.fb.absolute_window(start_x, start_y, w, h);
        self.text_renderer.draw(&text, 0xFFFFFFFF, window);
    }

    fn draw_perf(&mut self, data: &DrawData, debug_draw_millis: f32, config: &Config) {
        {
            let start_y = config
                .resolution
                .height
                .saturating_sub(2 * self.text_renderer.line_height());
            // Bottom left corner above ahead/lag count
            let window = self.fb.absolute_window(
                0,
                start_y,
                config.resolution.width,
                self.text_renderer.line_height(),
            );
            self.text_renderer.draw(
                &[format!("Debug draw took {:>5.2}ms!", debug_draw_millis)],
                0xFFFFFFFF,
                window,
            );
        }

        {
            let start_y = config
                .resolution
                .height
                .saturating_sub(self.text_renderer.line_height());
            // Bottom left corner
            let window = self.fb.absolute_window(
                0,
                start_y,
                config.resolution.width,
                self.text_renderer.line_height(),
            );
            let (message, color) = if data.missing_nanos > 0 {
                (
                    format!("Lagged {:>5.2}ms behind!", data.missing_nanos as f32 * 1e-6),
                    0xFFFF0000,
                )
            } else {
                (
                    format!(
                        "Emulation is {:>5.2}ms ahead!",
                        data.extra_nanos as f32 * 1e-6
                    ),
                    0xFFFFFFFF,
                )
            };
            self.text_renderer.draw(&[message], color, window);
        }
    }
}

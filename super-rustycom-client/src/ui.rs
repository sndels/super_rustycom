use glium::glutin;
use imgui::{Context, FontConfig, FontSource};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use itertools::Itertools;
use std::time::{Duration, Instant};
use super_rustycom_core::snes::SNES;

use crate::{
    debugger::{
        cpu_status_str, disassemble_current, disassemble_peek, smp_status_str, DebugState, Debugger,
    },
    draw_data::DrawData,
    expect,
};

pub struct UI {
    context: Context,
    platform: WinitPlatform,
    renderer: Renderer,
}

#[derive(Default)]
pub struct UIState {
    pub is_any_item_active: bool,
}

impl UI {
    pub fn new(display: &glium::Display) -> Self {
        let mut context = imgui::Context::create();
        context.set_ini_filename(None);

        let mut platform = WinitPlatform::init(&mut context);

        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        context.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                size_pixels: font_size,
                ..FontConfig::default()
            }),
        }]);

        context.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

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
            HiDpiMode::Rounded,
        );

        UI {
            context,
            platform,
            renderer,
        }
    }

    pub fn handle_event<'b, T: 'static>(
        &mut self,
        window: &glutin::window::Window,
        event: &glutin::event::Event<'b, T>,
    ) {
        self.platform
            .handle_event(self.context.io_mut(), window, event);
    }

    pub fn update_delta_time(&mut self, delta: Duration) {
        self.context.io_mut().update_delta_time(delta);
    }

    pub fn render(
        &mut self,
        window: &glutin::window::Window,
        render_target: &mut glium::Frame,
        data: &mut DrawData,
        snes: &mut SNES,
        debugger: &mut Debugger,
    ) -> UIState {
        let ui_start = Instant::now();

        expect!(
            self.platform.prepare_frame(self.context.io_mut(), window),
            "Failed to prepare imgui gl frame"
        );
        let resolution = window.inner_size();

        let mut state = UIState::default();

        {
            let frame_ui = self.context.frame();

            disassembly_window(frame_ui, data, snes, debugger);
            mem_window(frame_ui, snes);
            cpu_window(frame_ui, &resolution, snes);
            smp_window(frame_ui, &resolution, snes);

            let ui_millis = ui_start.elapsed().as_nanos() as f32 * 1e-6;

            perf_window(frame_ui, &resolution, data, ui_millis);

            state.is_any_item_active = frame_ui.is_any_item_active();

            self.platform.prepare_render(frame_ui, window);
        }

        expect!(
            self.renderer.render(render_target, self.context.render()),
            "Rendering GL window failed"
        );

        state
    }
}

const DISASSEMBLY_WINDOW_SIZE: [f32; 2] = [360.0, 424.0];
const DISASSEMBLY_CHILD_WINDOW_SIZE: [f32; 2] = [DISASSEMBLY_WINDOW_SIZE[0] - 10.0, 320.0];
static mut DISASSEMBLY_SCROLL_TO_CURRENT: bool = true;
static mut DISASSEMBLY_STEPS: i32 = 1;

fn disassembly_window(
    ui: &mut imgui::Ui,
    data: &mut DrawData,
    snes: &mut SNES,
    debugger: &mut Debugger,
) {
    ui.window("Execution")
        .position([0.0, 0.0], imgui::Condition::Appearing)
        .size(DISASSEMBLY_WINDOW_SIZE, imgui::Condition::Appearing)
        .resizable(false)
        .collapsible(false)
        .build(|| {
            ui.child_window("Disassembly")
                .size(DISASSEMBLY_CHILD_WINDOW_SIZE)
                .scroll_bar(true)
                .build(|| {
                    for row in data.disassembled_history() {
                        ui.text(format!("  {}", row));
                    }

                    let (current_str, current_size) =
                        disassemble_current(&snes.cpu, &mut snes.abus);
                    ui.text(format!("> {}", current_str));

                    unsafe {
                        if DISASSEMBLY_SCROLL_TO_CURRENT {
                            ui.set_scroll_here_y();
                        }
                    }

                    let mut peek_offset = current_size;
                    for _ in 0..20 {
                        let (disassembled, next_size) =
                            disassemble_peek(&snes.cpu, &mut snes.abus, peek_offset);
                        ui.text(disassembled);
                        peek_offset += next_size;
                    }
                });

            if ui.button("Run") {
                debugger.state = DebugState::Run;
            }

            ui.same_line();
            let should_step = ui.button("Step");

            ui.same_line();
            let steps = unsafe {
                let _width = ui.push_item_width(40.0);
                let _ = imgui::Drag::new("##Steps")
                    .range(1, 1000)
                    .build(ui, &mut DISASSEMBLY_STEPS);
                DISASSEMBLY_STEPS
            };

            if should_step {
                debugger.state = DebugState::Step;
                debugger.steps = steps as u32;
            }

            ui.same_line();
            if ui.button("Reset") {
                snes.cpu.reset(&mut snes.abus);
                data.clear_history();
                debugger.state = DebugState::Active;
            }

            {
                let _width = ui.push_item_width(106.0);
                let mut bp = debugger.breakpoint as i32;
                // TODO: Remove +-
                let _ = ui
                    .input_int("Breakpoint", &mut bp)
                    .chars_hexadecimal(true)
                    .display_format("$%06X")
                    .build();
                debugger.breakpoint = bp.max(0).min(0xFFFFFF) as u32;
            }

            unsafe {
                ui.checkbox("Scroll to current", &mut DISASSEMBLY_SCROLL_TO_CURRENT);
            }
        });
}

const MEMORY_WINDOW_SIZE: [f32; 2] = [388.0, 372.0];

fn mem_window(ui: &mut imgui::Ui, snes: &mut SNES) {
    let start_byte = 0x0000;
    let shown_row_count: usize = 20;
    // Drop one line since we have the column header
    let end_byte = start_byte + shown_row_count.saturating_sub(1) * 0x0010;

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

    ui.window("Memory")
        .position(
            [DISASSEMBLY_WINDOW_SIZE[0], 0.0],
            imgui::Condition::Appearing,
        )
        .size(MEMORY_WINDOW_SIZE, imgui::Condition::Appearing)
        .resizable(false)
        .collapsible(false)
        .build(|| {
            for row in wram_text {
                ui.text(row);
            }
        });
}

const CPU_WINDOW_SIZE: [f32; 2] = [110.0, 236.0];

fn cpu_window(ui: &mut imgui::Ui, resolution: &glutin::dpi::PhysicalSize<u32>, snes: &SNES) {
    ui.window("CPU state")
        .position(
            [resolution.width as f32 - CPU_WINDOW_SIZE[0], 0.0],
            imgui::Condition::Appearing,
        )
        .size(CPU_WINDOW_SIZE, imgui::Condition::Appearing)
        .resizable(false)
        .collapsible(false)
        .build(|| {
            for row in cpu_status_str(&snes.cpu) {
                ui.text(row);
            }
        });
}

const SMP_WINDOW_SIZE: [f32; 2] = [CPU_WINDOW_SIZE[0], 152.0];

fn smp_window(ui: &mut imgui::Ui, resolution: &glutin::dpi::PhysicalSize<u32>, snes: &SNES) {
    ui.window("SMP state")
        .position(
            [
                resolution.width as f32 - SMP_WINDOW_SIZE[0],
                CPU_WINDOW_SIZE[1],
            ],
            imgui::Condition::Appearing,
        )
        .size(SMP_WINDOW_SIZE, imgui::Condition::Appearing)
        .resizable(false)
        .collapsible(false)
        .build(|| {
            for row in smp_status_str(&snes.apu.smp) {
                ui.text(row);
            }
        });
}

const PERF_WINDOW_SIZE: [f32; 2] = [200.0, 66.0];

fn perf_window(
    ui: &mut imgui::Ui,
    resolution: &glutin::dpi::PhysicalSize<u32>,
    data: &DrawData,
    ui_millis: f32,
) {
    ui.window("Perf info")
        .position(
            [
                resolution.width as f32 - PERF_WINDOW_SIZE[0],
                resolution.height as f32 - PERF_WINDOW_SIZE[1],
            ],
            imgui::Condition::Appearing,
        )
        .size(PERF_WINDOW_SIZE, imgui::Condition::Appearing)
        .resizable(false)
        .collapsible(false)
        .build(|| {
            ui.text(format!("Debug draw took {:>5.2}ms!", ui_millis));
            {
                let (message, color) = if data.missing_nanos > 0 {
                    (
                        format!("Lagged {:>5.2}ms behind!", data.missing_nanos as f32 * 1e-6),
                        [1.0, 0.0, 0.0, 1.0],
                    )
                } else {
                    (
                        format!(
                            "Emulation is {:>5.2}ms ahead!",
                            data.extra_nanos as f32 * 1e-6
                        ),
                        [1.0, 1.0, 1.0, 1.0],
                    )
                };
                ui.text_colored(color, message);
            }
        });
}

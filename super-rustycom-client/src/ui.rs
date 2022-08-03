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

const MEMORY_WINDOW_SIZE: [f32; 2] = [388.0, 344.0];
static mut WRAM_START_BYTE: u16 = 0;
static mut APU_RAM_START_BYTE: u16 = 0;

impl UI {
    pub fn new(display: &glium::Display) -> Self {
        let mut context = imgui::Context::create();
        context.set_ini_filename(None);

        let mut platform = WinitPlatform::init(&mut context);

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
            unsafe {
                mem_window(
                    frame_ui,
                    snes.abus.wram(),
                    "WRAM",
                    [DISASSEMBLY_WINDOW_SIZE[0], 0.0],
                    &mut WRAM_START_BYTE,
                );
            }
            unsafe {
                mem_window(
                    frame_ui,
                    snes.apu.bus.ram(),
                    "APU RAM",
                    [DISASSEMBLY_WINDOW_SIZE[0], MEMORY_WINDOW_SIZE[1]],
                    &mut APU_RAM_START_BYTE,
                );
            }
            palettes_window(frame_ui, snes);
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
        .movable(false)
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

fn mem_window(
    ui: &mut imgui::Ui,
    memory: &[u8],
    name: &str,
    position: [f32; 2],
    start_byte: &mut u16,
) {
    let shown_row_count: usize = 16;
    // Drop one line since we have the column header
    let end_byte = (*start_byte as usize) + shown_row_count * 0x0010;

    let mut text = vec![String::from(
        "      00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F",
    )];
    text.extend(
        memory[*start_byte as usize..end_byte]
            .chunks(0x10)
            .into_iter()
            // Zip line addrs with lines
            .zip((*start_byte as usize..memory.len()).step_by(0x0010))
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

    ui.window(name)
        .position(position, imgui::Condition::Appearing)
        .size(MEMORY_WINDOW_SIZE, imgui::Condition::Appearing)
        .resizable(false)
        .collapsible(false)
        .movable(false)
        .build(|| {
            for row in text {
                ui.text(row);
            }

            {
                let _width = ui.push_item_width(106.0);
                let mut addr = *start_byte as i32;
                let _ = ui
                    .input_int("Start addr", &mut addr)
                    .chars_hexadecimal(true)
                    .step(16)
                    .step_fast(16)
                    .display_format("$%04X")
                    .build();
                // Each row should be 16 bytes starting at XXX0
                *start_byte = (addr - addr % 16).max(0) as u16;
            }
        });
}

const PALETTES_WINDOW_SIZE: [f32; 2] = [336.0, 340.0];

fn palettes_window(ui: &mut imgui::Ui, snes: &SNES) {
    ui.window("Palettes")
        .position(
            [0.0, DISASSEMBLY_WINDOW_SIZE[1]],
            imgui::Condition::Appearing,
        )
        .size(PALETTES_WINDOW_SIZE, imgui::Condition::Appearing)
        .resizable(false)
        .collapsible(false)
        .movable(false)
        .build(|| {
            let cgram = snes.abus.cgram();
            for (ip, palette) in cgram.iter().chunks(32).into_iter().enumerate() {
                for (ic, color_chunk) in palette.into_iter().chunks(2).into_iter().enumerate() {
                    if ic == 0 {
                        ui.text(format!("{:X}", ip));
                        ui.same_line();
                    }
                    let _no_spacing = ui.push_style_var(imgui::StyleVar::ItemSpacing([0.0, 0.0]));

                    let color_bytes: Vec<u8> = color_chunk.cloned().collect();
                    let packed_color = ((color_bytes[1] as u16) << 8) | (color_bytes[0] as u16);
                    ui.color_button(
                        format!("##palette{}{}", ip, ic),
                        palette_color(packed_color),
                    );
                    if ic < 15 {
                        ui.same_line();
                    }
                }
            }
        });
}

fn palette_color(bgr555: u16) -> [f32; 4] {
    [
        ((((bgr555 << 3) & 0b1111_1000) | 0b111) as f32) / 255.0,
        ((((bgr555 >> 2) & 0b1111_1000) | 0b111) as f32) / 255.0,
        ((((bgr555 >> 7) & 0b1111_1000) | 0b111) as f32) / 255.0,
        1.0,
    ]
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
        .movable(false)
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
        .movable(false)
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
        .movable(false)
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

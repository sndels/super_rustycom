use glium::glutin;
use imgui::{FontConfig, FontSource};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use itertools::Itertools;
use std::time::Instant;
use super_rustycom_core::snes::SNES;

use crate::{
    debugger::{
        cpu_status_str, disassemble_current, disassemble_peek, smp_status_str, DebugState, Debugger,
    },
    draw_data::DrawData,
    expect,
};

pub struct UIContext {
    pub ui: UI,
    pub context: imgui::Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
}

impl UIContext {
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

        UIContext {
            ui: UI::default(),
            context,
            platform,
            renderer,
        }
    }
}

pub struct UI {
    execution: Execution,
    wram: MemWindow,
    apu_ram: MemWindow,
    cpu_window_opened: bool,
    smp_window_opened: bool,
    palettes_opened: bool,
}

impl Default for UI {
    fn default() -> Self {
        Self {
            execution: Execution::default(),
            wram: MemWindow::default(),
            apu_ram: MemWindow::default(),
            cpu_window_opened: true,
            smp_window_opened: false,
            palettes_opened: true,
        }
    }
}

struct Execution {
    opened: bool,
    scroll_to_current: bool,
    steps: i32,
}

impl Default for Execution {
    fn default() -> Self {
        Self {
            opened: true,
            scroll_to_current: true,
            steps: 1,
        }
    }
}

struct MemWindow {
    opened: bool,
    start_byte: u16,
}

impl Default for MemWindow {
    fn default() -> Self {
        Self {
            opened: false,
            start_byte: 0x0,
        }
    }
}

#[derive(Default)]
pub struct UIState {
    pub is_any_item_active: bool,
}

const MENU_BAR_HEIGHT: f32 = 19.0;
const TOP_LEFT: [f32; 2] = [0.0, MENU_BAR_HEIGHT];
const MEMORY_WINDOW_SIZE: [f32; 2] = [388.0, 344.0];
const EXECUTION_WINDOW_SIZE: [f32; 2] = [360.0, 424.0];
const EXECUTION_CHILD_WINDOW_SIZE: [f32; 2] = [EXECUTION_WINDOW_SIZE[0] - 10.0, 320.0];
const CPU_WINDOW_SIZE: [f32; 2] = [110.0, 236.0];
const SMP_WINDOW_SIZE: [f32; 2] = [CPU_WINDOW_SIZE[0], 152.0];
const PERF_WINDOW_SIZE: [f32; 2] = [204.0, 47.0];
const PALETTES_WINDOW_SIZE: [f32; 2] = [336.0, 340.0];

impl UI {
    pub fn draw(
        &mut self,
        ui: &mut imgui::Ui,
        resolution: &glutin::dpi::PhysicalSize<u32>,
        data: &mut DrawData,
        snes: &mut SNES,
        debugger: &mut Debugger,
    ) -> UIState {
        let ui_start = Instant::now();

        self.menu_bar(ui);

        execution_window(ui, snes, data, debugger, &mut self.execution);
        mem_window(ui, snes.abus.wram(), "WRAM", &mut self.wram);
        mem_window(ui, snes.apu.bus.ram(), "APU RAM", &mut self.apu_ram);
        palettes_window(ui, snes, &mut self.palettes_opened);
        cpu_window(ui, snes, &resolution, &mut self.cpu_window_opened);
        smp_window(ui, snes, &resolution, &mut self.smp_window_opened);

        let ui_millis = ui_start.elapsed().as_nanos() as f32 * 1e-6;

        perf_window(ui, &resolution, data, ui_millis);

        UIState {
            is_any_item_active: ui.is_any_item_active(),
        }
    }

    fn menu_bar(&mut self, ui: &mut imgui::Ui) {
        macro_rules! toggle {
            ($pred:expr, $boolean:expr) => {
                if $pred {
                    $boolean = !$boolean;
                }
            };
        }

        ui.main_menu_bar(|| {
            toggle!(ui.menu_item("Execution"), self.execution.opened);
            ui.menu("CPU", || {
                toggle!(ui.menu_item("CPU registers"), self.cpu_window_opened);
                toggle!(ui.menu_item("WRAM"), self.wram.opened);
            });
            ui.menu("APU", || {
                toggle!(ui.menu_item("SMP registers"), self.smp_window_opened);
                toggle!(ui.menu_item("APU RAM"), self.apu_ram.opened);
            });
            ui.menu("PPU", || {
                toggle!(ui.menu_item("Palettes"), self.palettes_opened);
            });
        });
    }
}

fn execution_window(
    ui: &mut imgui::Ui,
    snes: &mut SNES,
    data: &mut DrawData,
    debugger: &mut Debugger,
    execution: &mut Execution,
) {
    if execution.opened {
        let scroll_to_current = &mut execution.scroll_to_current;
        let steps = &mut execution.steps;
        ui.window("Execution")
            .position(TOP_LEFT, imgui::Condition::Appearing)
            .size(EXECUTION_WINDOW_SIZE, imgui::Condition::Appearing)
            .resizable(false)
            .collapsible(false)
            .opened(&mut execution.opened)
            .build(|| {
                ui.child_window("Disassembly")
                    .size(EXECUTION_CHILD_WINDOW_SIZE)
                    .scroll_bar(true)
                    .build(|| {
                        for row in data.disassembled_history() {
                            ui.text(format!("  {}", row));
                        }

                        let (current_str, current_size) =
                            disassemble_current(&snes.cpu, &mut snes.abus);
                        ui.text(format!("> {}", current_str));

                        if *scroll_to_current {
                            ui.set_scroll_here_y();
                        }

                        let mut peek_offset = current_size;
                        for _ in 0..20 {
                            let (disassembled, next_size) =
                                disassemble_peek(&snes.cpu, &mut snes.abus, peek_offset);
                            ui.text(disassembled);
                            peek_offset += next_size;
                        }
                    });

                match debugger.state {
                    DebugState::Active => {
                        if ui.button("Run ") {
                            debugger.state = DebugState::Run;
                        }
                    }
                    DebugState::Run => {
                        if ui.button("Stop") {
                            debugger.state = DebugState::Active;
                        }
                    }
                    _ => (),
                }

                ui.same_line();
                let should_step = ui.button("Step");

                ui.same_line();
                let steps = {
                    let _width = ui.push_item_width(40.0);
                    let _ = imgui::Drag::new("##Steps").range(1, 1000).build(ui, steps);
                    *steps
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

                ui.checkbox("Scroll to current", scroll_to_current);
            });
    }
}

fn mem_window(ui: &mut imgui::Ui, memory: &[u8], name: &str, settings: &mut MemWindow) {
    if settings.opened {
        let shown_row_count: usize = 16;
        // Drop one line since we have the column header
        let end_byte = (settings.start_byte as usize) + shown_row_count * 0x0010;

        let mut text = vec![String::from(
            "      00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F",
        )];
        text.extend(
            memory[settings.start_byte as usize..end_byte]
                .chunks(0x10)
                .into_iter()
                // Zip line addrs with lines
                .zip((settings.start_byte as usize..memory.len()).step_by(0x0010))
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

        // Explicit ref to avoid closure trying (and failing) to capture settings as a whole
        let start_byte = &mut settings.start_byte;
        ui.window(name)
            .position(TOP_LEFT, imgui::Condition::Appearing)
            .size(MEMORY_WINDOW_SIZE, imgui::Condition::Appearing)
            .resizable(false)
            .collapsible(false)
            .opened(&mut settings.opened)
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
}

fn palettes_window(ui: &mut imgui::Ui, snes: &SNES, opened: &mut bool) {
    if *opened {
        ui.window("Palettes")
            .position(
                [EXECUTION_WINDOW_SIZE[0], MENU_BAR_HEIGHT],
                imgui::Condition::Appearing,
            )
            .size(PALETTES_WINDOW_SIZE, imgui::Condition::Appearing)
            .resizable(false)
            .collapsible(false)
            .opened(opened)
            .build(|| {
                let cgram = snes.abus.cgram();
                for (ip, palette) in cgram.iter().chunks(32).into_iter().enumerate() {
                    for (ic, color_chunk) in palette.into_iter().chunks(2).into_iter().enumerate() {
                        if ic == 0 {
                            ui.text(format!("{:X}", ip));
                            ui.same_line();
                        }
                        let _no_spacing =
                            ui.push_style_var(imgui::StyleVar::ItemSpacing([0.0, 0.0]));

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
}

fn palette_color(bgr555: u16) -> [f32; 4] {
    [
        ((((bgr555 << 3) & 0b1111_1000) | 0b111) as f32) / 255.0,
        ((((bgr555 >> 2) & 0b1111_1000) | 0b111) as f32) / 255.0,
        ((((bgr555 >> 7) & 0b1111_1000) | 0b111) as f32) / 255.0,
        1.0,
    ]
}

fn cpu_window(
    ui: &mut imgui::Ui,
    snes: &SNES,
    resolution: &glutin::dpi::PhysicalSize<u32>,
    opened: &mut bool,
) {
    if *opened {
        ui.window("CPU state")
            .position(
                [
                    resolution.width as f32 - CPU_WINDOW_SIZE[0],
                    MENU_BAR_HEIGHT,
                ],
                imgui::Condition::Appearing,
            )
            .size(CPU_WINDOW_SIZE, imgui::Condition::Appearing)
            .resizable(false)
            .collapsible(false)
            .opened(opened)
            .build(|| {
                for row in cpu_status_str(&snes.cpu) {
                    ui.text(row);
                }
            });
    }
}

fn smp_window(
    ui: &mut imgui::Ui,
    snes: &SNES,
    resolution: &glutin::dpi::PhysicalSize<u32>,
    opened: &mut bool,
) {
    if *opened {
        ui.window("SMP state")
            .position(
                [
                    resolution.width as f32 - SMP_WINDOW_SIZE[0],
                    MENU_BAR_HEIGHT + CPU_WINDOW_SIZE[1],
                ],
                imgui::Condition::Appearing,
            )
            .size(SMP_WINDOW_SIZE, imgui::Condition::Appearing)
            .resizable(false)
            .collapsible(false)
            .opened(opened)
            .build(|| {
                for row in smp_status_str(&snes.apu.smp) {
                    ui.text(row);
                }
            });
    }
}

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
        .no_decoration()
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

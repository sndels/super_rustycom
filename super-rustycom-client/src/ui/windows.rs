use glium::glutin;
use itertools::Itertools;
use super_rustycom_core::snes::Snes;

use crate::{
    debugger::{
        cpu_status_str, disassemble_current, disassemble_peek, smp_status_str, DebugState, Debugger,
    },
    draw_data::DrawData,
};

const REAL_TIME_FRAME_NANOS: u128 = 16666667;
const MENU_BAR_HEIGHT: f32 = 19.0;
const TOP_LEFT: [f32; 2] = [0.0, MENU_BAR_HEIGHT];
const MEMORY_WINDOW_SIZE: [f32; 2] = [388.0, 344.0];
const EXECUTION_WINDOW_SIZE: [f32; 2] = [360.0, 424.0];
const EXECUTION_CHILD_WINDOW_SIZE: [f32; 2] = [EXECUTION_WINDOW_SIZE[0] - 10.0, 320.0];
const CPU_WINDOW_SIZE: [f32; 2] = [110.0, 236.0];
const SMP_WINDOW_SIZE: [f32; 2] = [CPU_WINDOW_SIZE[0], 152.0];
const PERF_WINDOW_SIZE: [f32; 2] = [204.0, 47.0];
const PALETTES_WINDOW_SIZE: [f32; 2] = [336.0, 340.0];

pub struct Execution {
    pub opened: bool,
    scroll_to_current: bool,
}

impl Execution {
    pub fn new(opened: bool) -> Self {
        Self {
            opened,
            scroll_to_current: true,
        }
    }

    pub fn draw(
        &mut self,
        ui: &mut imgui::Ui,
        snes: &mut Snes,
        data: &mut DrawData,
        debugger: &mut Debugger,
        full_reset_triggered: &mut bool,
    ) {
        if self.opened {
            let scroll_to_current = &mut self.scroll_to_current;
            ui.window("Execution")
                .position(TOP_LEFT, imgui::Condition::Appearing)
                .size(EXECUTION_WINDOW_SIZE, imgui::Condition::Appearing)
                .resizable(false)
                .collapsible(false)
                .opened(&mut self.opened)
                .build(|| {
                    ui.child_window("Disassembly")
                        .size(EXECUTION_CHILD_WINDOW_SIZE)
                        .scroll_bar(true)
                        .build(|| {
                            for row in data.disassembled_history() {
                                ui.text(format!("  {}", row));
                            }

                            let (current_str, current_size) =
                                disassemble_current(&snes.cpu, &snes.abus);
                            ui.text(format!("> {}", current_str));

                            if *scroll_to_current {
                                ui.set_scroll_here_y();
                            }

                            let mut peek_offset = current_size;
                            for _ in 0..20 {
                                let (disassembled, next_size) =
                                    disassemble_peek(&snes.cpu, &snes.abus, peek_offset);
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
                    if ui.button("Step") {
                        debugger.state = DebugState::Step;
                    }

                    ui.same_line();
                    {
                        let _width = ui.push_item_width(40.0);
                        let _ = imgui::Drag::new("##Steps")
                            .range(1, 1000)
                            .build(ui, &mut debugger.steps);
                    }

                    ui.same_line();
                    if ui.button("Cpu reset") {
                        snes.cpu.reset(&mut snes.abus);
                        data.clear_history();
                        debugger.state = DebugState::Active;
                    }

                    ui.same_line();
                    *full_reset_triggered = ui.button("Full reset");

                    {
                        let _width = ui.push_item_width(58.0);
                        let mut bp = debugger.breakpoint as i32;
                        let _ = ui
                            .input_scalar("Breakpoint", &mut bp)
                            .chars_hexadecimal(true)
                            .display_format("$%06X")
                            .build();
                        debugger.breakpoint = bp.max(0).min(0xFFFFFF) as u32;
                    }

                    ui.checkbox("Scroll to current", scroll_to_current);
                });
        }
    }
}

pub struct Memory {
    pub opened: bool,
    name: String,
    start_byte: u16,
}

impl Memory {
    pub fn new(name: &str, opened: bool) -> Self {
        Self {
            opened,
            name: String::from(name),
            start_byte: 0x0,
        }
    }

    pub fn draw(&mut self, ui: &mut imgui::Ui, memory: &[u8]) {
        if self.opened {
            let shown_row_count: usize = 16;
            // Drop one line since we have the column header
            let end_byte = (self.start_byte as usize) + shown_row_count * 0x0010;

            let mut text = vec![String::from(
                "      00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F",
            )];
            text.extend(
                memory[self.start_byte as usize..end_byte]
                    .chunks(0x10)
                    .into_iter()
                    // Zip line addrs with lines
                    .zip((self.start_byte as usize..memory.len()).step_by(0x0010))
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
            let start_byte = &mut self.start_byte;
            ui.window(&self.name)
                .position(TOP_LEFT, imgui::Condition::Appearing)
                .size(MEMORY_WINDOW_SIZE, imgui::Condition::Appearing)
                .resizable(false)
                .collapsible(false)
                .opened(&mut self.opened)
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
}

pub struct Palettes {
    pub opened: bool,
}

impl Palettes {
    pub fn new(opened: bool) -> Self {
        Self { opened }
    }

    pub fn draw(&mut self, ui: &mut imgui::Ui, snes: &Snes) {
        if self.opened {
            ui.window("Palettes")
                .position(
                    [EXECUTION_WINDOW_SIZE[0], MENU_BAR_HEIGHT],
                    imgui::Condition::Appearing,
                )
                .size(PALETTES_WINDOW_SIZE, imgui::Condition::Appearing)
                .resizable(false)
                .collapsible(false)
                .opened(&mut self.opened)
                .build(|| {
                    for p in 0..=0xF {
                        for c in 0..=0xF {
                            if c == 0 {
                                ui.text(format!("{:X}", p));
                                // this before no spacing forces a space
                                ui.same_line();
                            }
                            let _no_spacing =
                                ui.push_style_var(imgui::StyleVar::ItemSpacing([0.0, 0.0]));
                            ui.color_button(
                                format!("##palette{}{}", p, c),
                                get_palette_color(p, c, snes.abus.cgram()),
                            );
                            if c < 15 {
                                // _no_spacing so next elem will be tight if it also has _no_spacing
                                ui.same_line();
                            }
                        }
                    }
                });
        }
    }
}

fn get_palette_color(palette: u8, color: u8, cgram: &[u8]) -> [f32; 4] {
    let word_addr = (palette as usize) * 16 + (color as usize);
    let low_byte = cgram[word_addr * 2];
    let high_byte = cgram[word_addr * 2 + 1];

    let bgr555 = ((high_byte as u16) << 8) | (low_byte as u16);

    [
        ((((bgr555 << 3) & 0b1111_1000) | 0b111) as f32) / 255.0,
        ((((bgr555 >> 2) & 0b1111_1000) | 0b111) as f32) / 255.0,
        ((((bgr555 >> 7) & 0b1111_1000) | 0b111) as f32) / 255.0,
        1.0,
    ]
}
pub struct Cpu {
    pub opened: bool,
}

impl Cpu {
    pub fn new(opened: bool) -> Self {
        Self { opened }
    }

    pub fn draw(
        &mut self,
        ui: &mut imgui::Ui,
        snes: &Snes,
        resolution: &glutin::dpi::PhysicalSize<u32>,
    ) {
        if self.opened {
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
                .opened(&mut self.opened)
                .build(|| {
                    for row in cpu_status_str(&snes.cpu) {
                        ui.text(row);
                    }
                });
        }
    }
}

pub struct Smp {
    pub opened: bool,
}

impl Smp {
    pub fn new(opened: bool) -> Self {
        Self { opened }
    }
    pub fn draw(
        &mut self,
        ui: &mut imgui::Ui,
        snes: &Snes,
        resolution: &glutin::dpi::PhysicalSize<u32>,
    ) {
        if self.opened {
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
                .opened(&mut self.opened)
                .build(|| {
                    for row in smp_status_str(&snes.apu.smp) {
                        ui.text(row);
                    }
                });
        }
    }
}

pub fn performance(
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
                let (message, color) = if data.emulated_nanos < data.spent_nanos {
                    (
                        format!(
                            "Lagged by {:>5.2}ms",
                            (data.spent_nanos - data.emulated_nanos) as f32 * 1e-6
                        ),
                        [1.0, 1.0, 0.0, 1.0],
                    )
                } else if data.spent_nanos > REAL_TIME_FRAME_NANOS {
                    (
                        format!(
                            "Not real-time! ({:>5.2}ms)",
                            (data.spent_nanos as f32) * 1e-6
                        ),
                        [1.0, 0.0, 0.0, 1.0],
                    )
                } else {
                    (
                        format!(
                            "Ahead by {:>5.2}ms",
                            (data.emulated_nanos - data.spent_nanos) as f32 * 1e-6
                        ),
                        [1.0, 1.0, 1.0, 1.0],
                    )
                };
                ui.text_colored(color, message);
            }
        });
}

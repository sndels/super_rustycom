use egui::Widget;
use glium::{
    backend::Facade,
    glutin,
    texture::RawImage2d,
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, SamplerBehavior},
    Rect, Texture2d,
};
use itertools::Itertools;
use std::{rc::Rc, str::FromStr, string::ToString};
use strum::{Display, EnumString, EnumVariantNames, VariantNames};
use super_rustycom_core::snes::Snes;

use super::hex_input::HexInput;
use crate::{
    debugger::{
        cpu_status_str, disassemble_current, disassemble_peek, smp_status_str, DebugState, Debugger,
    },
    draw_data::DrawData,
};

const REAL_TIME_FRAME_NANOS: u128 = 16666667;
const MENU_BAR_HEIGHT: f32 = 19.0;
const TOP_LEFT: [f32; 2] = [0.0, MENU_BAR_HEIGHT];
const EXECUTION_WINDOW_SIZE: [f32; 2] = [360.0, 424.0];
const EXECUTION_CHILD_WINDOW_SIZE: [f32; 2] = [EXECUTION_WINDOW_SIZE[0] - 10.0, 320.0];
const CPU_WINDOW_SIZE: [f32; 2] = [110.0, 236.0];
const SMP_WINDOW_SIZE: [f32; 2] = [CPU_WINDOW_SIZE[0], 152.0];
const PERF_WINDOW_SIZE: [f32; 2] = [204.0, 47.0];
const PALETTES_WINDOW_SIZE: [f32; 2] = [334.0, 340.0];
const SPRITE_ATTRIBUTES_WINDOW_SIZE: [f32; 2] = [334.0, 310.0];
const SPRITE_ATTRIBUTES_CHILD_WINDOW_SIZE: [f32; 2] = [
    SPRITE_ATTRIBUTES_WINDOW_SIZE[0] - 10.0,
    SPRITE_ATTRIBUTES_WINDOW_SIZE[1] - 35.0,
];

const MEMORY_HEX_WINDOW_SIZE: [f32; 2] = [388.0, 344.0];
const MEMORY_TILE_WINDOW_SIZE: [f32; 2] = [528.0, 382.0];
const MEMORY_TILE_CHILD_WINDOW_SIZE: [f32; 2] = [527.0, 324.0];
const MEMORY_TILE_WINDOW_TEXTURE_SCALE: f32 = 4.0;
const ROWS_IN_TILE: u16 = 8;
const COLUMNS_IN_TILE: u16 = 8;
const PIXELS_IN_TILE: u16 = ROWS_IN_TILE * COLUMNS_IN_TILE;
const MEMORY_TILE_WINDOW_ROW_LENGTH: u16 = 16;
const MEMORY_TILE_WINDOW_ROW_COUNT: u16 = 10;

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
        ctx: &egui::Context,
        snes: &mut Snes,
        data: &mut DrawData,
        debugger: &mut Debugger,
        full_reset_triggered: &mut bool,
    ) {
        let scroll_to_current = &mut self.scroll_to_current;
        egui::Window::new("Execution")
            .fixed_size(EXECUTION_WINDOW_SIZE)
            .default_pos(TOP_LEFT)
            .resizable(false)
            .collapsible(false)
            .open(&mut self.opened)
            .show(ctx, |ui| {
                ui.heading("Disassembly");
                // TODO: Should be a toggleable panel?
                egui::ScrollArea::new([false, true])
                    .max_width(EXECUTION_CHILD_WINDOW_SIZE[0])
                    .max_height(EXECUTION_CHILD_WINDOW_SIZE[1])
                    .show(ui, |ui| {
                        for row in data.disassembled_history() {
                            ui.label(format!("  {}", row));
                        }

                        let (current_str, current_size) =
                            disassemble_current(&snes.cpu, &snes.abus);
                        let label_rect = ui.label(format!("> {}", current_str)).rect;

                        if *scroll_to_current {
                            ui.scroll_to_rect(label_rect, Some(egui::Align::Center));
                        }

                        let mut peek_offset = current_size;
                        for _ in 0..20 {
                            let (disassembled, next_size) =
                                disassemble_peek(&snes.cpu, &snes.abus, peek_offset);
                            ui.label(disassembled);
                            peek_offset += next_size;
                        }
                    });

                ui.horizontal(|ui| {
                    match debugger.state {
                        DebugState::Active => {
                            if ui.button("Run ").clicked() {
                                debugger.state = DebugState::Run;
                            }
                        }
                        DebugState::Run => {
                            if ui.button("Stop").clicked() {
                                debugger.state = DebugState::Active;
                            }
                        }
                        _ => (),
                    }

                    if ui.button("Step").clicked() {
                        debugger.state = DebugState::Step;
                    }

                    {
                        // TODO: ui.set_width(40.0);
                        let _ = egui::DragValue::new(&mut debugger.steps).clamp_range(1..=1000);
                    }

                    if ui.button("Cpu reset").clicked() {
                        snes.cpu.reset(&mut snes.abus);
                        data.clear_history();
                        debugger.state = DebugState::Active;
                    }

                    *full_reset_triggered = ui.button("Full reset").clicked();
                });

                // { // TODO: Widget for this needs a non-transient backing buffer for editable string
                //     // TOOD: ui.set_width(58.0);
                //     ui.label("Breakpoint");
                //     let _ = HexInput::new(&mut debugger.breakpoint).ui(ui);
                //     debugger.breakpoint = debugger.breakpoint.max(0).min(0xFFFFFF) as u32;
                // }

                ui.checkbox(scroll_to_current, "Scroll to current");
            });
    }
}

// pub struct Memory {
//     pub opened: bool,
//     mode: MemoryMode,
//     name: String,
//     // HexDump
//     start_byte: u16,
//     // Tiles
//     start_row: u16,
//     palette: u8,
//     tile_texture: Rc<Texture2d>,
//     tile_texture_id: TextureId,
// }

#[derive(Display, EnumVariantNames, EnumString)]
pub enum MemoryMode {
    HexDump,
    Tiles,
}

// impl Memory {
//     pub fn new<F>(
//         name: &str,
//         opened: bool,
//         mode: MemoryMode,
//         context: &F,
//         textures: &mut Textures<Texture>,
//     ) -> Self
//     where
//         F: ?Sized + Facade,
//     {
//         let tile_texture = Rc::new(
//             Texture2d::empty(
//                 context,
//                 (COLUMNS_IN_TILE as u32) * (MEMORY_TILE_WINDOW_ROW_LENGTH as u32),
//                 (ROWS_IN_TILE as u32) * (MEMORY_TILE_WINDOW_ROW_COUNT as u32),
//             )
//             .expect("Failed to create a tile view texture"),
//         );
//         let sampler = SamplerBehavior {
//             magnify_filter: MagnifySamplerFilter::Nearest,
//             minify_filter: MinifySamplerFilter::Nearest,
//             ..SamplerBehavior::default()
//         };

//         let tile_texture_id = textures.insert(Texture {
//             texture: Rc::clone(&tile_texture),
//             sampler,
//         });

//         Self {
//             opened,
//             mode,
//             name: String::from(name),
//             start_byte: 0x0,
//             start_row: 0,
//             palette: 8, // Sprite palette 0
//             tile_texture,
//             tile_texture_id,
//         }
//     }

//     pub fn draw(&mut self, ui: &mut imgui::Ui, memory: &[u8], cgram: &[u8]) {
//         if self.opened {
//             match self.mode {
//                 MemoryMode::HexDump => self.draw_hex_dump(ui, memory),
//                 MemoryMode::Tiles => self.draw_tiles(ui, memory, cgram),
//             }
//         }
//     }

//     fn draw_hex_dump(&mut self, ui: &mut imgui::Ui, memory: &[u8]) {
//         let shown_row_count: usize = 16;
//         // Drop one line since we have the column header
//         let end_byte = (self.start_byte as usize) + shown_row_count * 0x0010;

//         let mut text = vec![String::from(
//             "      00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F",
//         )];
//         text.extend(
//             memory[self.start_byte as usize..end_byte]
//                 .chunks(0x10)
//                 .into_iter()
//                 // Zip line addrs with lines
//                 .zip((self.start_byte as usize..memory.len()).step_by(0x0010))
//                 // Create line string with space between bytes
//                 .map(|(line, addr)| {
//                     format!(
//                         "${:04X} {}",
//                         addr,
//                         line.iter()
//                             .format_with(" ", |elt, f| f(&format_args!("{:02X}", elt)))
//                     )
//                 })
//                 .collect_vec(),
//         );

//         // Explicit refs to avoid closure trying (and failing) to capture self
//         let start_byte = &mut self.start_byte;
//         let name = &self.name;
//         let mode = &mut self.mode;
//         ui.window(name)
//             .position(
//                 [EXECUTION_WINDOW_SIZE[0], MENU_BAR_HEIGHT],
//                 imgui::Condition::Appearing,
//             )
//             .size(MEMORY_HEX_WINDOW_SIZE, imgui::Condition::Always)
//             .resizable(false)
//             .collapsible(false)
//             .opened(&mut self.opened)
//             .build(|| {
//                 for row in text {
//                     ui.text(row);
//                 }

//                 {
//                     let _width = ui.push_item_width(106.0);
//                     enum_combo_box(ui, &(String::from("Mode##") + name), mode);
//                 }
//                 {
//                     let _width = ui.push_item_width(106.0);
//                     let mut addr = *start_byte as i32;
//                     ui.same_line();
//                     let _ = ui
//                         .input_int("Start addr", &mut addr)
//                         .chars_hexadecimal(true)
//                         .step(16)
//                         .step_fast(16)
//                         .display_format("$%04X")
//                         .build();
//                     // Each row should be 16 bytes starting at XXX0
//                     *start_byte = (addr - addr % 16).max(0) as u16;
//                 }
//             });
//     }

//     fn draw_tiles(&mut self, ui: &mut imgui::Ui, memory: &[u8], cgram: &[u8]) {
//         const SHOWN_PX_COUNT: usize = (PIXELS_IN_TILE as usize)
//             * (MEMORY_TILE_WINDOW_ROW_LENGTH as usize)
//             * (MEMORY_TILE_WINDOW_ROW_COUNT as usize);
//         assert!(memory.len() % (32 * MEMORY_TILE_WINDOW_ROW_LENGTH as usize) == 0);
//         let memory_row_count =
//             (memory.len() / 32 / (MEMORY_TILE_WINDOW_ROW_LENGTH as usize)) as u16;

//         let mut pixels = Vec::with_capacity(SHOWN_PX_COUNT * 3);
//         for r in 0..MEMORY_TILE_WINDOW_ROW_COUNT {
//             for tr in 0..ROWS_IN_TILE {
//                 for t in 0..MEMORY_TILE_WINDOW_ROW_LENGTH {
//                     let tile_index = (self.start_row + r) * MEMORY_TILE_WINDOW_ROW_LENGTH + t;
//                     let tile_row_pixels = read_tile32_row(tile_index, tr as u8, memory);
//                     for px in tile_row_pixels {
//                         let colors = get_palette_color(self.palette, px, cgram);
//                         pixels.push((colors[0] * 255.0) as u8);
//                         pixels.push((colors[1] * 255.0) as u8);
//                         pixels.push((colors[2] * 255.0) as u8);
//                     }
//                 }
//             }
//         }
//         // TODO: Don't allocate new backing and texture for every frame
//         let image = RawImage2d::from_raw_rgb(
//             pixels,
//             (
//                 (MEMORY_TILE_WINDOW_ROW_LENGTH * COLUMNS_IN_TILE) as u32,
//                 (MEMORY_TILE_WINDOW_ROW_COUNT * ROWS_IN_TILE) as u32,
//             ),
//         );

//         self.tile_texture.write(
//             Rect {
//                 left: 0,
//                 bottom: 0,
//                 width: self.tile_texture.width(),
//                 height: self.tile_texture.height(),
//             },
//             image,
//         );

//         // Explicit refs to avoid closure trying (and failing) to capture self
//         let name = &self.name;
//         let mode = &mut self.mode;
//         let start_row = &mut self.start_row;
//         let palette = &mut self.palette; // TODO: Wrap in MemoryMode
//         let tile_texture = &mut self.tile_texture;
//         let tile_texture_id = self.tile_texture_id;
//         ui.window(name)
//             .position(
//                 [EXECUTION_WINDOW_SIZE[0], MENU_BAR_HEIGHT],
//                 imgui::Condition::Appearing,
//             )
//             .size(MEMORY_TILE_WINDOW_SIZE, imgui::Condition::Always)
//             .resizable(false)
//             .collapsible(false)
//             .opened(&mut self.opened)
//             .build(|| {
//                 ui.child_window("Disassembly")
//                     .size(MEMORY_TILE_CHILD_WINDOW_SIZE)
//                     .scroll_bar(true)
//                     .build(|| {
//                         Image::new(
//                             tile_texture_id,
//                             [
//                                 (tile_texture.width() as f32) * MEMORY_TILE_WINDOW_TEXTURE_SCALE,
//                                 (tile_texture.height() as f32) * MEMORY_TILE_WINDOW_TEXTURE_SCALE,
//                             ],
//                         )
//                         .build(ui);
//                     });
//                 {
//                     let _width = ui.push_item_width(106.0);
//                     enum_combo_box(ui, &(String::from("Mode##") + name), mode);
//                 }
//                 {
//                     let _width = ui.push_item_width(24.0);
//                     ui.same_line();
//                     let _ = ui
//                         .input_scalar("Palette", palette)
//                         .chars_hexadecimal(true)
//                         .display_format("%X")
//                         .build();
//                     *palette = (*palette).max(0).min(0xF);
//                 }
//                 {
//                     let _width = ui.push_item_width(75.0);
//                     ui.same_line();
//                     let _ = ui
//                         .input_scalar("Start row", start_row)
//                         .step(1)
//                         .step_fast(1)
//                         .build();
//                     *start_row = (*start_row)
//                         .max(0)
//                         .min(memory_row_count - MEMORY_TILE_WINDOW_ROW_COUNT);
//                 }
//             });
//     }
// }

// pub fn read_tile32_row(tile: u16, row: u8, memory: &[u8]) -> [u8; 8] {
//     assert!((tile as usize) < memory.len() / 32);

//     let mut planes = [0; 4];
//     // 8x8 tiles stored 4bits per pixel as 4 planes
//     // Plane N is the Nth bits of each pixel, stored byte per row
//     // Planes 0 and 1, 2 and 3 are stored as interleaved
//     // So p0r0, p1r0, p0r1, p1r1, ... , p1r7, p2r0, p3r0, ... , p3r7
//     planes[0] = memory[(tile as usize) * 32 + 2 * (row as usize)];
//     planes[1] = memory[(tile as usize) * 32 + 2 * (row as usize) + 1];
//     planes[2] = memory[(tile as usize) * 32 + 16 + 2 * (row as usize)];
//     planes[3] = memory[(tile as usize) * 32 + 16 + 2 * (row as usize) + 1];

//     let mut pixels = [0; 8];
//     for px in 0..8 {
//         for pl in 0..3 {
//             pixels[px] |= ((planes[pl] >> (7 - px)) & 0x1) << pl;
//         }
//     }

//     pixels
// }

// fn enum_combo_box<T>(ui: &imgui::Ui, name: &str, value: &mut T) -> bool
// where
//     T: VariantNames + ToString + FromStr,
//     T::Err: std::fmt::Debug,
// {
//     let mut current_t = T::VARIANTS
//         .iter()
//         .position(|&n| n == value.to_string())
//         .unwrap();

//     let changed = ui.combo_simple_string(name, &mut current_t, T::VARIANTS);

//     if changed {
//         *value = T::from_str(T::VARIANTS[current_t]).unwrap();
//     }

//     changed
// }

// pub struct Palettes {
//     pub opened: bool,
// }

// impl Palettes {
//     pub fn new(opened: bool) -> Self {
//         Self { opened }
//     }

//     pub fn draw(&mut self, ui: &mut imgui::Ui, snes: &Snes) {
//         if self.opened {
//             ui.window("Palettes")
//                 .position(
//                     [0.0, MENU_BAR_HEIGHT + EXECUTION_WINDOW_SIZE[1]],
//                     imgui::Condition::Appearing,
//                 )
//                 .size(PALETTES_WINDOW_SIZE, imgui::Condition::Appearing)
//                 .resizable(false)
//                 .collapsible(false)
//                 .opened(&mut self.opened)
//                 .build(|| {
//                     for p in 0..=0xF {
//                         for c in 0..=0xF {
//                             if c == 0 {
//                                 ui.text(format!("{:X}", p));
//                                 // this before no spacing forces a space
//                                 ui.same_line();
//                             }
//                             let _no_spacing =
//                                 ui.push_style_var(imgui::StyleVar::ItemSpacing([0.0, 0.0]));
//                             ui.color_button(
//                                 format!("##palette{}{}", p, c),
//                                 get_palette_color(p, c, snes.abus.cgram()),
//                             );
//                             if c < 15 {
//                                 // _no_spacing so next elem will be tight if it also has _no_spacing
//                                 ui.same_line();
//                             }
//                         }
//                     }
//                 });
//         }
//     }
// }

// fn get_palette_color(palette: u8, color: u8, cgram: &[u8]) -> [f32; 4] {
//     let word_addr = (palette as usize) * 16 + (color as usize);
//     let low_byte = cgram[word_addr * 2];
//     let high_byte = cgram[word_addr * 2 + 1];

//     let bgr555 = ((high_byte as u16) << 8) | (low_byte as u16);

//     [
//         ((((bgr555 << 3) & 0b1111_1000) | 0b111) as f32) / 255.0,
//         ((((bgr555 >> 2) & 0b1111_1000) | 0b111) as f32) / 255.0,
//         ((((bgr555 >> 7) & 0b1111_1000) | 0b111) as f32) / 255.0,
//         1.0,
//     ]
// }

// pub struct SpriteAttributes {
//     pub opened: bool,
// }

// impl SpriteAttributes {
//     pub fn new(opened: bool) -> Self {
//         Self { opened }
//     }

//     pub fn draw(&mut self, ui: &mut imgui::Ui, snes: &Snes) {
//         if self.opened {
//             ui.window("Sprite Attributes")
//                 .position(
//                     [
//                         EXECUTION_WINDOW_SIZE[0],
//                         MENU_BAR_HEIGHT + MEMORY_TILE_WINDOW_SIZE[1],
//                     ],
//                     imgui::Condition::Appearing,
//                 )
//                 .size(SPRITE_ATTRIBUTES_WINDOW_SIZE, imgui::Condition::Appearing)
//                 .resizable(false)
//                 .collapsible(false)
//                 .opened(&mut self.opened)
//                 .build(|| {
//                     ui.child_window("Disassembly")
//                         .size(SPRITE_ATTRIBUTES_CHILD_WINDOW_SIZE)
//                         .scroll_bar(true)
//                         .build(|| {
//                             const SPRITE_COUNT: usize = 128;
//                             const MAIN_BANK_BYTESIZE: usize = 4 * SPRITE_COUNT;
//                             let oam = snes.abus.oam();
//                             for i in 0..SPRITE_COUNT {
//                                 // 4 bytes indexed from the start of the memory
//                                 let bytes = &oam[(i * 4)..((i + 1) * 4)];
//                                 // Additional 2 bits at the end, indexed from the end of the 4byte entries
//                                 let additional_bits =
//                                     (oam[MAIN_BANK_BYTESIZE + i / 4] >> ((i % 4) * 2)) & 0b11;

//                                 let x_coord =
//                                     (((additional_bits & 0b1) as u16) << 8) | (bytes[0] as u16);
//                                 let y_coord = bytes[1];

//                                 let attributes = bytes[3];
//                                 let tile = (((attributes & 0b1) as u16) << 8) | (bytes[2] as u16);
//                                 let palette = (attributes >> 1) & 0b111;
//                                 let priority = (attributes >> 4) & 0b11;
//                                 let mirror_x = (attributes >> 6) & 0b1;
//                                 let mirror_y = (attributes >> 7) & 0b1;
//                                 let size = additional_bits >> 1;

//                                 ui.text(format!(
//                                     "{i:<3}      X: {x_coord:<3}  Mirror: {}",
//                                     if mirror_x == 1 { "Y" } else { "N" },
//                                 ));
//                                 ui.text(format!(
//                                     "         Y: {y_coord:<3}  Mirror: {}",
//                                     if mirror_y == 1 { "Y" } else { "N" },
//                                 ));
//                                 ui.text(format!("      Tile: {tile:<3} Palette: {palette}"));
//                                 ui.text(format!(
//                                     "  Priority: {priority}      Size: {}",
//                                     if size == 1 { "Large" } else { "Small" }
//                                 ));
//                             }
//                         })
//                 });
//         }
//     }
// }

// pub struct Cpu {
//     pub opened: bool,
// }

// impl Cpu {
//     pub fn new(opened: bool) -> Self {
//         Self { opened }
//     }

//     pub fn draw(
//         &mut self,
//         ui: &mut imgui::Ui,
//         snes: &Snes,
//         resolution: &glutin::dpi::PhysicalSize<u32>,
//     ) {
//         if self.opened {
//             ui.window("CPU state")
//                 .position(
//                     [
//                         resolution.width as f32 - CPU_WINDOW_SIZE[0],
//                         MENU_BAR_HEIGHT,
//                     ],
//                     imgui::Condition::Appearing,
//                 )
//                 .size(CPU_WINDOW_SIZE, imgui::Condition::Appearing)
//                 .resizable(false)
//                 .collapsible(false)
//                 .opened(&mut self.opened)
//                 .build(|| {
//                     for row in cpu_status_str(&snes.cpu) {
//                         ui.text(row);
//                     }
//                 });
//         }
//     }
// }

// pub struct Smp {
//     pub opened: bool,
// }

// impl Smp {
//     pub fn new(opened: bool) -> Self {
//         Self { opened }
//     }
//     pub fn draw(
//         &mut self,
//         ui: &mut imgui::Ui,
//         snes: &Snes,
//         resolution: &glutin::dpi::PhysicalSize<u32>,
//     ) {
//         if self.opened {
//             ui.window("SMP state")
//                 .position(
//                     [
//                         resolution.width as f32 - SMP_WINDOW_SIZE[0],
//                         MENU_BAR_HEIGHT + CPU_WINDOW_SIZE[1],
//                     ],
//                     imgui::Condition::Appearing,
//                 )
//                 .size(SMP_WINDOW_SIZE, imgui::Condition::Appearing)
//                 .resizable(false)
//                 .collapsible(false)
//                 .opened(&mut self.opened)
//                 .build(|| {
//                     for row in smp_status_str(&snes.apu.smp) {
//                         ui.text(row);
//                     }
//                 });
//         }
//     }
// }

// pub fn performance(
//     ui: &mut imgui::Ui,
//     resolution: &glutin::dpi::PhysicalSize<u32>,
//     data: &DrawData,
//     ui_millis: f32,
// ) {
//     ui.window("Perf info")
//         .position(
//             [
//                 resolution.width as f32 - PERF_WINDOW_SIZE[0],
//                 resolution.height as f32 - PERF_WINDOW_SIZE[1],
//             ],
//             imgui::Condition::Appearing,
//         )
//         .size(PERF_WINDOW_SIZE, imgui::Condition::Appearing)
//         .no_decoration()
//         .movable(false)
//         .build(|| {
//             ui.text(format!("Debug draw took {:>5.2}ms!", ui_millis));
//             {
//                 let (message, color) = if data.emulated_nanos < data.spent_nanos {
//                     (
//                         format!(
//                             "Lagged by {:>5.2}ms",
//                             (data.spent_nanos - data.emulated_nanos) as f32 * 1e-6
//                         ),
//                         [1.0, 1.0, 0.0, 1.0],
//                     )
//                 } else if data.spent_nanos > REAL_TIME_FRAME_NANOS {
//                     (
//                         format!(
//                             "Not real-time! ({:>5.2}ms)",
//                             (data.spent_nanos as f32) * 1e-6
//                         ),
//                         [1.0, 0.0, 0.0, 1.0],
//                     )
//                 } else {
//                     (
//                         format!(
//                             "Ahead by {:>5.2}ms",
//                             (data.emulated_nanos - data.spent_nanos) as f32 * 1e-6
//                         ),
//                         [1.0, 1.0, 1.0, 1.0],
//                     )
//                 };
//                 ui.text_colored(color, message);
//             }
//         });
// }

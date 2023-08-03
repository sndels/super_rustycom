mod context;
mod hex_input;
mod windows;

pub use context::Context;
use windows::MemoryMode;

use glium::{backend::Facade, glutin};
use std::time::Instant;
use super_rustycom_core::snes::Snes;

use crate::{debugger::Debugger, draw_data::DrawData};

pub struct Ui {
    execution: windows::Execution,
    // wram: windows::Memory,
    // apu_ram: windows::Memory,
    // vram: windows::Memory,
    // cpu: windows::Cpu,
    // smp: windows::Smp,
    // palettes: windows::Palettes,
    // oam: windows::SpriteAttributes,
}

#[derive(Default)]
pub struct State {
    pub is_any_item_active: bool,
    pub full_reset_triggered: bool,
}

impl Ui {
    fn new() -> Self
where {
        Self {
            execution: windows::Execution::new(true),
            // wram: windows::Memory::new("WRAM", false, MemoryMode::HexDump, context, textures),
            // apu_ram: windows::Memory::new("APU RAM", false, MemoryMode::HexDump, context, textures),
            // vram: windows::Memory::new("VRAM", true, MemoryMode::Tiles, context, textures),
            // cpu: windows::Cpu::new(true),
            // smp: windows::Smp::new(false),
            // palettes: windows::Palettes::new(true),
            // oam: windows::SpriteAttributes::new(true),
        }
    }

    pub fn reset(&mut self) {
        // TODO: Not needed with egui?
    }

    pub fn draw(
        &mut self,
        ctx: &egui::Context,
        data: &mut DrawData,
        snes: &mut Snes,
        debugger: &mut Debugger,
    ) -> State {
        let ui_start = Instant::now();

        egui::TopBottomPanel::top("TopBar").show(ctx, |ui| {
            self.menu_bar(ui);
        });

        let mut full_reset_triggered = false;

        self.execution
            .draw(ctx, snes, data, debugger, &mut full_reset_triggered);
        // self.wram.draw(ctx, snes.abus.wram(), snes.abus.cgram());
        // self.apu_ram
        //     .draw(ctx, snes.apu.bus.ram(), snes.abus.cgram());
        // self.vram.draw(ctx, snes.abus.vram(), snes.abus.cgram());
        // self.palettes.draw(ctx, snes);
        // self.oam.draw(ctx, snes);
        // self.cpu.draw(ctx, snes, resolution);
        // self.smp.draw(ctx, snes, resolution);

        let ui_millis = ui_start.elapsed().as_nanos() as f32 * 1e-6;

        // windows::performance(ctx, resolution, data, ui_millis);

        // TODO: query active input fields etc.
        State {
            is_any_item_active: false,
            full_reset_triggered,
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        macro_rules! toggle {
            ($ui:expr, $name:expr, $boolean:expr) => {
                if $ui.button($name).clicked() {
                    $boolean = !$boolean;
                }
            };
        }

        egui::menu::bar(ui, |ui| {
            toggle!(ui, "Execution", self.execution.opened);
            // ui.menu_button("CPU", |ui| {
            //     toggle!(ui, "CPU registers", self.cpu.opened);
            //     toggle!(ui, "WRAM", self.wram.opened);
            // });
            // ui.menu("APU", || {
            //     toggle!(ui.menu_item("SMP registers"), self.smp.opened);
            //     toggle!(ui.menu_item("APU RAM"), self.apu_ram.opened);
            // });
            // ui.menu("PPU", || {
            //     toggle!(ui.menu_item("Palettes"), self.palettes.opened);
            // });
            // ui.menu("PPU", || {
            //     toggle!(ui.menu_item("Sprites"), self.oam.opened);
            // });
        });
    }
}

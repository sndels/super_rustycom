mod context;
mod windows;

pub use context::Context;
use windows::MemoryMode;

use glium::{backend::Facade, glutin};
use imgui::Textures;
use imgui_glium_renderer::Texture;
use std::time::Instant;
use super_rustycom_core::snes::Snes;

use crate::{debugger::Debugger, draw_data::DrawData};

pub struct Ui {
    execution: windows::Execution,
    wram: windows::Memory,
    apu_ram: windows::Memory,
    vram: windows::Memory,
    cpu: windows::Cpu,
    smp: windows::Smp,
    palettes: windows::Palettes,
    oam: windows::SpriteAttributes,
}

#[derive(Default)]
pub struct State {
    pub is_any_item_active: bool,
    pub full_reset_triggered: bool,
}

impl Ui {
    fn new<F>(context: &F, textures: &mut Textures<Texture>) -> Self
    where
        F: ?Sized + Facade,
    {
        Self {
            execution: windows::Execution::new(true),
            wram: windows::Memory::new("WRAM", false, MemoryMode::HexDump, context, textures),
            apu_ram: windows::Memory::new("APU RAM", false, MemoryMode::HexDump, context, textures),
            vram: windows::Memory::new("VRAM", true, MemoryMode::Tiles, context, textures),
            cpu: windows::Cpu::new(true),
            smp: windows::Smp::new(false),
            palettes: windows::Palettes::new(true),
            oam: windows::SpriteAttributes::new(true),
        }
    }

    pub fn reset<F>(&mut self, context: &F, textures: &mut Textures<Texture>)
    where
        F: ?Sized + Facade,
    {
        *self = Ui::new(context, textures);
    }

    pub fn draw(
        &mut self,
        ui: &mut imgui::Ui,
        resolution: &glutin::dpi::PhysicalSize<u32>,
        data: &mut DrawData,
        snes: &mut Snes,
        debugger: &mut Debugger,
    ) -> State {
        let ui_start = Instant::now();

        self.menu_bar(ui);

        let mut full_reset_triggered = false;
        self.execution
            .draw(ui, snes, data, debugger, &mut full_reset_triggered);
        self.wram.draw(ui, snes.abus.wram(), snes.abus.cgram());
        self.apu_ram.draw(ui, snes.apu.bus.ram(), snes.abus.cgram());
        self.vram.draw(ui, snes.abus.vram(), snes.abus.cgram());
        self.palettes.draw(ui, snes);
        self.oam.draw(ui, snes);
        self.cpu.draw(ui, snes, resolution);
        self.smp.draw(ui, snes, resolution);

        let ui_millis = ui_start.elapsed().as_nanos() as f32 * 1e-6;

        windows::performance(ui, resolution, data, ui_millis);

        State {
            is_any_item_active: ui.is_any_item_active(),
            full_reset_triggered,
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
                toggle!(ui.menu_item("CPU registers"), self.cpu.opened);
                toggle!(ui.menu_item("WRAM"), self.wram.opened);
            });
            ui.menu("APU", || {
                toggle!(ui.menu_item("SMP registers"), self.smp.opened);
                toggle!(ui.menu_item("APU RAM"), self.apu_ram.opened);
            });
            ui.menu("PPU", || {
                toggle!(ui.menu_item("Palettes"), self.palettes.opened);
            });
            ui.menu("PPU", || {
                toggle!(ui.menu_item("Sprites"), self.oam.opened);
            });
        });
    }
}

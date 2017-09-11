use std::env;

mod abus;
mod cpu;
mod debugger;
mod dma;
mod mmap;
mod ppu_io;
mod rom;
mod op;

use abus::ABus;
use cpu::Cpu;
use debugger::Debugger;

fn main() {
    let rom_path = env::args().nth(1).expect("No rom defined");
    let mut abus = ABus::new(&rom_path);
    let mut cpu = Cpu::new(&mut abus);
    let mut debugger = Debugger::new();
    // Hack past the apu check in elix_nu
    abus.apu_write8(0xAA, 0x00);
    abus.apu_write8(0xBB, 0x01);
    loop {
        if debugger.active {
            debugger.take_command(&mut cpu, &mut abus);
        } else {
            while cpu.current_address() != debugger.breakpoint {
                cpu.step(&mut abus);
            }
            debugger.active = true;
        }
    }
}

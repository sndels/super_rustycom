mod abus;
mod cpu;
mod debugger;
mod dma;
mod joypad;
mod mmap;
mod mpydiv;
mod op;
mod ppu_io;
mod rom;

use std::env;

use abus::ABus;
use cpu::W65C816S;
use debugger::Debugger;

fn main() {
    let rom_path = env::args().nth(1).expect("No rom defined");
    let mut abus = ABus::new(&rom_path);
    let mut cpu = W65C816S::new(&mut abus);
    let mut debugger = Debugger::new();
    // Hack past the apu check in elix_nu
    mmap::apu_write8(&mut abus, 0x00, 0xAA);
    mmap::apu_write8(&mut abus, 0x01, 0xBB);
    loop {
        if debugger.quit {
            break;
        } else if debugger.active {
            debugger.take_command(&mut cpu, &mut abus);
        } else {
            while cpu.current_address() != debugger.breakpoint {
                cpu.step(&mut abus);
            }
            debugger.active = true;
        }
    }
}

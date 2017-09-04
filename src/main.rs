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

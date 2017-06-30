use std::env;

mod abus;
use abus::ABus;
mod cpu;
use cpu::Cpu;

fn main() {
    let rom_path = env::args().nth(1).expect("No rom defined");
    let mut abus = ABus::new(&rom_path);
    let mut cpu = Cpu::new(&mut abus);
    loop {
        cpu.step(&mut abus);
    }
}

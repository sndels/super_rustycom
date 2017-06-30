use std::env;

mod abus;
use abus::ABus;
mod cpu;
use cpu::Cpu;
mod debugger;
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
            while cpu.get_pb_pc() != debugger.breakpoint {
                cpu.step(&mut abus);
            }
            debugger.active = true;
        }
    }
}

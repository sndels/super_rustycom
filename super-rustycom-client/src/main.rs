extern crate super_rustycom_core;

mod debugger;

use std::env;

use super_rustycom_core::abus::ABus;
use super_rustycom_core::cpu::W65C816S;
use super_rustycom_core::mmap;
use debugger::Debugger;
use debugger::disassemble;

fn main() {
    let rom_path = env::args().nth(1).expect("No rom defined");
    let mut abus = ABus::new(&rom_path);
    let mut cpu = W65C816S::new(&mut abus);
    let mut debugger = Debugger::new();
    // Hack past the apu check in elix_nu
    abus.apu_write8(0x00, 0xAA);
    abus.apu_write8(0x01, 0xBB);
    loop {
        if debugger.quit {
            break;
        } else if debugger.active {
            debugger.take_command(&mut cpu, &mut abus);
        } else {
            if cpu.current_address() != debugger.breakpoint {
                cpu.step(&mut abus); // Step before loop to skip redundant disassembly
                while cpu.current_address() != debugger.breakpoint {
                    if debugger.disassemble {
                        disassemble(cpu.current_address(), &cpu, &mut abus)
                    }
                    cpu.step(&mut abus);
                }
            }
            debugger.active = true;
        }
    }
}

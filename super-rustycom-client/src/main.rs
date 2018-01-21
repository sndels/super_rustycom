extern crate super_rustycom_core;

mod debugger;

use std::env;
use std::fs::File;
use std::io::prelude::*;

use super_rustycom_core::abus::ABus;
use super_rustycom_core::cpu::W65C816S;
use super_rustycom_core::mmap;
use debugger::Debugger;
use debugger::disassemble_current;

fn main() {
    // Get ROM path from first argument
    let rom_path = env::args().nth(1).expect("No rom defined");

    // Load ROM from file
    let mut rom_file = File::open(&rom_path).expect("Opening rom failed");
    let mut rom_bytes = Vec::new();
    let read_bytes = rom_file
        .read_to_end(&mut rom_bytes)
        .expect("Reading rom to bytes failed");
    println!("Read {} bytes from {}", read_bytes, rom_path);

    // Init hardware
    let mut abus = ABus::new(rom_bytes);
    let mut cpu = W65C816S::new(&mut abus);
    let mut debugger = Debugger::new();

    // Hack past the apu check in elix_nu
    mmap::apu_write8(&mut abus, 0x00, 0xAA);
    mmap::apu_write8(&mut abus, 0x01, 0xBB);

    // Run
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
                        disassemble_current(&cpu, &mut abus)
                    }
                    cpu.step(&mut abus);
                }
            }
            debugger.active = true;
        }
    }
}

extern crate super_rustycom_core;

mod debugger;
mod time_source;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::thread;
use std::time;

use super_rustycom_core::abus::ABus;
use super_rustycom_core::cpu::W65C816S;
use super_rustycom_core::mmap;
use debugger::{Debugger, DebugState};
use debugger::disassemble_current;
use time_source::TimeSource;

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
    abus.apu_write8(0x00, 0xAA);
    abus.apu_write8(0x01, 0xBB);

    // Init time source
    let time_source = TimeSource::new();
    let mut emulated_cpu_cycles = 0;

    // Run
    loop {
        // Update time
        // TODO: Final timing in executed cpu cycles inside the emu struct (?), so keep unspent
        // clock cycles in mind
        let current_ns = time_source.elapsed_ns();

        // Calculate clock pulses, cpu cycles that should have passed
        let mut cpu_cycles = get_num_cpu_cycles(current_ns);
        let mut diff_cycles = (cpu_cycles - emulated_cpu_cycles) as i64;

        // Handle debugger state
        match debugger.state {
            DebugState::Active => {
                debugger.take_command(&mut cpu, &mut abus);
                // Update cycle count to prevent warping
                cpu_cycles = get_num_cpu_cycles(time_source.elapsed_ns());
                diff_cycles = 0;
            }
            DebugState::Step => {
                // Go through steps
                for _ in 0..debugger.steps {
                    if debugger.disassemble {
                        disassemble_current(&cpu, &mut abus)
                    }
                    cpu.step(&mut abus);
                }
                // Reset debugger state
                debugger.steps = 0;
                debugger.state = DebugState::Active;
                // Update cycle count to prevent warping
                cpu_cycles = get_num_cpu_cycles(time_source.elapsed_ns());
                diff_cycles = 0;
            }
            DebugState::Run => while diff_cycles > 0 {
                if cpu.current_address() != debugger.breakpoint {
                    if debugger.disassemble {
                        disassemble_current(&cpu, &mut abus)
                    }
                    diff_cycles -= cpu.step(&mut abus) as i64;
                } else {
                    debugger.state = DebugState::Active;
                    break;
                }
            },
            DebugState::Quit => break,
        }
        // Update emulated cycles and take overshoot into account
        emulated_cpu_cycles = cpu_cycles + diff_cycles.abs() as u64;
        thread::sleep(time::Duration::from_millis(2));
    }
}

fn get_num_cpu_cycles(elapsed_ns: u64) -> u64 {
    let pulses = elapsed_ns / 47;
    pulses / 8 // SlowROM(?)
}

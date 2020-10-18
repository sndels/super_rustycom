use crate::abus::ABus;
use crate::cpu::W65C816S;

/// Abstraction around the actual emu implementation
pub struct SNES {
    pub abus: ABus,
    pub cpu: W65C816S,
}

impl SNES {
    /// Initializes new instance with given ROM
    pub fn new(rom_bytes: Vec<u8>) -> SNES {
        let mut abus = ABus::new(rom_bytes);
        SNES {
            cpu: W65C816S::new(&mut abus),
            abus: abus,
        }
    }

    /// Runs the hardware for given number of ticks and returns actual ticks emulated and wheter
    /// or not a breakpoint was hit
    pub fn run(
        &mut self,
        clock_ticks: u64,
        breakpoint: u32,
        disassemble: bool,
        da_func: fn(&W65C816S, &mut ABus),
    ) -> (u64, bool) {
        let target_cpu_cycles = clock_ticks / 8; // SlowROM (?)
        let mut cpu_cycles = 0;
        let mut hit_breakpoint = false;
        while cpu_cycles < target_cpu_cycles {
            if self.cpu.current_address() != breakpoint {
                if disassemble {
                    da_func(&self.cpu, &mut self.abus);
                }
                cpu_cycles += self.cpu.step(&mut self.abus) as u64;
            } else {
                hit_breakpoint = true;
                break;
            }
        }
        (cpu_cycles * 8, hit_breakpoint) // SlowROM
    }

    /// Runs the hardware for given number instructions
    pub fn run_steps(
        &mut self,
        instructions: u32,
        disassemble: bool,
        da_func: fn(&W65C816S, &mut ABus),
    ) {
        for _ in 0..instructions {
            if disassemble {
                da_func(&self.cpu, &mut self.abus)
            }
            self.cpu.step(&mut self.abus);
        }
    }
}

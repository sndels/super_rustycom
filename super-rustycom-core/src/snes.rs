use crate::abus::ABus;
use crate::apu::APU;
use crate::cpu::W65C816S;

/// Abstraction around the actual emu implementation
pub struct SNES {
    pub abus: ABus,
    pub cpu: W65C816S,
    pub apu: APU,
}

impl SNES {
    /// Initializes new instance with given ROM
    pub fn new(rom_bytes: Vec<u8>) -> SNES {
        let mut abus = ABus::new(rom_bytes);
        SNES {
            cpu: W65C816S::new(&mut abus),
            abus: abus,
            apu: APU::new(),
        }
    }

    /// Runs the hardware for given number of ticks and returns actual ticks emulated and wheter
    /// or not a breakpoint was hit
    pub fn run<F>(
        &mut self,
        clock_ticks: u128,
        breakpoint: u32,
        mut disassemble_func: F,
    ) -> (u128, bool)
    where
        F: FnMut(&W65C816S, &mut ABus, usize),
    {
        let ticks_per_cycle = 8;
        let target_cpu_cycles = clock_ticks / ticks_per_cycle; // SlowROM (?)
        let mut cpu_cycles = 0;
        let mut hit_breakpoint = false;
        while cpu_cycles < target_cpu_cycles {
            if self.cpu.current_address() != breakpoint {
                disassemble_func(
                    &self.cpu,
                    &mut self.abus,
                    (target_cpu_cycles - cpu_cycles) as usize,
                );
                cpu_cycles += self.cpu.step(&mut self.abus) as u128;
                let (_, apu_io) = self.apu.step(self.abus.apu_io());
                self.abus.copy_smp_io(apu_io);
            } else {
                hit_breakpoint = true;
                break;
            }
        }
        (cpu_cycles * ticks_per_cycle, hit_breakpoint) // SlowROM
    }

    /// Runs the hardware for given number instructions
    pub fn run_steps<F>(&mut self, instructions: u32, mut disassemble_func: F)
    where
        F: FnMut(&W65C816S, &mut ABus),
    {
        for _ in 0..instructions {
            disassemble_func(&self.cpu, &mut self.abus);
            self.cpu.step(&mut self.abus);
        }
    }
}

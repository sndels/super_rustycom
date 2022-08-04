use crate::abus::ABus;
use crate::apu::Apu;
use crate::cpu::W65c816s;

/// Abstraction around the actual emu implementation
pub struct Snes {
    pub abus: ABus,
    pub cpu: W65c816s,
    pub apu: Apu,
}

impl Snes {
    /// Initializes new instance with given ROM
    pub fn new(rom_bytes: Vec<u8>) -> Snes {
        let mut abus = ABus::new(rom_bytes);
        Snes {
            cpu: W65c816s::new(&mut abus),
            abus,
            apu: Apu::default(),
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
        F: FnMut(&W65c816s, &mut ABus),
    {
        let ticks_per_cycle = 8;
        let target_cpu_cycles = clock_ticks / ticks_per_cycle; // SlowROM (?)
        let mut cpu_cycles = 0;
        let mut hit_breakpoint = false;
        while cpu_cycles < target_cpu_cycles {
            if self.cpu.current_address() != breakpoint {
                disassemble_func(&self.cpu, &mut self.abus);
                cpu_cycles += self.cpu.step(&mut self.abus) as u128;
                let (_, apu_io) = self.apu.step(self.abus.apu_io());
                self.abus.write_smp_io(apu_io);
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
        F: FnMut(&W65c816s, &mut ABus),
    {
        for _ in 0..instructions {
            disassemble_func(&self.cpu, &mut self.abus);
            self.cpu.step(&mut self.abus);
            let (_, apu_io) = self.apu.step(self.abus.apu_io());
            self.abus.write_smp_io(apu_io);
        }
    }
}

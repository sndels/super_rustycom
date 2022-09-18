use crate::abus::ABus;
use crate::apu::Apu;
use crate::cpu::W65c816s;

/// Abstraction around the actual emu implementation
pub struct Snes {
    pub abus: ABus,
    pub cpu: W65c816s,
    pub apu: Apu,
    rom_bytes: Vec<u8>,
}

impl Snes {
    /// Initializes new instance with given ROM
    pub fn new(rom_bytes: Vec<u8>) -> Snes {
        let mut abus = ABus::new(rom_bytes.clone());
        Snes {
            cpu: W65c816s::new(&mut abus),
            abus,
            apu: Apu::default(),
            rom_bytes,
        }
    }

    /// Performs a full reset (off and on again)
    pub fn reset(&mut self) {
        *self = Snes::new(self.rom_bytes.clone());
    }

    /// Runs the hardware for given number of ticks and returns actual ticks emulated and wheter
    /// or not a breakpoint was hit
    pub fn run<F>(
        &mut self,
        clock_ticks: u128,
        breakpoint: u32,
        disassemble_func: &mut F,
    ) -> (u128, bool)
    where
        F: FnMut(&W65c816s, &mut ABus),
    {
        let mut emulated_ticks = 0;
        let mut hit_breakpoint = false;
        while emulated_ticks < clock_ticks {
            if emulated_ticks == 0 || self.cpu.current_address() != breakpoint {
                disassemble_func(&self.cpu, &mut self.abus);
                emulated_ticks += self.cpu.step(&mut self.abus) as u128;
                // TODO:
                // APU might run ahead or fall behind noticeably?
                let (_, apu_io) = self.apu.step(self.abus.apu_io());
                self.abus.write_smp_io(apu_io);
            } else {
                hit_breakpoint = true;
                break;
            }
        }
        (emulated_ticks, hit_breakpoint)
    }

    /// Runs the hardware for given number instructions
    pub fn run_steps<F>(&mut self, instructions: u32, disassemble_func: &mut F)
    where
        F: FnMut(&W65c816s, &mut ABus),
    {
        for _ in 0..instructions {
            self.run(1, 0, disassemble_func);
        }
    }
}

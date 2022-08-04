pub mod bus;
pub mod smp;

use self::bus::Bus;
use self::smp::Spc700;
use super::apu_io::ApuIo;

pub struct Apu {
    pub smp: Spc700,
    pub bus: Bus,
}

impl Apu {
    pub fn default() -> Apu {
        Apu {
            smp: Spc700::default(),
            bus: Bus::default(),
        }
    }

    /// Steps the entire APU and returns SMP cycles evaluated as well as the state of the IO ports
    pub fn step(&mut self, io: ApuIo) -> (Option<u8>, ApuIo) {
        self.bus.write_cpu_io(io);
        let cycles = self.smp.step(&mut self.bus);
        let io = self.bus.apu_io();
        (cycles, io)
    }
}

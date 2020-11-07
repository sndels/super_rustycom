pub mod bus;
pub mod smp;

use self::bus::Bus;
use self::smp::SPC700;
use super::apu_io::ApuIo;

pub struct APU {
    pub smp: SPC700,
    pub bus: Bus,
}

impl APU {
    pub fn new() -> APU {
        APU {
            smp: SPC700::new(),
            bus: Bus::new(),
        }
    }

    /// Steps the entire APU and returns SMP cycles evaluated as well as the state of the IO ports
    pub fn step(&mut self, io: ApuIo) -> (u8, ApuIo) {
        self.bus.write_cpu_io(io);
        let cycles = self.smp.step(&mut self.bus);
        let io = self.bus.apu_io();
        (cycles, io)
    }
}

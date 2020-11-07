/// CPU<->APU communication
/// CPU writes to low bits and reads from high, APU vice versa
#[derive(Clone, Copy)]
pub struct ApuIo {
    port0: u16,
    port1: u16,
    port2: u16,
    port3: u16,
}

impl ApuIo {
    pub fn new() -> ApuIo {
        ApuIo {
            port0: 0x0000,
            port1: 0x0000,
            port2: 0x0000,
            port3: 0x0000,
        }
    }

    pub fn copy_cpu(&mut self, other: &ApuIo) {
        macro_rules! copy {
            ($port:ident) => {
                self.$port = (other.$port & 0xFF00) | (self.$port & 0x00FF)
            };
        }
        copy!(port0);
        copy!(port1);
        copy!(port2);
        copy!(port3);
    }

    pub fn copy_smp(&mut self, other: &ApuIo) {
        macro_rules! copy {
            ($port:ident) => {
                self.$port = (self.$port & 0xFF00) | (other.$port & 0x00FF)
            };
        }
        copy!(port0);
        copy!(port1);
        copy!(port2);
        copy!(port3);
    }

    pub fn cpu_write(&mut self, port: usize, value: u8) {
        macro_rules! write {
            ($port:expr) => {
                $port = ($port & 0xFF00) | value as u16
            };
        }
        match port {
            0x00 => write!(self.port0),
            0x01 => write!(self.port1),
            0x02 => write!(self.port2),
            0x03 => write!(self.port3),
            _ => unreachable!(),
        }
    }

    pub fn cpu_read(&self, port: usize) -> u8 {
        match port {
            0x00 => (self.port0 >> 8) as u8,
            0x01 => (self.port1 >> 8) as u8,
            0x02 => (self.port2 >> 8) as u8,
            0x03 => (self.port3 >> 8) as u8,
            _ => unreachable!(),
        }
    }

    pub fn smp_write(&mut self, port: usize, value: u8) {
        macro_rules! write {
            ($port:expr) => {
                $port = ((value as u16) << 8) | ($port & 0x00FF)
            };
        }
        match port {
            0x00 => write!(self.port0),
            0x01 => write!(self.port1),
            0x02 => write!(self.port2),
            0x03 => write!(self.port3),
            _ => unreachable!(),
        }
    }

    pub fn smp_read(&self, port: usize) -> u8 {
        match port {
            0x00 => self.port0 as u8,
            0x01 => self.port1 as u8,
            0x02 => self.port2 as u8,
            0x03 => self.port3 as u8,
            _ => unreachable!(),
        }
    }
}

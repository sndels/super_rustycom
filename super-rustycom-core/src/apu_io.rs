use log::error;

/// CPU<->APU communication
/// CPU writes to low bits and reads from high, APU vice versa
#[derive(Clone, Copy)]
pub struct ApuIo {
    port0: u8,
    port1: u8,
    port2: u8,
    port3: u8,
}

impl ApuIo {
    pub fn new(port0: u8, port1: u8, port2: u8, port3: u8) -> ApuIo {
        ApuIo {
            port0,
            port1,
            port2,
            port3,
        }
    }

    pub fn default() -> ApuIo {
        ApuIo {
            port0: 0x00,
            port1: 0x00,
            port2: 0x00,
            port3: 0x00,
        }
    }

    pub fn write(&mut self, port: u8, value: u8) {
        match port {
            0x00 => self.port0 = value,
            0x01 => self.port1 = value,
            0x02 => self.port2 = value,
            0x03 => self.port3 = value,
            _ => error!("Invalid APU IO port {}", port),
        }
    }

    pub fn read(&self, port: u8) -> u8 {
        match port {
            0x00 => self.port0,
            0x01 => self.port1,
            0x02 => self.port2,
            0x03 => self.port3,
            _ => {
                error!("Invalid APU IO port {}", port);
                0
            }
        }
    }
}

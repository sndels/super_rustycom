/// 64k bytes of video memory
const VRAM_SIZE: usize = 64 * 1024;

pub struct Vram {
    vmain: IncrementMode,
    vmadd: u16,
    prefetch: u16,
    mem: Box<[u8]>,
}

struct IncrementMode {
    byte: Byte,
    translation: Translation,
    step: u16,
}

impl Default for IncrementMode {
    fn default() -> Self {
        Self {
            byte: Byte::Low,
            translation: Translation::None,
            step: 1,
        }
    }
}

enum Translation {
    None,
    _8bit,
    _9bit,
    _10bit,
}

enum Byte {
    Low,
    High,
}

impl Vram {
    pub fn mem(&self) -> &[u8] {
        &self.mem
    }

    pub fn write_vmain(&mut self, value: u8) {
        self.vmain.byte = match value >> 7 {
            0 => Byte::Low,
            1 => Byte::High,
            _ => unreachable!(),
        };
        self.vmain.translation = match value & 0b1100 >> 2 {
            0 => Translation::None,
            1 => Translation::_8bit,
            2 => Translation::_9bit,
            3 => Translation::_10bit,
            _ => unreachable!(),
        };
        self.vmain.step = match value & 0b1100 {
            0 => 1,
            1 => 32,
            2 => 128,
            3 => 128,
            _ => unreachable!(),
        };
    }

    pub fn write_vmaddl(&mut self, addr: u8) {
        self.vmadd = self.vmadd & 0xFF00 | (addr as u16);
        self.prefetch()
    }

    pub fn write_vmaddh(&mut self, addr: u8) {
        self.vmadd = ((addr as u16) << 8) | self.vmadd & 0xFF;
        self.prefetch()
    }

    pub fn write_vmdatal(&mut self, value: u8) {
        let addr = self.low_byte_addr();
        self.mem[addr] = value;
        if let Byte::Low = self.vmain.byte {
            self.increment();
        }
    }

    pub fn write_vmdatah(&mut self, value: u8) {
        let addr = self.low_byte_addr();
        self.mem[addr + 1] = value;
        if let Byte::High = self.vmain.byte {
            self.increment();
        }
    }

    pub fn read_low(&mut self) -> u8 {
        let value = self.peek_low();
        if let Byte::Low = self.vmain.byte {
            self.prefetch();
            self.increment();
        }
        value
    }

    pub fn read_high(&mut self) -> u8 {
        let value = self.peek_high();
        if let Byte::High = self.vmain.byte {
            self.prefetch();
            self.increment();
        }
        value
    }

    pub fn peek_low(&self) -> u8 {
        self.prefetch as u8
    }

    pub fn peek_high(&self) -> u8 {
        (self.prefetch & 0xFF00 >> 8) as u8
    }

    pub fn increment(&mut self) {
        self.vmadd = self.vmadd.wrapping_add(self.vmain.step);
    }

    fn low_byte_addr(&self) -> usize {
        let word_addr = match self.vmain.translation {
            Translation::None => self.vmadd,
            Translation::_8bit => {
                let high_addr = self.vmadd & 0xFF00;
                let low_addr = self.vmadd & 0xFF;
                high_addr | (low_addr << 3) | ((low_addr & 0b1110_0000) >> 5)
            }
            Translation::_9bit => {
                let high_addr = self.vmadd & 0xFE00;
                let low_addr = self.vmadd & 0x1FF;
                high_addr | (low_addr << 3) | ((low_addr & 0b1_1100_0000) >> 6)
            }
            Translation::_10bit => {
                let high_addr = self.vmadd & 0xFC00;
                let low_addr = self.vmadd & 0x3FF;
                high_addr | (low_addr << 3) | ((low_addr & 0b11_1000_0000) >> 6)
            }
        };
        (word_addr as usize) << 1
    }

    fn prefetch(&mut self) {
        let addr = self.low_byte_addr();
        self.prefetch = ((self.mem[addr + 1] as u16) << 8) | (self.mem[addr] as u16);
        // TODO: Timing?
    }
}

impl Default for Vram {
    fn default() -> Self {
        Self {
            vmain: IncrementMode::default(),
            vmadd: 0,
            prefetch: 0x0,
            mem: Box::new([0; VRAM_SIZE]),
        }
    }
}

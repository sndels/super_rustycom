#[derive(Copy, Clone)]
struct DoubleReg {
    value: u16,
    high_active: bool, // TODO: Should there be separate flags for read and write?
}

impl DoubleReg {
    pub fn new() -> DoubleReg {
        DoubleReg {
            value:         0,
            high_active: false,
        }
    }

    pub fn read(&mut self) -> u8 {
        let value = if self.high_active { (self.value >> 8) as u8 }
                    else { (self.value & 0xFF) as u8 };
        self.high_active = !self.high_active;
        value
    }

    pub fn write(&mut self, value: u8) {
        if self.high_active {
            self.value = (self.value & 0x00FF) | ((value as u16) << 8);
        } else {
            self.value = (self.value & 0xFF00) | value as u16;
        }
        self.high_active = !self.high_active;
    }
}

pub struct PpuIo {
    ports:    [u8; 64],
    bg_ofs:   [DoubleReg; 8],
    m7:       [DoubleReg; 6],
    cg_data:  DoubleReg,
    rd_oam:   DoubleReg,
    rd_cgram: DoubleReg,
    op_hct:   DoubleReg,
    op_vct:   DoubleReg,
}

impl PpuIo {
    // TODO: Randomize memory?
    // TODO: Implement write-/read-twice regs
    // Initial values from fullsnes
    pub fn new() -> PpuIo {
        let mut ppu_io: PpuIo = PpuIo {
            ports:    [0; 64],
            bg_ofs:   [DoubleReg::new(); 8],
            m7:       [DoubleReg::new(); 6],
            cg_data:  DoubleReg::new(),
            rd_oam:   DoubleReg::new(),
            rd_cgram: DoubleReg::new(),
            op_hct:   DoubleReg::new(),
            op_vct:   DoubleReg::new(),
        };
        ppu_io.ports[0x00]  = 0x08;
        ppu_io.ports[0x05]  = 0x0F;
        ppu_io.ports[0x15] |= 0xF;
        ppu_io.m7[0].write(0xFF);
        ppu_io.m7[0].write(0x00);
        ppu_io.m7[1].write(0xFF);
        ppu_io.m7[1].write(0x00);
        ppu_io.ports[0x33] = 0x00;
        ppu_io.ports[0x34] = 0x01;
        ppu_io.ports[0x35] = 0x00;
        ppu_io.ports[0x36] = 0x00;
        ppu_io.op_hct.write(0xFF);
        ppu_io.op_hct.write(0x01);
        ppu_io.op_vct.write(0xFF);
        ppu_io.op_vct.write(0x01);
        ppu_io.ports[0x3F] |= 0b0100_000;
        ppu_io
    }

    pub fn cpu_write(&mut self, value: u8, addr: u8) {
        // Panic if accessing read-only regs
        if addr > 0x33 {
            panic!("Cpu tried writing to read-only port at {:x}", addr);
        }

        // Match on write-twice regs, default to reg array
        // TODO: Enum to int matching if it gets back to rust
        match addr {
            0x0D => self.bg_ofs[0].write(value),
            0x0E => self.bg_ofs[1].write(value),
            0x0F => self.bg_ofs[2].write(value),
            0x10 => self.bg_ofs[3].write(value),
            0x11 => self.bg_ofs[4].write(value),
            0x12 => self.bg_ofs[5].write(value),
            0x13 => self.bg_ofs[6].write(value),
            0x14 => self.bg_ofs[7].write(value),
            0x1B => self.m7[0].write(value),
            0x1C => self.m7[1].write(value),
            0x1D => self.m7[2].write(value),
            0x1E => self.m7[3].write(value),
            0x1F => self.m7[4].write(value),
            0x20 => self.m7[5].write(value),
            0x22 => self.cg_data.write(value),
            _    => self.ports[addr as usize] = value
        }
    }

    pub fn cpu_read(&mut self, addr: u8) -> u8 {
        // Panic if accessing write-only regs
        if addr < 0x34 {
            panic!("Cpu tried reading from write-only port at {:x}", addr);
        }

        // Match on read-twice regs, default to reg array
        // TODO: Enum to int matching if it gets back to rust
        match addr {
            0x38 => self.rd_oam.read(),
            0x3B => self.rd_cgram.read(),
            0x3C => self.op_hct.read(),
            0x3D => self.op_vct.read(),
            _ => self.ports[addr as usize]
        }
    }
}

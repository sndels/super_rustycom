mod rom;
use self::rom::Rom;
mod mmap;

pub struct ABus {
    wram:     [u8; 131072],
    vram:     [u8; 65536],
    oam:      [u8; 544],
    cgram:    [u8; 512],
    rom:      Rom,
    mpy_div:  MpyDiv,
    // PPU IO
    ppu_io:   [u8; 64],
    bg_ofs:   [DoubleReg; 8],
    m7:       [DoubleReg; 6],
    cg_data:  DoubleReg,
    rd_oam:   DoubleReg,
    rd_cgram: DoubleReg,
    op_hct:   DoubleReg,
    op_vct:   DoubleReg,
    // "On-chip" CPU W
    nmitimen: u8,
    wrio:     u8,
    htime:    u16,
    vtime:    u16,
    mdmaen:   u8,
    hdmaen:   u8,
    memsel:   u8,
    // APU comms
    apu_ports: [u8; 4],
    // TODO: PPU1,2
    // TODO: PPU control regs
    // TODO: Joypad
    // TODO: Math regs
    // TODO: H-/V-blank regs and timers
    // TODO: DMA + control regs
}

impl ABus {
    // TODO: Randomize values?
    pub fn new(rom_path: &String) -> ABus {
        let mut abus: ABus = ABus {
            wram:     [0; 131072],
            vram:     [0; 65536],
            oam:      [0; 544],
            cgram:    [0; 512],
            rom:      Rom::new(rom_path),
            mpy_div:  MpyDiv::new(),
            ppu_io:   [0; 64],
            bg_ofs:   [DoubleReg::new(); 8],
            m7:       [DoubleReg::new(); 6],
            cg_data:  DoubleReg::new(),
            rd_oam:   DoubleReg::new(),
            rd_cgram: DoubleReg::new(),
            op_hct:   DoubleReg::new(),
            op_vct:   DoubleReg::new(),
            nmitimen: 0x00,
            wrio:     0xFF,
            htime:    0x01FF,
            vtime:    0x01FF,
            mdmaen:   0x00,
            hdmaen:   0x00,
            memsel:   0x00,
            apu_ports: [0; 4],
        };
        abus.ppu_io[mmap::INIDISP] = 0x08;
        abus.ppu_io[mmap::BGMODE]  = 0x0F;
        abus.ppu_io[mmap::VMAIN]  |= 0xF;
        abus.m7[0].write(0xFF);
        abus.m7[0].write(0x00);
        abus.m7[1].write(0xFF);
        abus.m7[1].write(0x00);
        abus.ppu_io[mmap::SETINI] = 0x00;
        abus.ppu_io[mmap::MPYL]   = 0x01;
        abus.ppu_io[mmap::MPYM]   = 0x00;
        abus.ppu_io[mmap::MPYH]   = 0x00;
        abus.op_hct.write(0xFF);
        abus.op_hct.write(0x01);
        abus.op_vct.write(0xFF);
        abus.op_vct.write(0x01);
        abus.ppu_io[mmap::STAT78] |= 0b0100_000;
        abus
    }

    #[cfg(test)]
    pub fn new_empty_rom() -> ABus {
        let mut abus: ABus = ABus {
            wram:     [0; 131072],
            vram:     [0; 65536],
            oam:      [0; 544],
            cgram:    [0; 512],
            rom:      Rom::new_empty(),
            mpy_div:  MpyDiv::new(),
            ppu_io:   [0; 64],
            bg_ofs:   [DoubleReg::new(); 8],
            m7:       [DoubleReg::new(); 6],
            cg_data:  DoubleReg::new(),
            rd_oam:   DoubleReg::new(),
            rd_cgram: DoubleReg::new(),
            op_hct:   DoubleReg::new(),
            op_vct:   DoubleReg::new(),
            nmitimen: 0x00,
            wrio:     0xFF,
            htime:    0x01FF,
            vtime:    0x01FF,
            mdmaen:   0x00,
            hdmaen:   0x00,
            memsel:   0x00,
            apu_ports: [0; 4],
        };
        abus.ppu_io[mmap::INIDISP] = 0x08;
        abus.ppu_io[mmap::BGMODE]  = 0x0F;
        abus.ppu_io[mmap::VMAIN]  |= 0xF;
        abus.m7[0].write(0xFF);
        abus.m7[0].write(0x00);
        abus.m7[1].write(0xFF);
        abus.m7[1].write(0x00);
        abus.ppu_io[mmap::SETINI] = 0x00;
        abus.ppu_io[mmap::MPYL]   = 0x01;
        abus.ppu_io[mmap::MPYM]   = 0x00;
        abus.ppu_io[mmap::MPYH]   = 0x00;
        abus.op_hct.write(0xFF);
        abus.op_hct.write(0x01);
        abus.op_vct.write(0xFF);
        abus.op_vct.write(0x01);
        abus.ppu_io[mmap::STAT78] |= 0b0100_000;
        abus
    }

    pub fn cpu_read_8(&mut self, addr: u32) -> u8 {
        // TODO: Only LoROM, WRAM implemented, under 0x040000
        // TODO: LUT for speed?
        let bank = (addr >> 16) as usize;
        let bank_addr = (addr & 0x00FFFF) as usize;
        match bank {
            mmap::WS1_SYSLR_FIRST_BANK...mmap::WS1_SYSLR_LAST_BANK => {
                match bank_addr {
                    mmap::WRAM_MIRR_FIRST...mmap::WRAM_MIRR_LAST => {
                        self.wram[bank_addr]
                    }
                    mmap::PPU_IO_FIRST...mmap::PPU_IO_LAST => { // PPU IO
                        let offset = bank_addr - mmap::PPU_IO_FIRST;
                        if offset < mmap::MPYL {
                            panic!("Read {:#01$x}: Address write-only", addr, 8);
                        }

                        // Match on read-twice regs, default to reg array
                        match offset {
                            mmap::RDOAM   => self.rd_oam.read(),
                            mmap::RDCGRAM => self.rd_cgram.read(),
                            mmap::OPHCT   => self.op_hct.read(),
                            mmap::OPVCT   => self.op_vct.read(),
                            _       => self.ppu_io[offset]
                        }
                    }
                    mmap::APU_IO_FIRST...mmap::APU_IO_LAST => { // APU IO
                        if bank_addr < 0x2144 {
                            self.apu_ports[bank_addr - 0x2140]
                        } else {
                            self.apu_ports[bank_addr - 0x2144]
                        }
                    }
                    mmap::WMDATA => panic!("Read {:#01$x}: WMDATA not implemented", addr, 8),
                    mmap::JOYA   => panic!("Read {:#01$x}: JOYA not implemented", addr, 8),
                    mmap::JOYB   => panic!("Read {:#01$x}: JOYA not implemented", addr, 8),
                    mmap::RDNMI  => panic!("Read {:#01$x}: RDNMI not implemented", addr, 8),
                    mmap::TIMEUP => panic!("Read {:#01$x}: TIMEUP not implemented", addr, 8),
                    mmap::HVBJOY => panic!("Read {:#01$x}: HVBJOY not implemented", addr, 8),
                    mmap::RDIO   => panic!("Read {:#01$x}: RDIO not implemented", addr, 8),
                    mmap::RDDIVL => panic!("Read {:#01$x}: RDDIVL not implemented", addr, 8),
                    mmap::RDDIVH => panic!("Read {:#01$x}: RDDIVH not implemented", addr, 8),
                    mmap::RDMPYL => panic!("Read {:#01$x}: RDMPYL not implemented", addr, 8),
                    mmap::RDMPYH => panic!("Read {:#01$x}: RDMPYH not implemented", addr, 8),
                    mmap::JOY1L  => panic!("Read {:#01$x}: JOY1L not implemented", addr, 8),
                    mmap::JOY1H  => panic!("Read {:#01$x}: JOY1H not implemented", addr, 8),
                    mmap::JOY2L  => panic!("Read {:#01$x}: JOY2L not implemented", addr, 8),
                    mmap::JOY2H  => panic!("Read {:#01$x}: JOY2H not implemented", addr, 8),
                    mmap::JOY3L  => panic!("Read {:#01$x}: JOY3L not implemented", addr, 8),
                    mmap::JOY3H  => panic!("Read {:#01$x}: JOY3H not implemented", addr, 8),
                    mmap::JOY4L  => panic!("Read {:#01$x}: JOY4L not implemented", addr, 8),
                    mmap::JOY4H  => panic!("Read {:#01$x}: JOY4H not implemented", addr, 8),
                    mmap::DMA_FIRST...mmap::DMA_LAST => { // DMA
                        panic!("Read {:#01$x}: DMA not implemented", addr, 8)
                    }
                    mmap::EXP_FIRST...mmap::EXP_LAST => { // Expansion
                        panic!("Read {:#01$x}: Expansion not implemented", addr, 8)
                    }
                    mmap::LOROM_FIRST...mmap::LOROM_LAST => {
                        let offset = bank_addr - mmap::LOROM_FIRST;
                        self.rom.read_8(bank * mmap::LOROM_FIRST + offset)
                    }
                    _ => panic!("Read {:#01$x}: Address unused or write-only", addr, 8)
                }
            }
            mmap::WRAM_FIRST_BANK...mmap::WRAM_LAST_BANK => {
                self.wram[(bank - mmap::WRAM_FIRST_BANK) * 0x10000 + bank_addr]
            }
            _ => panic!("Read {:#01$x}: Bank not implemented!", addr, 8)
        }
    }

    pub fn cpu_read_16(&mut self, addr: u32) -> u16 {
        self.cpu_read_8(addr) as u16 | ((self.cpu_read_8(addr + 1) as u16) << 8)
    }

    pub fn bank_wrapping_cpu_read_16(&mut self, addr: u32) -> u16 {
        self.cpu_read_8(addr) as u16 |
        ((self.cpu_read_8(bank_wrapping_add(addr, 1)) as u16) << 8)
    }

    pub fn bank_wrapping_cpu_read_24(&mut self, addr: u32) -> u32 {
        self.cpu_read_8(addr) as u32 |
        ((self.cpu_read_8(bank_wrapping_add(addr, 1)) as u32) << 8) |
        ((self.cpu_read_8(bank_wrapping_add(addr, 2)) as u32) << 16)
    }

    pub fn page_wrapping_cpu_read_16(&mut self, addr: u32) -> u16 {
        self.cpu_read_8(addr) as u16 |
        ((self.cpu_read_8(page_wrapping_add(addr, 1)) as u16) << 8)
    }

    pub fn page_wrapping_cpu_read_24(&mut self, addr: u32) -> u32 {
        self.cpu_read_8(addr) as u32 |
        ((self.cpu_read_8(page_wrapping_add(addr, 1)) as u32) << 8) |
        ((self.cpu_read_8(page_wrapping_add(addr, 2)) as u32) << 16)
    }

    pub fn fetch_operand_8(&mut self, addr: u32) -> u8 {
        self.cpu_read_8(bank_wrapping_add(addr, 1))
    }

    pub fn fetch_operand_16(&mut self, addr: u32) -> u16 {
        self.bank_wrapping_cpu_read_16(bank_wrapping_add(addr, 1))
    }

    pub fn fetch_operand_24(&mut self, addr: u32) -> u32 {
        self.bank_wrapping_cpu_read_24(bank_wrapping_add(addr, 1))
    }

    pub fn cpu_write_8(&mut self, value: u8, addr: u32) {
        let bank = (addr >> 16) as usize;
        let bank_addr = (addr & 0x00FFFF) as usize;
        match bank {
            mmap::WS1_SYSLR_FIRST_BANK...mmap::WS1_SYSLR_LAST_BANK => {
                match bank_addr {
                    mmap::WRAM_MIRR_FIRST...mmap::WRAM_MIRR_LAST => { // WRAM
                        self.wram[bank_addr] = value
                    }
                    mmap::PPU_IO_FIRST...mmap::PPU_IO_LAST => { // PPU IO
                        let offset = bank_addr - mmap::PPU_IO_FIRST;
                        if offset > mmap::SETINI {
                            panic!("Write {:#01$x}: Address read-only", addr, 8);
                        }

                        // Match on write-twice regs, default to reg array
                        match offset {
                            mmap::BG1HOFS => self.bg_ofs[0].write(value),
                            mmap::BG1VOFS => self.bg_ofs[1].write(value),
                            mmap::BG2HOFS => self.bg_ofs[2].write(value),
                            mmap::BG2VOFS => self.bg_ofs[3].write(value),
                            mmap::BG3HOFS => self.bg_ofs[4].write(value),
                            mmap::BG3VOFS => self.bg_ofs[5].write(value),
                            mmap::BG4HOFS => self.bg_ofs[6].write(value),
                            mmap::BG4VOFS => self.bg_ofs[7].write(value),
                            mmap::M7A     => self.m7[0].write(value),
                            mmap::M7B     => self.m7[1].write(value),
                            mmap::M7C     => self.m7[2].write(value),
                            mmap::M7D     => self.m7[3].write(value),
                            mmap::M7X     => self.m7[4].write(value),
                            mmap::M7Y     => self.m7[5].write(value),
                            mmap::CGDATA  => self.cg_data.write(value),
                            _       => self.ppu_io[offset] = value
                        }
                    }
                    mmap::APU_IO_FIRST...mmap::APU_IO_LAST => { // APU IO
                        if bank_addr < 0x2144 {
                            self.apu_ports[bank_addr - 0x2140] = value;
                        } else {
                            self.apu_ports[bank_addr - 0x2144] = value;
                        }
                    }
                    mmap::WMDATA   => panic!("Write {:#01$x}: WMDATA not implemented", addr, 8),
                    mmap::WMADDL   => panic!("Write {:#01$x}: WMADDL not implemented", addr, 8),
                    mmap::WMADDM   => panic!("Write {:#01$x}: WMADDM not implemented", addr, 8),
                    mmap::WMADDH   => panic!("Write {:#01$x}: WMADDH not implemented", addr, 8),
                    mmap::JOYWR    => panic!("Write {:#01$x}: JOYWR not implemented", addr, 8),
                    mmap::NMITIMEN => self.nmitimen = value,
                    mmap::WRIO     => self.wrio = value,
                    mmap::WRMPYA   => self.mpy_div.set_mpy_a(value),
                    mmap::WRMPYB   => self.mpy_div.set_mpy_b(value),
                    mmap::WRDIVL   => self.mpy_div.set_dividend_low(value),
                    mmap::WRDIVH   => self.mpy_div.set_dividend_high(value),
                    mmap::WRDIVB   => self.mpy_div.set_divisor(value),
                    mmap::HTIMEL   => self.htime = (self.htime & 0xFF00) | value as u16,
                    mmap::HTIMEH   => self.htime = ((value as u16) << 8) | (self.htime & 0x00FF),
                    mmap::VTIMEL   => self.vtime = (self.vtime & 0xFF00) | value as u16,
                    mmap::VTIMEH   => self.vtime = ((value as u16) << 8) | (self.vtime & 0x00FF),
                    mmap::MDMAEN   => self.mdmaen = value, // TODO: Start transfer
                    mmap::HDMAEN   => self.hdmaen = value,
                    mmap::MEMSEL   => self.memsel = value,
                    mmap::DMA_FIRST...mmap::DMA_LAST => { // DMA
                        panic!("Write {:#01$x}: DMA not implemented", addr, 8)
                    }
                    mmap::EXP_FIRST...mmap::EXP_LAST => { // Expansion
                        panic!("Write {:#01$x}: Expansion not implemented", addr, 8)
                    }
                    _ => panic!("Write {:#01$x}: Address unused or read-only", addr, 8)
                }
            }
            mmap::WRAM_FIRST_BANK...mmap::WRAM_LAST_BANK => {
                self.wram[(bank - mmap::WRAM_FIRST_BANK) * 0x10000 + bank_addr] = value;
            }
            _ => panic!("Write {:#01$x}: Bank not implemented", addr, 8)
        }
    }

    pub fn cpu_write_16(&mut self, value: u16, addr: u32) {
        self.cpu_write_8(value as u8, addr);
        self.cpu_write_8((value >> 8) as u8, addr + 1);
    }

    pub fn bank_wrapping_cpu_write_16(&mut self, value: u16, addr: u32) {
        self.cpu_write_8(value as u8, addr);
        self.cpu_write_8((value >> 8) as u8, bank_wrapping_add(addr, 1));
    }

    pub fn bank_wrapping_cpu_write_24(&mut self, value: u32, addr: u32) {
        self.cpu_write_8(value as u8, addr);
        self.cpu_write_8((value >> 8) as u8, bank_wrapping_add(addr, 1));
        self.cpu_write_8((value >> 16) as u8, bank_wrapping_add(addr, 2));
    }

    pub fn page_wrapping_cpu_write_16(&mut self, value: u16, addr: u32) {
        self.cpu_write_8(value as u8, addr);
        self.cpu_write_8((value >> 8) as u8, page_wrapping_add(addr, 1));
    }

    pub fn page_wrapping_cpu_write_24(&mut self, value: u32, addr: u32) {
        self.cpu_write_8(value as u8, addr);
        self.cpu_write_8((value >> 8) as u8, page_wrapping_add(addr, 1));
        self.cpu_write_8((value >> 16) as u8, page_wrapping_add(addr, 2));
    }
}

pub fn bank_wrapping_add(addr: u32, offset: u16) -> u32 {
    (addr & 0xFF0000 ) | ((addr as u16).wrapping_add(offset) as u32)
}

pub fn page_wrapping_add(addr: u32, offset: u8) -> u32 {
    (addr & 0xFFFF00 ) | ((addr as u8).wrapping_add(offset) as u32)
}

pub fn bank_wrapping_sub(addr: u32, offset: u16) -> u32 {
    (addr & 0xFF0000 ) | ((addr as u16).wrapping_sub(offset) as u32)
}

pub fn page_wrapping_sub(addr: u32, offset: u8) -> u32 {
    (addr & 0xFFFF00 ) | ((addr as u8).wrapping_sub(offset) as u32)
}

struct MpyDiv {
    mpy_a:    u8,
    mpy_b:    u8,
    dividend: u16,
    divisor:  u8,
    mpy_res:  u16, // Doubles as division reminder
    div_res:  u16,
}

impl MpyDiv {
    pub fn new() -> MpyDiv {
        MpyDiv {
            mpy_a:    0xFF,
            mpy_b:    0xFF,
            dividend: 0xFFFF,
            divisor:  0xFF,
            mpy_res:  0x0000, // TODO: Check result inits
            div_res:  0x0000,
        }
    }

    pub fn set_mpy_a(&mut self, value: u8) {
        self.mpy_a = value;
    }

    pub fn set_mpy_b(&mut self, value: u8) {
        self.mpy_b = value;
        self.mpy_res = (self.mpy_a as u16) * (self.mpy_b as u16); // TODO: Timing
        self.div_res = self.mpy_b as u16;
    }

    pub fn set_dividend_low(&mut self, value: u8) {
        self.dividend = (self.dividend & 0xFF00) | value as u16;
    }

    pub fn set_dividend_high(&mut self, value: u8) {
        self.dividend = ((value as u16) << 8) | (self.dividend & 0x00FF);
    }

    pub fn set_divisor(&mut self, value: u8) {
        self.divisor = value; // TODO: Timing
        if self.divisor == 0 {
            self.div_res = 0xFFFF;
            self.mpy_res = self.dividend;
        } else {
            self.div_res = self.dividend / self.divisor as u16;
            self.mpy_res = self.dividend % self.divisor as u16;
        }
    }

    pub fn get_mpy_res_low(&self) -> u8 {
        self.mpy_res as u8
    }

    pub fn get_mpy_res_high(&self) -> u8 {
        (self.mpy_res >> 8) as u8
    }

    pub fn get_div_res_low(&self) -> u8 {
        self.div_res as u8
    }

    pub fn get_div_res_high(&self) -> u8 {
        (self.div_res >> 8) as u8
    }
}

#[derive(Copy, Clone)]
struct DoubleReg {
    value: u16,
    high_active: bool, // TODO: Should there be separate flags for read and write?
}

impl DoubleReg {
    pub fn new() -> DoubleReg {
        DoubleReg {
            value:           0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_reg_write() {
        let mut reg = DoubleReg::new();
        reg.write(0x34);
        reg.write(0x12);
        assert_eq!(0x1234, reg.value);
        reg.write(0xCD);
        reg.write(0xAB);
        assert_eq!(0xABCD, reg.value);
    }

    #[test]
    fn double_reg_read() {
        let mut reg = DoubleReg::new();
        reg.value = 0x1234;
        assert_eq!(0x34, reg.read());
        assert_eq!(0x12, reg.read());
        reg.value = 0xABCD;
        assert_eq!(0xCD, reg.read());
        assert_eq!(0xAB, reg.read());
    }

    #[test]
    fn mpy() {
        let mut mpy_div = MpyDiv::new();
        // Straight multiplication
        mpy_div.set_mpy_a(0xFA);
        mpy_div.set_mpy_b(0xFB);
        assert_eq!(0xF51E, mpy_div.mpy_res);
        assert_eq!(0x1E, mpy_div.get_mpy_res_low());
        assert_eq!(0xF5, mpy_div.get_mpy_res_high());
        // Side-effect
        assert_eq!(0x00FB, mpy_div.div_res); // According to fullsnes

        // Only b triggers mpy
        mpy_div.set_mpy_a(0xFB);
        assert_eq!(0xF51E, mpy_div.mpy_res);

        // By zero
        mpy_div.set_mpy_b(0x00);
        assert_eq!(0x0000, mpy_div.mpy_res);
    }

    #[test]
    fn div() {
        let mut mpy_div = MpyDiv::new();
        // Straight division
        mpy_div.set_dividend_low(0xFB);
        mpy_div.set_dividend_high(0xFA);
        mpy_div.set_divisor(0x1A);
        assert_eq!(0x09A7, mpy_div.div_res);
        assert_eq!(0xA7, mpy_div.get_div_res_low());
        assert_eq!(0x09, mpy_div.get_div_res_high());
        assert_eq!(0x0005, mpy_div.mpy_res);

        // Only divisor triggers div
        mpy_div.set_dividend_low(0xFD);
        mpy_div.set_dividend_high(0xFC);
        assert_eq!(0x09A7, mpy_div.div_res);
        assert_eq!(0x0005, mpy_div.mpy_res);

        // Zero remainder
        mpy_div.set_divisor(0x01);
        assert_eq!(0xFCFD, mpy_div.div_res);
        assert_eq!(0x0000, mpy_div.mpy_res);

        // Only remainder
        mpy_div.set_dividend_low(0x09);
        mpy_div.set_dividend_high(0x00);
        mpy_div.set_divisor(0x1A);
        assert_eq!(0x0000, mpy_div.div_res);
        assert_eq!(0x0009, mpy_div.mpy_res);

        // Division by zero
        mpy_div.set_dividend_low(0xFB);
        mpy_div.set_dividend_high(0xFA);
        mpy_div.set_divisor(0x00);
        assert_eq!(0xFFFF, mpy_div.div_res);
        assert_eq!(0xFAFB, mpy_div.mpy_res);
    }

    #[test]
    fn wrapping_adds() {
        assert_eq!(0xAAABA9, bank_wrapping_add(0xAAAAAA, 0xFF));
        assert_eq!(0xAA2AA9, bank_wrapping_add(0xAAAAAA, 0x7FFF));
        assert_eq!(0xAAAAB9, page_wrapping_add(0xAAAAAA, 0xF));
        assert_eq!(0xAAAA29, page_wrapping_add(0xAAAAAA, 0x7F));
    }

    #[test]
    fn wrapping_subs() {
        assert_eq!(0xAAA9AB, bank_wrapping_sub(0xAAAAAA, 0xFF));
        assert_eq!(0xAA2AAB, bank_wrapping_sub(0xAAAAAA, 0x7FFF));
        assert_eq!(0xAAAA9B, page_wrapping_sub(0xAAAAAA, 0xF));
        assert_eq!(0xAAAA2B, page_wrapping_sub(0xAAAAAA, 0x7F));
    }

    #[test]
    fn cpu_wrapping_reads() {
        let mut abus = ABus::new_empty_rom();
        abus.wram[0x100FF] = 0xEF;
        abus.wram[0x10000] = 0xCD;
        abus.wram[0x10001] = 0xAB;
        assert_eq!(0xCDEF, abus.page_wrapping_cpu_read_16(0x7F00FF));
        assert_eq!(0xABCDEF, abus.page_wrapping_cpu_read_24(0x7F00FF));
        abus.wram[0x100FF] = 0x0;
        abus.wram[0x1FFFF] = 0xEF;
        assert_eq!(0xCDEF, abus.bank_wrapping_cpu_read_16(0x7FFFFF));
        assert_eq!(0xABCDEF, abus.bank_wrapping_cpu_read_24(0x7FFFFF));
    }

    #[test]
    fn cpu_wrapping_writes() {
        let mut abus = ABus::new_empty_rom();
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x7FFFFF);
        assert_eq!(0xCD, abus.wram[0x1FFFF]);
        assert_eq!(0xAB, abus.wram[0x10000]);
        abus.wram[0x1FFFF] = 0x0;
        abus.wram[0x10000] = 0x0;
        abus.bank_wrapping_cpu_write_24(0xABCDEF, 0x7FFFFF);
        assert_eq!(0xEF, abus.wram[0x1FFFF]);
        assert_eq!(0xCD, abus.wram[0x10000]);
        assert_eq!(0xAB, abus.wram[0x10001]);
        abus.wram[0x1FFFF] = 0x0;
        abus.wram[0x10000] = 0x0;
        abus.wram[0x10001] = 0x0;
        abus.page_wrapping_cpu_write_16(0xABCD, 0x7F00FF);
        assert_eq!(0xCD, abus.wram[0x100FF]);
        assert_eq!(0xAB, abus.wram[0x10000]);
        abus.wram[0x100FF] = 0x0;
        abus.wram[0x10000] = 0x0;
        abus.page_wrapping_cpu_write_24(0xABCDEF, 0x7F00FF);
        assert_eq!(0xEF, abus.wram[0x100FF]);
        assert_eq!(0xCD, abus.wram[0x10000]);
        assert_eq!(0xAB, abus.wram[0x10001]);
    }

    #[test]
    fn fetch_operand() {
        let mut abus = ABus::new_empty_rom();
        abus.wram[0x1FFFD] = 0x9A;
        abus.wram[0x1FFFE] = 0x78;
        abus.wram[0x1FFFF] = 0x56;
        abus.wram[0x10000] = 0x34;
        abus.wram[0x10001] = 0x12;
        assert_eq!(0x56, abus.fetch_operand_8(0x7FFFFE));
        assert_eq!(0x34, abus.fetch_operand_8(0x7FFFFF));
        assert_eq!(0x5678, abus.fetch_operand_16(0x7FFFFD));
        assert_eq!(0x3456, abus.fetch_operand_16(0x7FFFFE));
        assert_eq!(0x56789A, abus.fetch_operand_24(0x7FFFFC));
        assert_eq!(0x123456, abus.fetch_operand_24(0x7FFFFE));
    }
}

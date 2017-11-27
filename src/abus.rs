use dma::Dma;
use joypad::JoyIo;
use mmap;
use mpydiv::MpyDiv;
use ppu_io::PpuIo;
use rom::Rom;

pub struct ABus {
    wram: [u8; 131072],
    vram: [u8; 65536],
    oam: [u8; 544],
    cgram: [u8; 512],
    rom: Rom,
    mpy_div: MpyDiv,
    ppu_io: PpuIo,
    joy_io: JoyIo,
    dma: Dma,
    // "On-chip" CPU W
    nmitimen: u8,
    htime: u16,
    vtime: u16,
    memsel: u8,
    // APU IO
    apu_io0: u8,
    apu_io1: u8,
    apu_io2: u8,
    apu_io3: u8,
    // WRAM access
    wm_add_l: u8,
    wm_add_m: u8,
    wm_add_h: u8,
    // Blank regs
    rd_nmi: u8,
    time_up: u8,
    hvb_joy: u8,
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
        ABus {
            wram: [0; 131072],
            vram: [0; 65536],
            oam: [0; 544],
            cgram: [0; 512],
            rom: Rom::new(rom_path),
            mpy_div: MpyDiv::new(),
            ppu_io: PpuIo::new(),
            joy_io: JoyIo::new(),
            dma: Dma::new(),
            nmitimen: 0x00,
            htime: 0x01FF,
            vtime: 0x01FF,
            memsel: 0x00,
            apu_io0: 0x00,
            apu_io1: 0x00,
            apu_io2: 0x00,
            apu_io3: 0x00,
            wm_add_l: 0x00,
            wm_add_m: 0x00,
            wm_add_h: 0x00,
            rd_nmi: 0x00,
            time_up: 0x00,
            hvb_joy: 0x00,
        }
    }

    #[cfg(test)]
    pub fn new_empty_rom() -> ABus {
        ABus {
            wram: [0; 131072],
            vram: [0; 65536],
            oam: [0; 544],
            cgram: [0; 512],
            rom: Rom::new_empty(),
            mpy_div: MpyDiv::new(),
            ppu_io: PpuIo::new(),
            joy_io: JoyIo::new(),
            dma: Dma::new(),
            nmitimen: 0x00,
            htime: 0x01FF,
            vtime: 0x01FF,
            memsel: 0x00,
            apu_io0: 0x00,
            apu_io1: 0x00,
            apu_io2: 0x00,
            apu_io3: 0x00,
            wm_add_l: 0x00,
            wm_add_m: 0x00,
            wm_add_h: 0x00,
            rd_nmi: 0x00,
            time_up: 0x00,
            hvb_joy: 0x00,
        }
    }

    fn cpu_read_sys(&mut self, addr: usize) -> u8 {
        match addr {
            mmap::WRAM_MIRR_FIRST...mmap::WRAM_MIRR_LAST => self.wram[addr],
            mmap::PPU_IO_FIRST...mmap::PPU_IO_LAST => {
                // PPU IO
                if addr < mmap::MPYL {
                    panic!("Read ${:06X}: PPU IO write-only for cpu", addr);
                }
                self.ppu_io.read(addr)
            }
            mmap::APU_IO_FIRST...mmap::APU_IO_LAST => {
                // APU IO
                let apu_port = if addr < 0x2144 { addr } else { addr - 4 };
                match apu_port {
                    mmap::APUI00 => self.apu_io0,
                    mmap::APUI01 => self.apu_io1,
                    mmap::APUI02 => self.apu_io2,
                    mmap::APUI03 => self.apu_io3,
                    _ => unreachable!(),
                }
            }
            mmap::WMDATA => {
                let wram_addr = ((self.wm_add_h as usize) << 16) | ((self.wm_add_m as usize) << 8)
                    | (self.wm_add_l as usize);
                self.wram[wram_addr]
            }
            mmap::JOYA => self.joy_io.joy_a(),
            mmap::JOYB => self.joy_io.joy_b(),
            mmap::RDNMI => {
                let val = self.rd_nmi;
                self.rd_nmi &= 0b0111_1111;
                val
            }
            mmap::TIMEUP => {
                let val = self.time_up;
                self.time_up &= 0b0111_1111;
                val
            }
            mmap::HVBJOY => self.hvb_joy,
            mmap::RDIO => self.joy_io.rd_io(),
            mmap::RDDIVL => self.mpy_div.get_div_res_low(),
            mmap::RDDIVH => self.mpy_div.get_div_res_high(),
            mmap::RDMPYL => self.mpy_div.get_mpy_res_low(),
            mmap::RDMPYH => self.mpy_div.get_mpy_res_high(),
            mmap::JOY1L => self.joy_io.joy_1l(),
            mmap::JOY1H => self.joy_io.joy_1h(),
            mmap::JOY2L => self.joy_io.joy_2l(),
            mmap::JOY2H => self.joy_io.joy_2h(),
            mmap::JOY3L => self.joy_io.joy_3l(),
            mmap::JOY3H => self.joy_io.joy_3h(),
            mmap::JOY4L => self.joy_io.joy_4l(),
            mmap::JOY4H => self.joy_io.joy_4h(),
            mmap::DMA_FIRST...mmap::DMA_LAST => {
                // DMA
                self.dma.read(addr)
            }
            mmap::EXP_FIRST...mmap::EXP_LAST => {
                // Expansion
                panic!("Read ${:06X}: Expansion not implemented", addr)
            }
            _ => panic!(
                "System area read ${:04X}: Address unused or write-only",
                addr
            ),
        }
    }

    pub fn cpu_read8(&mut self, addr: u32) -> u8 {
        let bank = (addr >> 16) as usize;
        let bank_addr = (addr & 0x00FFFF) as usize;
        match bank {
            mmap::WS1_SYSLR_FIRST_BANK...mmap::WS1_SYSLR_LAST_BANK => match bank_addr {
                mmap::SYS_FIRST...mmap::SYS_LAST => self.cpu_read_sys(bank_addr),
                mmap::LOROM_FIRST...mmap::LOROM_LAST => self.rom.read_ws1_lo_rom8(bank, bank_addr),
                _ => unreachable!(),
            },
            mmap::WS1_HIROM_FIRST_BANK...mmap::WS1_HIROM_LAST_BANK => {
                self.rom.read_ws1_hi_rom8(bank, bank_addr)
            }
            mmap::WRAM_FIRST_BANK...mmap::WRAM_LAST_BANK => {
                self.wram[(bank - mmap::WRAM_FIRST_BANK) * 0x10000 + bank_addr]
            }
            mmap::WS2_SYSLR_FIRST_BANK...mmap::WS2_SYSLR_LAST_BANK => match bank_addr {
                mmap::SYS_FIRST...mmap::SYS_LAST => self.cpu_read_sys(bank_addr),
                mmap::LOROM_FIRST...mmap::LOROM_LAST => self.rom.read_ws2_lo_rom8(bank, bank_addr),
                _ => unreachable!(),
            },
            mmap::WS2_HIROM_FIRST_BANK...mmap::WS2_HIROM_LAST_BANK => {
                self.rom.read_ws2_hi_rom8(bank, bank_addr)
            }
            _ => unreachable!(),
        }
    }

    #[allow(dead_code)]
    pub fn apu_read8(&mut self, reg: u8) -> u8 {
        match reg {
            0x00 => self.apu_io0,
            0x01 => self.apu_io1,
            0x02 => self.apu_io2,
            0x03 => self.apu_io3,
            _ => panic!("APU read from invalid IO register!"),
        }
    }

    pub fn addr_wrapping_cpu_read16(&mut self, addr: u32) -> u16 {
        self.cpu_read8(addr) as u16 | ((self.cpu_read8(addr_wrapping_add(addr, 1)) as u16) << 8)
    }

    #[allow(dead_code)]
    pub fn addr_wrapping_cpu_read24(&mut self, addr: u32) -> u32 {
        self.cpu_read8(addr) as u32 | ((self.cpu_read8(addr_wrapping_add(addr, 1)) as u32) << 8)
            | ((self.cpu_read8(addr_wrapping_add(addr, 2)) as u32) << 16)
    }


    pub fn bank_wrapping_cpu_read16(&mut self, addr: u32) -> u16 {
        self.cpu_read8(addr) as u16 | ((self.cpu_read8(bank_wrapping_add(addr, 1)) as u16) << 8)
    }

    pub fn bank_wrapping_cpu_read24(&mut self, addr: u32) -> u32 {
        self.cpu_read8(addr) as u32 | ((self.cpu_read8(bank_wrapping_add(addr, 1)) as u32) << 8)
            | ((self.cpu_read8(bank_wrapping_add(addr, 2)) as u32) << 16)
    }

    pub fn page_wrapping_cpu_read16(&mut self, addr: u32) -> u16 {
        self.cpu_read8(addr) as u16 | ((self.cpu_read8(page_wrapping_add(addr, 1)) as u16) << 8)
    }

    pub fn page_wrapping_cpu_read24(&mut self, addr: u32) -> u32 {
        self.cpu_read8(addr) as u32 | ((self.cpu_read8(page_wrapping_add(addr, 1)) as u32) << 8)
            | ((self.cpu_read8(page_wrapping_add(addr, 2)) as u32) << 16)
    }

    pub fn fetch_operand8(&mut self, addr: u32) -> u8 { self.cpu_read8(bank_wrapping_add(addr, 1)) }

    pub fn fetch_operand16(&mut self, addr: u32) -> u16 {
        self.bank_wrapping_cpu_read16(bank_wrapping_add(addr, 1))
    }

    pub fn fetch_operand24(&mut self, addr: u32) -> u32 {
        self.bank_wrapping_cpu_read24(bank_wrapping_add(addr, 1))
    }

    fn cpu_write_sys(&mut self, value: u8, addr: usize) {
        match addr {
            mmap::WRAM_MIRR_FIRST...mmap::WRAM_MIRR_LAST => {
                // WRAM
                self.wram[addr] = value
            }
            mmap::PPU_IO_FIRST...mmap::PPU_IO_LAST => {
                // PPU IO
                if addr > mmap::SETINI {
                    panic!("Write ${:06X}: PPU IO read-only for cpu", addr);
                }
                self.ppu_io.write(value, addr);
            }
            mmap::APU_IO_FIRST...mmap::APU_IO_LAST => {
                // APU IO
                let apu_port = if addr < 0x2144 { addr } else { addr - 4 };
                match apu_port {
                    mmap::APUI00 => self.apu_io0 = value,
                    mmap::APUI01 => self.apu_io1 = value,
                    mmap::APUI02 => self.apu_io2 = value,
                    mmap::APUI03 => self.apu_io3 = value,
                    _ => unreachable!(),
                }
            }
            mmap::WMDATA => {
                let wram_addr = ((self.wm_add_h as usize) << 16) | ((self.wm_add_m as usize) << 8)
                    | (self.wm_add_l as usize);
                self.wram[wram_addr] = value;
            }
            mmap::WMADDL => self.wm_add_l = value,
            mmap::WMADDM => self.wm_add_m = value,
            mmap::WMADDH => self.wm_add_h = value,
            mmap::JOYWR => self.joy_io.set_joy_wr(value),
            mmap::NMITIMEN => self.nmitimen = value,
            mmap::WRIO => self.joy_io.set_wr_io(value),
            mmap::WRMPYA => self.mpy_div.set_mpy_a(value),
            mmap::WRMPYB => self.mpy_div.set_mpy_b(value),
            mmap::WRDIVL => self.mpy_div.set_dividend_low(value),
            mmap::WRDIVH => self.mpy_div.set_dividend_high(value),
            mmap::WRDIVB => self.mpy_div.set_divisor(value),
            mmap::HTIMEL => self.htime = (self.htime & 0xFF00) | value as u16,
            mmap::HTIMEH => self.htime = ((value as u16) << 8) | (self.htime & 0x00FF),
            mmap::VTIMEL => self.vtime = (self.vtime & 0xFF00) | value as u16,
            mmap::VTIMEH => self.vtime = ((value as u16) << 8) | (self.vtime & 0x00FF),
            mmap::MDMAEN => self.dma.write_mdma_en(value),
            mmap::HDMAEN => self.dma.write_hdma_en(value),
            mmap::MEMSEL => self.memsel = value,
            mmap::DMA_FIRST...mmap::DMA_LAST => {
                // DMA
                self.dma.write(value, addr);
            }
            mmap::EXP_FIRST...mmap::EXP_LAST => {
                // Expansion
                panic!("Write ${:06X}: Expansion not implemented", addr)
            }
            _ => panic!(
                "System area write ${:04X}: Address unused or read-only",
                addr
            ),
        }
    }

    pub fn cpu_write8(&mut self, value: u8, addr: u32) {
        let bank = (addr >> 16) as usize;
        let bank_addr = (addr & 0x00FFFF) as usize;
        match bank {
            mmap::WS1_SYSLR_FIRST_BANK...mmap::WS1_SYSLR_LAST_BANK => match bank_addr {
                mmap::SYS_FIRST...mmap::SYS_LAST => self.cpu_write_sys(value, bank_addr),
                mmap::LOROM_FIRST...mmap::LOROM_LAST => {
                    self.rom.write_ws1_lo_rom8(value, bank, bank_addr);
                }
                _ => unreachable!(),
            },
            mmap::WS1_HIROM_FIRST_BANK...mmap::WS1_HIROM_LAST_BANK => {
                self.rom.write_ws1_hi_rom8(value, bank, bank_addr);
            }
            mmap::WRAM_FIRST_BANK...mmap::WRAM_LAST_BANK => {
                self.wram[(bank - mmap::WRAM_FIRST_BANK) * 0x10000 + bank_addr] = value;
            }
            mmap::WS2_SYSLR_FIRST_BANK...mmap::WS2_SYSLR_LAST_BANK => match bank_addr {
                mmap::SYS_FIRST...mmap::SYS_LAST => self.cpu_write_sys(value, bank_addr),
                mmap::LOROM_FIRST...mmap::LOROM_LAST => {
                    self.rom.write_ws2_lo_rom8(value, bank, bank_addr);
                }
                _ => unreachable!(),
            },
            mmap::WS2_HIROM_FIRST_BANK...mmap::WS2_HIROM_LAST_BANK => {
                self.rom.write_ws2_hi_rom8(value, bank, bank_addr);
            }
            _ => unreachable!(),
        }
    }

    pub fn apu_write8(&mut self, value: u8, reg: u8) {
        match reg {
            0x00 => self.apu_io0 = value,
            0x01 => self.apu_io1 = value,
            0x02 => self.apu_io2 = value,
            0x03 => self.apu_io3 = value,
            _ => panic!("APU write to invalid IO register!"),
        }
    }

    pub fn addr_wrapping_cpu_write16(&mut self, value: u16, addr: u32) {
        self.cpu_write8(value as u8, addr);
        self.cpu_write8((value >> 8) as u8, addr_wrapping_add(addr, 1));
    }

    #[allow(dead_code)]
    pub fn addr_wrapping_cpu_write24(&mut self, value: u32, addr: u32) {
        self.cpu_write8(value as u8, addr);
        self.cpu_write8((value >> 8) as u8, addr_wrapping_add(addr, 1));
        self.cpu_write8((value >> 16) as u8, addr_wrapping_add(addr, 2));
    }

    pub fn bank_wrapping_cpu_write16(&mut self, value: u16, addr: u32) {
        self.cpu_write8(value as u8, addr);
        self.cpu_write8((value >> 8) as u8, bank_wrapping_add(addr, 1));
    }

    pub fn bank_wrapping_cpu_write24(&mut self, value: u32, addr: u32) {
        self.cpu_write8(value as u8, addr);
        self.cpu_write8((value >> 8) as u8, bank_wrapping_add(addr, 1));
        self.cpu_write8((value >> 16) as u8, bank_wrapping_add(addr, 2));
    }

    pub fn page_wrapping_cpu_write16(&mut self, value: u16, addr: u32) {
        self.cpu_write8(value as u8, addr);
        self.cpu_write8((value >> 8) as u8, page_wrapping_add(addr, 1));
    }

    pub fn page_wrapping_cpu_write24(&mut self, value: u32, addr: u32) {
        self.cpu_write8(value as u8, addr);
        self.cpu_write8((value >> 8) as u8, page_wrapping_add(addr, 1));
        self.cpu_write8((value >> 16) as u8, page_wrapping_add(addr, 2));
    }
}
pub fn addr_wrapping_add(addr: u32, offset: u32) -> u32 { (addr + offset) & 0x00FFFFFF }

pub fn bank_wrapping_add(addr: u32, offset: u16) -> u32 {
    (addr & 0xFF0000) | ((addr as u16).wrapping_add(offset) as u32)
}

pub fn page_wrapping_add(addr: u32, offset: u8) -> u32 {
    (addr & 0xFFFF00) | ((addr as u8).wrapping_add(offset) as u32)
}

#[allow(dead_code)]
pub fn bank_wrapping_sub(addr: u32, offset: u16) -> u32 {
    (addr & 0xFF0000) | ((addr as u16).wrapping_sub(offset) as u32)
}

#[allow(dead_code)]
pub fn page_wrapping_sub(addr: u32, offset: u8) -> u32 {
    (addr & 0xFFFF00) | ((addr as u8).wrapping_sub(offset) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(0xCDEF, abus.page_wrapping_cpu_read16(0x7F00FF));
        assert_eq!(0xABCDEF, abus.page_wrapping_cpu_read24(0x7F00FF));
        abus.wram[0x100FF] = 0x0;
        abus.wram[0x1FFFF] = 0xEF;
        assert_eq!(0xCDEF, abus.bank_wrapping_cpu_read16(0x7FFFFF));
        assert_eq!(0xABCDEF, abus.bank_wrapping_cpu_read24(0x7FFFFF));
    }

    #[test]
    fn cpu_wrapping_writes() {
        let mut abus = ABus::new_empty_rom();
        abus.bank_wrapping_cpu_write16(0xABCD, 0x7FFFFF);
        assert_eq!(0xCD, abus.wram[0x1FFFF]);
        assert_eq!(0xAB, abus.wram[0x10000]);
        abus.wram[0x1FFFF] = 0x0;
        abus.wram[0x10000] = 0x0;
        abus.bank_wrapping_cpu_write24(0xABCDEF, 0x7FFFFF);
        assert_eq!(0xEF, abus.wram[0x1FFFF]);
        assert_eq!(0xCD, abus.wram[0x10000]);
        assert_eq!(0xAB, abus.wram[0x10001]);
        abus.wram[0x1FFFF] = 0x0;
        abus.wram[0x10000] = 0x0;
        abus.wram[0x10001] = 0x0;
        abus.page_wrapping_cpu_write16(0xABCD, 0x7F00FF);
        assert_eq!(0xCD, abus.wram[0x100FF]);
        assert_eq!(0xAB, abus.wram[0x10000]);
        abus.wram[0x100FF] = 0x0;
        abus.wram[0x10000] = 0x0;
        abus.page_wrapping_cpu_write24(0xABCDEF, 0x7F00FF);
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
        assert_eq!(0x56, abus.fetch_operand8(0x7FFFFE));
        assert_eq!(0x34, abus.fetch_operand8(0x7FFFFF));
        assert_eq!(0x5678, abus.fetch_operand16(0x7FFFFD));
        assert_eq!(0x3456, abus.fetch_operand16(0x7FFFFE));
        assert_eq!(0x56789A, abus.fetch_operand24(0x7FFFFC));
        assert_eq!(0x123456, abus.fetch_operand24(0x7FFFFE));
    }

    #[test]
    fn cpu_read_system_area() {
        let mut abus = ABus::new_empty_rom();
        // WRAM
        for i in 0..0x2000 {
            abus.wram[i] = 0xAA;
        }
        for i in 0..0x2000 {
            assert_eq!(0xAA, abus.cpu_read8(i));
        }
    }
}

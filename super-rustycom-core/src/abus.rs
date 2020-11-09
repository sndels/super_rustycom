use crate::apu_io::ApuIo;
use crate::dma::Dma;
use crate::joypad::JoyIo;
use crate::mmap;
use crate::mpydiv::MpyDiv;
use crate::ppu_io::PpuIo;
use crate::rom::Rom;

use log::{error, warn};

/// 128 kB of "work" memory
const WRAM_SIZE: usize = 128 * 1024;
/// 64 kB of video memory
const VRAM_SIZE: usize = 64 * 1024;
/// 544 bytes of Object Attribute Memory used for sprite info
const OAM_SIZE: usize = 544;
/// 512 bytes of color palette memory
const CGRAM_SIZE: usize = 512;

/// Main interface for accessing different memory chunks and common registers
pub struct ABus {
    // TODO: Use straight arrays instead and wrap ABus in box to get better cache coherency?
    /// "Work"RAM
    wram: Box<[u8]>,
    /// VideoRAM
    vram: Box<[u8]>,
    /// Object Attribute Memory, used for sprite info
    oam: Box<[u8]>,
    /// Color palette memory
    cgram: Box<[u8]>,
    /// The loaded ROM instance
    rom: Rom,
    /// Combined multiplication and division register
    mpy_div: MpyDiv,
    /// Registers for CPU<->PPU communication
    ppu_io: PpuIo,
    /// Joypad registers
    joy_io: JoyIo,
    /// Dma controller
    dma: Dma,
    /// Interrupt Enable and Joypad Request
    nmitimen: u8,
    /// H-count timer setting
    htime: u16,
    /// V-count timer setting
    vtime: u16,
    /// Memory-2 waitstate control
    memsel: u8,
    /// APU communication
    apu_io_r: ApuIo,
    apu_io_w: ApuIo,
    /// Lower 8bit of WRAM address used by WMDATA reads
    wm_add_l: u8,
    /// Middle 8bit of WRAM address used by WMDATA reads
    wm_add_m: u8,
    /// Upper 1bit of WRAM address used by WMDATA reads
    wm_add_h: u8,
    /// V-blank NMI flag and CPU version number
    rd_nmi: u8,
    /// H/V-timer IRG flag
    time_up: u8,
    /// H/V-blank flag and joypad busy flag
    hvb_joy: u8,
}

impl ABus {
    /// Initializes a new instance with default values and loads the given ROM
    pub fn new(rom_bytes: Vec<u8>) -> ABus {
        // TODO: Randomize values?
        ABus {
            wram: Box::new([0; WRAM_SIZE]),
            vram: Box::new([0; VRAM_SIZE]),
            oam: Box::new([0; OAM_SIZE]),
            cgram: Box::new([0; CGRAM_SIZE]),
            rom: Rom::new(rom_bytes),
            mpy_div: MpyDiv::new(),
            ppu_io: PpuIo::new(),
            joy_io: JoyIo::new(),
            dma: Dma::new(),
            nmitimen: 0x00,
            htime: 0x01FF,
            vtime: 0x01FF,
            memsel: 0x00,
            apu_io_r: ApuIo::default(),
            apu_io_w: ApuIo::default(),
            wm_add_l: 0x00,
            wm_add_m: 0x00,
            wm_add_h: 0x00,
            rd_nmi: 0x00,
            time_up: 0x00,
            hvb_joy: 0x00,
        }
    }

    #[cfg(test)]
    /// Initializes a new instance with default values and an empty rom for unit testing purposes
    pub fn new_empty_rom() -> ABus {
        ABus {
            wram: Box::new([0; WRAM_SIZE]),
            vram: Box::new([0; VRAM_SIZE]),
            oam: Box::new([0; OAM_SIZE]),
            cgram: Box::new([0; CGRAM_SIZE]),
            rom: Rom::new_empty(),
            mpy_div: MpyDiv::new(),
            ppu_io: PpuIo::new(),
            joy_io: JoyIo::new(),
            dma: Dma::new(),
            nmitimen: 0x00,
            htime: 0x01FF,
            vtime: 0x01FF,
            memsel: 0x00,
            apu_io_r: ApuIo::default(),
            apu_io_w: ApuIo::default(),
            wm_add_l: 0x00,
            wm_add_m: 0x00,
            wm_add_h: 0x00,
            rd_nmi: 0x00,
            time_up: 0x00,
            hvb_joy: 0x00,
        }
    }

    pub fn wram(&self) -> &[u8] {
        &self.wram
    }
    pub fn vram(&self) -> &[u8] {
        &self.vram
    }
    pub fn oam(&self) -> &[u8] {
        &self.oam
    }
    pub fn cgram(&self) -> &[u8] {
        &self.cgram
    }

    pub fn apu_io(&self) -> ApuIo {
        self.apu_io_w
    }

    pub fn write_smp_io(&mut self, io: ApuIo) {
        self.apu_io_r = io;
    }

    fn cpu_read_sys(&mut self, addr: usize) -> u8 {
        match addr {
            mmap::WRAM_MIRR_FIRST..=mmap::WRAM_MIRR_LAST => self.wram[addr],
            mmap::PPU_IO_FIRST..=mmap::PPU_IO_LAST => {
                // PPU IO
                if addr < mmap::MPYL {
                    error!("Read ${:06X}: PPU IO write-only for cpu", addr);
                }
                self.ppu_io.read(addr)
            }
            mmap::APU_IO_FIRST..=mmap::APU_IO_LAST => {
                // APU IO
                let port = (if addr < 0x2144 { addr } else { addr - 4 } as u8) & 0x0F;
                self.apu_io_r.read(port)
            }
            mmap::WMDATA => {
                // Get current address
                let wram_addr = (((self.wm_add_h & 0x1) as usize) << 16)
                    | ((self.wm_add_m as usize) << 8)
                    | (self.wm_add_l as usize);
                // Increment address
                self.wm_add_l = self.wm_add_l.wrapping_add(1);
                if self.wm_add_l == 0x00 {
                    self.wm_add_m = self.wm_add_m.wrapping_add(1);
                    if self.wm_add_m == 0x00 {
                        self.wm_add_h = self.wm_add_h.wrapping_add(1);
                        if self.wm_add_h & 0x01 == 0x00 {
                            self.wm_add_l = 0x01;
                        }
                    }
                }
                // Return data at the original address
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
            mmap::DMA_FIRST..=mmap::DMA_LAST => {
                // DMA
                self.dma.read(addr)
            }
            mmap::EXP_FIRST..=mmap::EXP_LAST => {
                // Expansion
                warn!("Read ${:06X}: Expansion not implemented", addr);
                0
            }
            _ => {
                error!(
                    "System area read ${:04X}: Address unused or write-only",
                    addr
                );
                0
            }
        }
    }

    pub fn cpu_read8(&mut self, addr: u32) -> u8 {
        let bank = (addr >> 16) as usize;
        let bank_addr = (addr & 0x00FFFF) as usize;
        match bank {
            mmap::WS1_SYSLR_FIRST_BANK..=mmap::WS1_SYSLR_LAST_BANK => match bank_addr {
                mmap::SYS_FIRST..=mmap::SYS_LAST => self.cpu_read_sys(bank_addr),
                mmap::LOROM_FIRST..=mmap::LOROM_LAST => self.rom.read_ws1_lo_rom8(bank, bank_addr),
                _ => unreachable!(),
            },
            mmap::WS1_HIROM_FIRST_BANK..=mmap::WS1_HIROM_LAST_BANK => {
                self.rom.read_ws1_hi_rom8(bank, bank_addr)
            }
            mmap::WRAM_FIRST_BANK..=mmap::WRAM_LAST_BANK => {
                self.wram[(bank - mmap::WRAM_FIRST_BANK) * 0x10000 + bank_addr]
            }
            mmap::WS2_SYSLR_FIRST_BANK..=mmap::WS2_SYSLR_LAST_BANK => match bank_addr {
                mmap::SYS_FIRST..=mmap::SYS_LAST => self.cpu_read_sys(bank_addr),
                mmap::LOROM_FIRST..=mmap::LOROM_LAST => self.rom.read_ws2_lo_rom8(bank, bank_addr),
                _ => unreachable!(),
            },
            mmap::WS2_HIROM_FIRST_BANK..=mmap::WS2_HIROM_LAST_BANK => {
                self.rom.read_ws2_hi_rom8(bank, bank_addr)
            }
            _ => unreachable!(),
        }
    }

    pub fn addr_wrapping_cpu_read16(&mut self, addr: u32) -> u16 {
        self.cpu_read8(addr) as u16 | ((self.cpu_read8(addr_wrapping_add(addr, 1)) as u16) << 8)
    }

    #[allow(dead_code)]
    pub fn addr_wrapping_cpu_read24(&mut self, addr: u32) -> u32 {
        self.cpu_read8(addr) as u32
            | ((self.cpu_read8(addr_wrapping_add(addr, 1)) as u32) << 8)
            | ((self.cpu_read8(addr_wrapping_add(addr, 2)) as u32) << 16)
    }

    pub fn bank_wrapping_cpu_read16(&mut self, addr: u32) -> u16 {
        self.cpu_read8(addr) as u16 | ((self.cpu_read8(bank_wrapping_add(addr, 1)) as u16) << 8)
    }

    pub fn bank_wrapping_cpu_read24(&mut self, addr: u32) -> u32 {
        self.cpu_read8(addr) as u32
            | ((self.cpu_read8(bank_wrapping_add(addr, 1)) as u32) << 8)
            | ((self.cpu_read8(bank_wrapping_add(addr, 2)) as u32) << 16)
    }

    pub fn page_wrapping_cpu_read16(&mut self, addr: u32) -> u16 {
        self.cpu_read8(addr) as u16 | ((self.cpu_read8(page_wrapping_add(addr, 1)) as u16) << 8)
    }

    pub fn page_wrapping_cpu_read24(&mut self, addr: u32) -> u32 {
        self.cpu_read8(addr) as u32
            | ((self.cpu_read8(page_wrapping_add(addr, 1)) as u32) << 8)
            | ((self.cpu_read8(page_wrapping_add(addr, 2)) as u32) << 16)
    }

    pub fn fetch_operand8(&mut self, addr: u32) -> u8 {
        self.cpu_read8(bank_wrapping_add(addr, 1))
    }

    pub fn fetch_operand16(&mut self, addr: u32) -> u16 {
        self.bank_wrapping_cpu_read16(bank_wrapping_add(addr, 1))
    }

    pub fn fetch_operand24(&mut self, addr: u32) -> u32 {
        self.bank_wrapping_cpu_read24(bank_wrapping_add(addr, 1))
    }

    fn cpu_write_sys(&mut self, addr: usize, value: u8) {
        match addr {
            mmap::WRAM_MIRR_FIRST..=mmap::WRAM_MIRR_LAST => {
                // WRAM
                self.wram[addr] = value
            }
            mmap::PPU_IO_FIRST..=mmap::PPU_IO_LAST => {
                // PPU IO
                if addr <= mmap::SETINI {
                    self.ppu_io.write(addr, value);
                } else {
                    error!("Write ${:06X}: PPU IO read-only for cpu", addr);
                }
            }
            mmap::APU_IO_FIRST..=mmap::APU_IO_LAST => {
                // APU IO
                let port = (if addr < 0x2144 { addr } else { addr - 4 } as u8) & 0x0F;
                self.apu_io_w.write(port, value);
            }
            mmap::WMDATA => {
                let wram_addr = ((self.wm_add_h as usize) << 16)
                    | ((self.wm_add_m as usize) << 8)
                    | (self.wm_add_l as usize);
                self.wram[wram_addr] = value;
            }
            mmap::WMADDL => self.wm_add_l = value,
            mmap::WMADDM => self.wm_add_m = value,
            mmap::WMADDH => self.wm_add_h = value,
            mmap::JOYWR => self.joy_io.set_joy_wr(value),
            mmap::NMITIMEN => self.nmitimen = value,
            mmap::WRIO => self.joy_io.set_wr_io(value),
            mmap::WRMPYA => self.mpy_div.set_multiplicand(value),
            mmap::WRMPYB => self.mpy_div.set_multiplier_and_start_multiply(value),
            mmap::WRDIVL => self.mpy_div.set_dividend_low(value),
            mmap::WRDIVH => self.mpy_div.set_dividend_high(value),
            mmap::WRDIVB => self.mpy_div.set_divisor_and_start_division(value),
            mmap::HTIMEL => self.htime = (self.htime & 0xFF00) | value as u16,
            mmap::HTIMEH => self.htime = ((value as u16) << 8) | (self.htime & 0x00FF),
            mmap::VTIMEL => self.vtime = (self.vtime & 0xFF00) | value as u16,
            mmap::VTIMEH => self.vtime = ((value as u16) << 8) | (self.vtime & 0x00FF),
            mmap::MDMAEN => self.dma.write_mdma_en(value),
            mmap::HDMAEN => self.dma.write_hdma_en(value),
            mmap::MEMSEL => self.memsel = value,
            mmap::DMA_FIRST..=mmap::DMA_LAST => {
                // DMA
                self.dma.write(addr, value);
            }
            mmap::EXP_FIRST..=mmap::EXP_LAST => {
                // Expansion
                warn!("Write ${:06X}: Expansion not implemented", addr)
            }
            _ => error!(
                "System area write ${:04X}: Address unused or read-only",
                addr
            ),
        }
    }

    pub fn cpu_write8(&mut self, addr: u32, value: u8) {
        let bank = (addr >> 16) as usize;
        let bank_addr = (addr & 0x00FFFF) as usize;
        match bank {
            mmap::WS1_SYSLR_FIRST_BANK..=mmap::WS1_SYSLR_LAST_BANK => match bank_addr {
                mmap::SYS_FIRST..=mmap::SYS_LAST => self.cpu_write_sys(bank_addr, value),
                mmap::LOROM_FIRST..=mmap::LOROM_LAST => {
                    self.rom.write_ws1_lo_rom8(bank, bank_addr, value);
                }
                _ => unreachable!(),
            },
            mmap::WS1_HIROM_FIRST_BANK..=mmap::WS1_HIROM_LAST_BANK => {
                self.rom.write_ws1_hi_rom8(bank, bank_addr, value);
            }
            mmap::WRAM_FIRST_BANK..=mmap::WRAM_LAST_BANK => {
                self.wram[(bank - mmap::WRAM_FIRST_BANK) * 0x10000 + bank_addr] = value;
            }
            mmap::WS2_SYSLR_FIRST_BANK..=mmap::WS2_SYSLR_LAST_BANK => match bank_addr {
                mmap::SYS_FIRST..=mmap::SYS_LAST => self.cpu_write_sys(bank_addr, value),
                mmap::LOROM_FIRST..=mmap::LOROM_LAST => {
                    self.rom.write_ws2_lo_rom8(bank, bank_addr, value);
                }
                _ => unreachable!(),
            },
            mmap::WS2_HIROM_FIRST_BANK..=mmap::WS2_HIROM_LAST_BANK => {
                self.rom.write_ws2_hi_rom8(bank, bank_addr, value);
            }
            _ => unreachable!(),
        }
    }

    pub fn addr_wrapping_cpu_write16(&mut self, addr: u32, value: u16) {
        self.cpu_write8(addr, value as u8);
        self.cpu_write8(addr_wrapping_add(addr, 1), (value >> 8) as u8);
    }

    #[allow(dead_code)]
    pub fn addr_wrapping_cpu_write24(&mut self, addr: u32, value: u32) {
        self.cpu_write8(addr, value as u8);
        self.cpu_write8(addr_wrapping_add(addr, 1), (value >> 8) as u8);
        self.cpu_write8(addr_wrapping_add(addr, 2), (value >> 16) as u8);
    }

    pub fn bank_wrapping_cpu_write16(&mut self, addr: u32, value: u16) {
        self.cpu_write8(addr, value as u8);
        self.cpu_write8(bank_wrapping_add(addr, 1), (value >> 8) as u8);
    }

    pub fn bank_wrapping_cpu_write24(&mut self, addr: u32, value: u32) {
        self.cpu_write8(addr, value as u8);
        self.cpu_write8(bank_wrapping_add(addr, 1), (value >> 8) as u8);
        self.cpu_write8(bank_wrapping_add(addr, 2), (value >> 16) as u8);
    }

    pub fn page_wrapping_cpu_write16(&mut self, addr: u32, value: u16) {
        self.cpu_write8(addr, value as u8);
        self.cpu_write8(page_wrapping_add(addr, 1), (value >> 8) as u8);
    }

    pub fn page_wrapping_cpu_write24(&mut self, addr: u32, value: u32) {
        self.cpu_write8(addr, value as u8);
        self.cpu_write8(page_wrapping_add(addr, 1), (value >> 8) as u8);
        self.cpu_write8(page_wrapping_add(addr, 2), (value >> 16) as u8);
    }
}
pub fn addr_wrapping_add(addr: u32, offset: u32) -> u32 {
    (addr + offset) & 0x00FFFFFF
}

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

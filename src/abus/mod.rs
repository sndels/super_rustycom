mod rom;
use self::rom::Rom;
mod ppu_io;
use self::ppu_io::PpuIo;
mod mmap;

pub struct ABus {
    wram:   [u8; 128000],
    vram:   [u8; 64000],
    oam:    [u8; 544],
    cgram:  [u8; 512],
    rom:    Rom,
    ppu_io: PpuIo,
    // TODO: PPU1,2
    // TODO: PPU control regs
    // TODO: APU com regs
    // TODO: Joypad
    // TODO: Math regs
    // TODO: H-/V-blank regs and timers
    // TODO: DMA + control regs
}

impl ABus {
    pub fn new(rom_path: &String) -> ABus {
        ABus {
            wram:   [0; 128000],
            vram:   [0; 64000],
            oam:    [0; 544],
            cgram:  [0; 512],
            rom:    Rom::new(rom_path),
            ppu_io: PpuIo::new(),
        }
    }

    pub fn read8(&self, addr: u32) -> u8 {
        // TODO: Only LoROM, WRAM implemented, under 0x040000
        // TODO: LUT for speed?
        let bank = (addr >> 16) as usize;
        let bank_addr = (addr & 0x00FFFF) as usize;
        match bank {
            mmap::WS1_SYSLR_FIRST_BANK...mmap::WS1_SYSLR_LAST_BANK => {
                match bank_addr {
                    mmap::WRAM_MIRR_FIRST...mmap::WRAM_MIRR_LAST => {
                        panic!("Read {:#01$x}: WRAM not implemented", addr, 8)
                    }
                    mmap::PPU_IO_FIRST...mmap::PPU_IO_LAST => { // PPU IO
                        panic!("Read {:#01$x}: PPU IO not implemented", addr, 8)
                    }
                    mmap::APU_IO_FIRST...mmap::APU_IO_LAST => { // APU IO
                        panic!("Read {:#01$x}: APU IO not implemented", addr, 8)
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
                        self.rom.read8(bank * mmap::LOROM_FIRST + offset)
                    }
                    _ => panic!("Read {:#01$x}: Address unused or write-only", addr, 8)
                }
            }
            _ => panic!("Read {:#01$x}: Bank not implemented!", addr, 8)
        }
    }

    pub fn read16_le(&self, addr: u32) -> u16 {
        self.read8(addr) as u16 + ((self.read8(addr + 1) as u16) << 8)
    }

    pub fn write8(&mut self, value: u8, addr: u32) {
        let bank = (addr >> 16) as usize;
        let bank_addr = (addr & 0x00FFFF) as usize;
        match bank {
            mmap::WS1_SYSLR_FIRST_BANK...mmap::WS1_SYSLR_LAST_BANK => {
                match bank_addr {
                    mmap::WRAM_MIRR_FIRST...mmap::WRAM_MIRR_LAST => { // WRAM
                        self.wram[bank_addr] = value
                    }
                    mmap::PPU_IO_FIRST...mmap::PPU_IO_LAST => { // PPU IO
                        self.ppu_io.cpu_write(value, (bank_addr - mmap::PPU_IO_FIRST) as u8)
                    }
                    mmap::APU_IO_FIRST...mmap::APU_IO_LAST => { // APU IO
                        panic!("Write {:#01$x}: APU IO not implemented", addr, 8)
                    }
                    mmap::WMDATA   => panic!("Write {:#01$x}: WMDATA not implemented", addr, 8),
                    mmap::WMADDL   => panic!("Write {:#01$x}: WMADDL not implemented", addr, 8),
                    mmap::WMADDM   => panic!("Write {:#01$x}: WMADDM not implemented", addr, 8),
                    mmap::WMADDH   => panic!("Write {:#01$x}: WMADDH not implemented", addr, 8),
                    mmap::JOYWR    => panic!("Write {:#01$x}: JOYWR not implemented", addr, 8),
                    mmap::NMITIMEN => panic!("Write {:#01$x}: NMITIMEN not implemented", addr, 8),
                    mmap::WRIO     => panic!("Write {:#01$x}: WRIO not implemented", addr, 8),
                    mmap::WRMPYA   => panic!("Write {:#01$x}: WRMPYA not implemented", addr, 8),
                    mmap::WRMPYB   => panic!("Write {:#01$x}: WRMPYB not implemented", addr, 8),
                    mmap::WRDIVL   => panic!("Write {:#01$x}: WRDIVL not implemented", addr, 8),
                    mmap::WRDIVH   => panic!("Write {:#01$x}: WRDIVH not implemented", addr, 8),
                    mmap::WRDIVB   => panic!("Write {:#01$x}: WRDIVB not implemented", addr, 8),
                    mmap::HTIMEL   => panic!("Write {:#01$x}: HTIMEL not implemented", addr, 8),
                    mmap::HTIMEH   => panic!("Write {:#01$x}: HTIMEH not implemented", addr, 8),
                    mmap::VTIMEL   => panic!("Write {:#01$x}: VTIMEL not implemented", addr, 8),
                    mmap::VTIMEH   => panic!("Write {:#01$x}: VTIMEH not implemented", addr, 8),
                    mmap::MDMAEN   => panic!("Write {:#01$x}: MDMAEN not implemented", addr, 8),
                    mmap::HDMAEN   => panic!("Write {:#01$x}: HDMAEN not implemented", addr, 8),
                    mmap::MEMSEL   => panic!("Write {:#01$x}: HDMAEN not implemented", addr, 8),
                    mmap::DMA_FIRST...mmap::DMA_LAST => { // DMA
                        panic!("Write {:#01$x}: DMA not implemented", addr, 8)
                    }
                    mmap::EXP_FIRST...mmap::EXP_LAST => { // Expansion
                        panic!("Write {:#01$x}: Expansion not implemented", addr, 8)
                    }
                    _ => panic!("Write {:#01$x}: Address unused or read-only", addr, 8)
                }
            }
            _ => panic!("Write {:#01$x}: Bank not implemented", addr, 8)
        }
    }

    pub fn write16_le(&mut self, value: u16, addr: u32) {
        self.write8((value & 0xFF) as u8, addr);
        self.write8((value >> 8) as u8, addr + 1);
    }

    pub fn push_stack(&mut self, value: u16, addr: u16) {
        self.write8((value >> 8) as u8, addr as u32);
        self.write8((value & 0xFF) as u8, (addr - 1) as u32);
    }
}

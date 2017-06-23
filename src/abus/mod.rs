mod rom;
use self::rom::Rom;
mod ppu_io;
use self::ppu_io::PpuIo;

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
        if addr < 0x040000 {
            let bank: usize = (addr >> 16) as usize;
            let mut offset: usize = (addr & 0x00FFFF) as usize;
            // WS1 LoROM
            if offset > 0x7FFF {
                offset -= 0x8000;
                self.rom.read8(bank * 0x8000 + offset)
            } else {
                panic!("Offset {} not implemented in system area", offset);
            }
        } else {
            panic!("Access to addr {} no implemented!", addr);
        }
    }

    pub fn read16_le(&self, addr: u32) -> u16 {
        self.read8(addr) as u16 + ((self.read8(addr + 1) as u16) << 8)
    }

    pub fn write8(&mut self, value: u8, addr: u32) {
        let bank: usize = (addr >> 16) as usize;
        let mut offset: usize = (addr & 0x00FFFF) as usize;
        // Panic if the address isn't implemented or is read-only
        if bank > 0x3F {
            panic!("Write to addr {:x} not implemented", addr);
        } else if offset > 0x7FFF {
            panic!("Attempted write to LoROM at {:x}", addr);
        } else if (offset > 0x1FFF && offset < 0x2100) ||
                  (offset > 0x21FF && offset < 0x4000) {
            panic!("System area {:x} is not used", addr);
        } else if (offset > 0x213F && offset < 0x2200) ||
                  (offset > 0x3FFF && offset < 0x6000) {
            panic!("Write to i/o not implemented at addr {:x}", addr);
        } else if offset > 0x5FFF && offset < 0x8000 {
            panic!("Write to expansion not implemented at addr {:x}", addr);
        } else if offset < 0x2000 {
            // WRAM
            self.wram[offset] = value;
        } else {
            // IO PPU
            self.ppu_io.cpu_write(value, (offset - 0x2100) as u8);
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

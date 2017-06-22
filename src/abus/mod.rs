mod rom;
use self::rom::Rom;

pub struct ABus {
    wram:   [u8; 128000],
    vram:   [u8; 64000],
    oam:    [u8; 544],
    cgram:  [u8; 512],
    rom: Rom,
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
            wram:  [0; 128000],
            vram:  [0; 64000],
            oam:   [0; 544],
            cgram: [0; 512],
            rom:   Rom::new(rom_path),
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
}

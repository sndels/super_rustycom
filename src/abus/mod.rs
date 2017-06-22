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
}

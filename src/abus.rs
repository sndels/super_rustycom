use dma::Dma;
use joypad::JoyIo;
use mpydiv::MpyDiv;
use ppu_io::PpuIo;
use rom::Rom;

pub struct ABus {
    pub wram: [u8; 131072],
    pub vram: [u8; 65536],
    pub oam: [u8; 544],
    pub cgram: [u8; 512],
    pub rom: Rom,
    pub mpy_div: MpyDiv,
    pub ppu_io: PpuIo,
    pub joy_io: JoyIo,
    pub dma: Dma,
    // "On-chip" CPU W
    pub nmitimen: u8,
    pub htime: u16,
    pub vtime: u16,
    pub memsel: u8,
    // APU IO
    pub apu_io0: u8,
    pub apu_io1: u8,
    pub apu_io2: u8,
    pub apu_io3: u8,
    // WRAM access
    pub wm_add_l: u8,
    pub wm_add_m: u8,
    pub wm_add_h: u8,
    // Blank regs
    pub rd_nmi: u8,
    pub time_up: u8,
    pub hvb_joy: u8,
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
}

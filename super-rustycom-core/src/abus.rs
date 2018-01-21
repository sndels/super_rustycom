use dma::Dma;
use joypad::JoyIo;
use mpydiv::MpyDiv;
use ppu_io::PpuIo;
use rom::Rom;

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
    /// "Work"RAM
    pub wram: Box<[u8]>,
    /// VideoRAM
    pub vram: Box<[u8]>,
    /// Object Attribute Memory, used for sprite info
    pub oam: Box<[u8]>,
    /// Color palette memory
    pub cgram: Box<[u8]>,
    /// The loaded ROM instance
    pub rom: Rom,
    /// Combined multiplication and division register
    pub mpy_div: MpyDiv,
    /// Registers for CPU<->PPU communication
    pub ppu_io: PpuIo,
    /// Joypad registers
    pub joy_io: JoyIo,
    /// Dma controller
    pub dma: Dma,
    /// Interrupt Enable and Joypad Request
    pub nmitimen: u8,
    /// H-count timer setting
    pub htime: u16,
    /// V-count timer setting
    pub vtime: u16,
    /// Memory-2 waitstate control
    pub memsel: u8,
    /// CPU<->APU communication port 0
    ///
    /// CPU writes to low bits and reads from high, APU vice versa
    pub apu_io0: u16,
    /// CPU<->APU communication port 1
    ///
    /// CPU writes to low bits and reads from high, APU vice versa
    pub apu_io1: u16,
    /// CPU<->APU communication port 2
    ///
    /// CPU writes to low bits and reads from high, APU vice versa
    pub apu_io2: u16,
    /// CPU<->APU communication port 3
    ///
    /// CPU writes to low bits and reads from high, APU vice versa
    pub apu_io3: u16,
    /// Lower 8bit of WRAM address used by WMDATA reads
    pub wm_add_l: u8,
    /// Middle 8bit of WRAM address used by WMDATA reads
    pub wm_add_m: u8,
    /// Upper 1bit of WRAM address used by WMDATA reads
    pub wm_add_h: u8,
    /// V-blank NMI flag and CPU version number
    pub rd_nmi: u8,
    /// H/V-timer IRG flag
    pub time_up: u8,
    /// H/V-blank flag and joypad busy flag
    pub hvb_joy: u8,
}

impl ABus {
    /// Initializes a new instance with default values and loads the given ROM
    pub fn new(rom_path: &String) -> ABus {
        // TODO: Randomize values?
        ABus {
            wram: Box::new([0; WRAM_SIZE]),
            vram: Box::new([0; VRAM_SIZE]),
            oam: Box::new([0; OAM_SIZE]),
            cgram: Box::new([0; CGRAM_SIZE]),
            rom: Rom::new(rom_path),
            mpy_div: MpyDiv::new(),
            ppu_io: PpuIo::new(),
            joy_io: JoyIo::new(),
            dma: Dma::new(),
            nmitimen: 0x00,
            htime: 0x01FF,
            vtime: 0x01FF,
            memsel: 0x00,
            apu_io0: 0x0000,
            apu_io1: 0x0000,
            apu_io2: 0x0000,
            apu_io3: 0x0000,
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
            apu_io0: 0x0000,
            apu_io1: 0x0000,
            apu_io2: 0x0000,
            apu_io3: 0x0000,
            wm_add_l: 0x00,
            wm_add_m: 0x00,
            wm_add_h: 0x00,
            rd_nmi: 0x00,
            time_up: 0x00,
            hvb_joy: 0x00,
        }
    }
}

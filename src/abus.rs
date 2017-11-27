use dma::Dma;
use rom::Rom;
use ppu_io::PpuIo;

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

pub struct MpyDiv {
    pub mpy_a: u8,
    pub mpy_b: u8,
    pub dividend: u16,
    pub divisor: u8,
    pub mpy_res: u16, // Doubles as division reminder
    pub div_res: u16,
}

impl MpyDiv {
    pub fn new() -> MpyDiv {
        MpyDiv {
            mpy_a: 0xFF,
            mpy_b: 0xFF,
            dividend: 0xFFFF,
            divisor: 0xFF,
            mpy_res: 0x0000, // TODO: Check result inits
            div_res: 0x0000,
        }
    }

    pub fn set_mpy_a(&mut self, value: u8) { self.mpy_a = value; }

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

    pub fn get_mpy_res_low(&self) -> u8 { self.mpy_res as u8 }

    pub fn get_mpy_res_high(&self) -> u8 { (self.mpy_res >> 8) as u8 }

    pub fn get_div_res_low(&self) -> u8 { self.div_res as u8 }

    pub fn get_div_res_high(&self) -> u8 { (self.div_res >> 8) as u8 }
}

pub struct JoyIo {
    pub joy_wr: u8,
    pub joy_a: u8,
    pub joy_b: u8,
    pub wr_io: u8,
    pub rd_io: u8,
    pub joy_1l: u8,
    pub joy_1h: u8,
    pub joy_2l: u8,
    pub joy_2h: u8,
    pub joy_3l: u8,
    pub joy_3h: u8,
    pub joy_4l: u8,
    pub joy_4h: u8,
}

impl JoyIo {
    pub fn new() -> JoyIo {
        JoyIo {
            joy_wr: 0x00,
            joy_a: 0x00,
            joy_b: 0x00,
            wr_io: 0xFF,
            rd_io: 0x00,
            joy_1l: 0x00,
            joy_1h: 0x00,
            joy_2l: 0x00,
            joy_2h: 0x00,
            joy_3l: 0x00,
            joy_3h: 0x00,
            joy_4l: 0x00,
            joy_4h: 0x00,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

mod io;

pub use io::Io;

use crate::abus::ABus;
use crate::cpu::W65c816s;

pub struct Ppu {
    h: u32,
    v: u32,
}

impl Default for Ppu {
    fn default() -> Self {
        Self { h: 0, v: 0 }
    }
}

const MASTER_CLOCKS_PER_DOT: u32 = 4;
const LINE_DOT_COUNT: u32 = 340;
const VBLANK_LINE: u32 = 225;
const LINE_COUNT: u32 = 261;

impl Ppu {
    pub fn run(&mut self, master_clocks: u32, cpu: &mut W65c816s, abus: &mut ABus) {
        let old_h = self.h;
        self.h = (self.h + master_clocks / MASTER_CLOCKS_PER_DOT) % LINE_DOT_COUNT;
        let h_carry = if old_h > self.h { 1 } else { 0 };
        // TODO:
        // HBLANK

        // TODO:
        // Extra line when interlace field = 0
        self.v = (self.v + h_carry + master_clocks / MASTER_CLOCKS_PER_DOT / LINE_DOT_COUNT)
            % LINE_COUNT;
        if self.v >= VBLANK_LINE {
            abus.set_vblank();
            abus.set_vblank_nmi();
            if abus.nmi_enabled() {
                cpu.request_nmi(abus);
            }
        } else {
            // TODO:
            // This should be only done once per frame if/when we emulate HBLANK
            abus.clear_vblank_nmi();
            abus.clear_vblank();
        };

        // TODO:
        // V-IRQ
        // HV-IRQ
    }

    pub fn clocks_until_signal_change(&self) -> u32 {
        self.clocks_until_vblank()
            .min(self.clocks_until_vblank_clear())
    }

    fn clocks_until_vblank(&self) -> u32 {
        let line_dots = LINE_DOT_COUNT - self.h;

        let lines = if self.v < VBLANK_LINE {
            VBLANK_LINE - self.v
        } else {
            self.v
        };

        (line_dots + lines * LINE_DOT_COUNT) * MASTER_CLOCKS_PER_DOT
    }

    fn clocks_until_vblank_clear(&self) -> u32 {
        let line_dots = LINE_DOT_COUNT - self.h;

        let lines = LINE_COUNT - self.v;

        (line_dots + lines * LINE_DOT_COUNT) * MASTER_CLOCKS_PER_DOT
    }
}

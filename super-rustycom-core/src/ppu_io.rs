use crate::mmap;

pub struct PpuIo {
    // Cpu write
    pub ini_disp: u8,
    pub ob_sel: u8,
    pub oam_add_l: u8,
    pub oam_add_h: u8,
    pub oam_data: DoubleReg,
    pub bg_mode: u8,
    pub mosaic: u8,
    pub bg1_sc: u8,
    pub bg2_sc: u8,
    pub bg3_sc: u8,
    pub bg4_sc: u8,
    pub bg12_nba: u8,
    pub bg34_nba: u8,
    pub bg1_hofs: DoubleReg,
    pub bg1_vofs: DoubleReg,
    pub bg2_hofs: DoubleReg,
    pub bg2_vofs: DoubleReg,
    pub bg3_hofs: DoubleReg,
    pub bg3_vofs: DoubleReg,
    pub bg4_hofs: DoubleReg,
    pub bg4_vofs: DoubleReg,
    pub vm_ain: u8,
    pub vm_add_l: u8,
    pub vm_add_h: u8,
    pub vm_data_l: u8,
    pub vm_data_h: u8,
    pub m7_sel: u8,
    pub m7_a: DoubleReg,
    pub m7_b: DoubleReg,
    pub m7_c: DoubleReg,
    pub m7_d: DoubleReg,
    pub m7_x: DoubleReg,
    pub m7_y: DoubleReg,
    pub cg_add: u8,
    pub cg_data: DoubleReg,
    pub w12_sel: u8,
    pub w34_sel: u8,
    pub wobj_sel: u8,
    pub wh0: u8,
    pub wh1: u8,
    pub wh2: u8,
    pub wh3: u8,
    pub wbg_log: u8,
    pub wobj_log: u8,
    pub tm: u8,
    pub ts: u8,
    pub tmw: u8,
    pub tsw: u8,
    pub cg_wsel: u8,
    pub cg_adsub: u8,
    pub col_data: u8,
    pub setini: u8,
    // Cpu read
    pub mpy_l: u8,
    pub mpy_m: u8,
    pub mpy_h: u8,
    pub sl_hv: u8,
    pub rd_oam: DoubleReg,
    pub rd_vram_l: u8,
    pub rd_vram_h: u8,
    pub rd_cgram: DoubleReg,
    pub op_hct: DoubleReg,
    pub op_vct: DoubleReg,
    pub stat_77: u8,
    pub stat_78: u8,
}

impl PpuIo {
    pub fn new() -> PpuIo {
        // TODO: Randomize non spec'd values?
        let mut ppu_io = PpuIo {
            ini_disp: 0x08,
            ob_sel: 0x00,
            oam_add_l: 0x00,
            oam_add_h: 0x00,
            oam_data: DoubleReg::new(),
            bg_mode: 0x0F,
            mosaic: 0x00,
            bg1_sc: 0x00,
            bg2_sc: 0x00,
            bg3_sc: 0x00,
            bg4_sc: 0x00,
            bg12_nba: 0x00,
            bg34_nba: 0x00,
            bg1_hofs: DoubleReg::new(),
            bg1_vofs: DoubleReg::new(),
            bg2_hofs: DoubleReg::new(),
            bg2_vofs: DoubleReg::new(),
            bg3_hofs: DoubleReg::new(),
            bg3_vofs: DoubleReg::new(),
            bg4_hofs: DoubleReg::new(),
            bg4_vofs: DoubleReg::new(),
            vm_ain: 0x0F,
            vm_add_l: 0x00,
            vm_add_h: 0x00,
            vm_data_l: 0x00,
            vm_data_h: 0x00,
            m7_sel: 0x00,
            m7_a: DoubleReg::new(),
            m7_b: DoubleReg::new(),
            m7_c: DoubleReg::new(),
            m7_d: DoubleReg::new(),
            m7_x: DoubleReg::new(),
            m7_y: DoubleReg::new(),
            cg_add: 0x00,
            cg_data: DoubleReg::new(),
            w12_sel: 0x00,
            w34_sel: 0x00,
            wobj_sel: 0x00,
            wh0: 0x00,
            wh1: 0x00,
            wh2: 0x00,
            wh3: 0x00,
            wbg_log: 0x00,
            wobj_log: 0x00,
            tm: 0x00,
            ts: 0x00,
            tmw: 0x00,
            tsw: 0x00,
            cg_wsel: 0x00,
            cg_adsub: 0x00,
            col_data: 0x00,
            setini: 0x00,
            mpy_l: 0x01,
            mpy_m: 0x00,
            mpy_h: 0x00,
            sl_hv: 0x00,
            rd_oam: DoubleReg::new(),
            rd_vram_l: 0x00,
            rd_vram_h: 0x00,
            rd_cgram: DoubleReg::new(),
            op_hct: DoubleReg::new(),
            op_vct: DoubleReg::new(),
            stat_77: 0x00,
            stat_78: 0x00,
        };
        ppu_io.m7_a.write(0xFF);
        ppu_io.m7_a.write(0x00);
        ppu_io.m7_b.write(0xFF);
        ppu_io.m7_b.write(0x00);
        ppu_io.op_hct.write(0xFF);
        ppu_io.op_hct.write(0x01);
        ppu_io.op_vct.write(0xFF);
        ppu_io.op_vct.write(0x01);
        ppu_io
    }

    pub fn read(&mut self, addr: usize) -> u8 {
        match addr {
            mmap::INIDISP => self.ini_disp,
            mmap::OBSEL => self.ob_sel,
            mmap::OAMADDL => self.oam_add_l,
            mmap::OAMADDH => self.oam_add_h,
            mmap::OAMDATA => self.oam_data.read(),
            mmap::BGMODE => self.bg_mode,
            mmap::MOSAIC => self.mosaic,
            mmap::BG1SC => self.bg1_sc,
            mmap::BG2SC => self.bg2_sc,
            mmap::BG3SC => self.bg3_sc,
            mmap::BG4SC => self.bg4_sc,
            mmap::BG12NBA => self.bg12_nba,
            mmap::BG34NBA => self.bg34_nba,
            mmap::BG1HOFS => self.bg1_hofs.read(),
            mmap::BG1VOFS => self.bg1_vofs.read(),
            mmap::BG2HOFS => self.bg2_hofs.read(),
            mmap::BG2VOFS => self.bg2_vofs.read(),
            mmap::BG3HOFS => self.bg3_hofs.read(),
            mmap::BG3VOFS => self.bg3_vofs.read(),
            mmap::BG4HOFS => self.bg4_hofs.read(),
            mmap::BG4VOFS => self.bg4_vofs.read(),
            mmap::VMAIN => self.vm_ain,
            mmap::VMADDL => self.vm_add_l,
            mmap::VMADDH => self.vm_add_h,
            mmap::VMDATAL => self.vm_data_l,
            mmap::VMDATAH => self.vm_data_h,
            mmap::M7SEL => self.m7_sel,
            mmap::M7A => self.m7_a.read(),
            mmap::M7B => self.m7_b.read(),
            mmap::M7C => self.m7_c.read(),
            mmap::M7D => self.m7_d.read(),
            mmap::M7X => self.m7_x.read(),
            mmap::M7Y => self.m7_y.read(),
            mmap::CGADD => self.cg_add,
            mmap::CGDATA => self.cg_data.read(),
            mmap::W12SEL => self.w12_sel,
            mmap::W34SEL => self.w34_sel,
            mmap::WOBJSEL => self.wobj_sel,
            mmap::WH0 => self.wh0,
            mmap::WH1 => self.wh1,
            mmap::WH2 => self.wh2,
            mmap::WH3 => self.wh3,
            mmap::WBGLOG => self.wbg_log,
            mmap::WOBJLOG => self.wobj_log,
            mmap::TM => self.tm,
            mmap::TS => self.ts,
            mmap::TMW => self.tmw,
            mmap::TSW => self.tsw,
            mmap::CGWSEL => self.cg_wsel,
            mmap::CGADSUB => self.cg_adsub,
            mmap::COLDATA => self.col_data,
            mmap::SETINI => self.setini,
            mmap::MPYL => self.mpy_l,
            mmap::MPYM => self.mpy_m,
            mmap::MPYH => self.mpy_h,
            mmap::SLHV => self.sl_hv,
            mmap::RDOAM => self.rd_oam.read(),
            mmap::RDVRAML => self.rd_vram_l,
            mmap::RDVRAMH => self.rd_vram_h,
            mmap::RDCGRAM => self.rd_cgram.read(),
            mmap::OPHCT => self.op_hct.read(),
            mmap::OPVCT => self.op_vct.read(),
            mmap::STAT77 => self.stat_77,
            mmap::STAT78 => self.stat_78,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: usize, value: u8) {
        match addr {
            mmap::INIDISP => self.ini_disp = value,
            mmap::OBSEL => self.ob_sel = value,
            mmap::OAMADDL => self.oam_add_l = value,
            mmap::OAMADDH => self.oam_add_h = value,
            mmap::OAMDATA => self.oam_data.write(value),
            mmap::BGMODE => self.bg_mode = value,
            mmap::MOSAIC => self.mosaic = value,
            mmap::BG1SC => self.bg1_sc = value,
            mmap::BG2SC => self.bg2_sc = value,
            mmap::BG3SC => self.bg3_sc = value,
            mmap::BG4SC => self.bg4_sc = value,
            mmap::BG12NBA => self.bg12_nba = value,
            mmap::BG34NBA => self.bg34_nba = value,
            mmap::BG1HOFS => self.bg1_hofs.write(value),
            mmap::BG1VOFS => self.bg1_vofs.write(value),
            mmap::BG2HOFS => self.bg2_hofs.write(value),
            mmap::BG2VOFS => self.bg2_vofs.write(value),
            mmap::BG3HOFS => self.bg3_hofs.write(value),
            mmap::BG3VOFS => self.bg3_vofs.write(value),
            mmap::BG4HOFS => self.bg4_hofs.write(value),
            mmap::BG4VOFS => self.bg4_vofs.write(value),
            mmap::VMAIN => self.vm_ain = value,
            mmap::VMADDL => self.vm_add_l = value,
            mmap::VMADDH => self.vm_add_h = value,
            mmap::VMDATAL => self.vm_data_l = value,
            mmap::VMDATAH => self.vm_data_h = value,
            mmap::M7SEL => self.m7_sel = value,
            mmap::M7A => self.m7_a.write(value),
            mmap::M7B => self.m7_b.write(value),
            mmap::M7C => self.m7_c.write(value),
            mmap::M7D => self.m7_d.write(value),
            mmap::M7X => self.m7_x.write(value),
            mmap::M7Y => self.m7_y.write(value),
            mmap::CGADD => self.cg_add = value,
            mmap::CGDATA => self.cg_data.write(value),
            mmap::W12SEL => self.w12_sel = value,
            mmap::W34SEL => self.w34_sel = value,
            mmap::WOBJSEL => self.wobj_sel = value,
            mmap::WH0 => self.wh0 = value,
            mmap::WH1 => self.wh1 = value,
            mmap::WH2 => self.wh2 = value,
            mmap::WH3 => self.wh3 = value,
            mmap::WBGLOG => self.wbg_log = value,
            mmap::WOBJLOG => self.wobj_log = value,
            mmap::TM => self.tm = value,
            mmap::TS => self.ts = value,
            mmap::TMW => self.tmw = value,
            mmap::TSW => self.tsw = value,
            mmap::CGWSEL => self.cg_wsel = value,
            mmap::CGADSUB => self.cg_adsub = value,
            mmap::COLDATA => self.col_data = value,
            mmap::SETINI => self.setini = value,
            mmap::MPYL => self.mpy_l = value,
            mmap::MPYM => self.mpy_m = value,
            mmap::MPYH => self.mpy_h = value,
            mmap::SLHV => self.sl_hv = value,
            mmap::RDOAM => self.rd_oam.write(value),
            mmap::RDVRAML => self.rd_vram_l = value,
            mmap::RDVRAMH => self.rd_vram_h = value,
            mmap::RDCGRAM => self.rd_cgram.write(value),
            mmap::OPHCT => self.op_hct.write(value),
            mmap::OPVCT => self.op_vct.write(value),
            mmap::STAT77 => self.stat_77 = value,
            mmap::STAT78 => self.stat_78 = value,
            _ => unreachable!(),
        }
    }
}

pub struct DoubleReg {
    value: u16,
    high_active: bool, // TODO: Should there be separate flags for read and write?
}

impl DoubleReg {
    pub fn new() -> DoubleReg {
        DoubleReg {
            value: 0,
            high_active: false,
        }
    }

    pub fn read(&mut self) -> u8 {
        let value = if self.high_active {
            (self.value >> 8) as u8
        } else {
            (self.value & 0xFF) as u8
        };
        self.high_active = !self.high_active;
        value
    }

    pub fn write(&mut self, value: u8) {
        if self.high_active {
            self.value = (self.value & 0x00FF) | ((value as u16) << 8);
        } else {
            self.value = (self.value & 0xFF00) | value as u16;
        }
        self.high_active = !self.high_active;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_reg_write() {
        let mut reg = DoubleReg::new();
        reg.write(0x34);
        reg.write(0x12);
        assert_eq!(0x1234, reg.value);
        reg.write(0xCD);
        reg.write(0xAB);
        assert_eq!(0xABCD, reg.value);
    }

    #[test]
    fn double_reg_read() {
        let mut reg = DoubleReg::new();
        reg.value = 0x1234;
        assert_eq!(0x34, reg.read());
        assert_eq!(0x12, reg.read());
        reg.value = 0xABCD;
        assert_eq!(0xCD, reg.read());
        assert_eq!(0xAB, reg.read());
    }
}

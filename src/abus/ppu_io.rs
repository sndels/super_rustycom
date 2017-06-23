#[derive(Copy, Clone)]
struct DoubleReg {
    value: u16,
    high_active: bool, // TODO: Should there be separate flags for read and write?
}

impl DoubleReg {
    pub fn new() -> DoubleReg {
        DoubleReg {
            value:           0,
            high_active: false,
        }
    }

    pub fn read(&mut self) -> u8 {
        let value = if self.high_active { (self.value >> 8) as u8 }
                    else { (self.value & 0xFF) as u8 };
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

pub struct PpuIo {
    ports:    [u8; 64],
    bg_ofs:   [DoubleReg; 8],
    m7:       [DoubleReg; 6],
    cg_data:  DoubleReg,
    rd_oam:   DoubleReg,
    rd_cgram: DoubleReg,
    op_hct:   DoubleReg,
    op_vct:   DoubleReg,
}

impl PpuIo {
    // TODO: Randomize memory?
    // Initial values from fullsnes
    pub fn new() -> PpuIo {
        let mut ppu_io: PpuIo = PpuIo {
            ports:    [0; 64],
            bg_ofs:   [DoubleReg::new(); 8],
            m7:       [DoubleReg::new(); 6],
            cg_data:  DoubleReg::new(),
            rd_oam:   DoubleReg::new(),
            rd_cgram: DoubleReg::new(),
            op_hct:   DoubleReg::new(),
            op_vct:   DoubleReg::new(),
        };
        ppu_io.ports[INIDISP as usize] = 0x08;
        ppu_io.ports[BGMODE as usize]  = 0x0F;
        ppu_io.ports[VMAIN as usize]  |= 0xF;
        ppu_io.m7[0].write(0xFF);
        ppu_io.m7[0].write(0x00);
        ppu_io.m7[1].write(0xFF);
        ppu_io.m7[1].write(0x00);
        ppu_io.ports[SETINI as usize] = 0x00;
        ppu_io.ports[MPYL as usize]   = 0x01;
        ppu_io.ports[MPYM as usize]   = 0x00;
        ppu_io.ports[MPYH as usize]   = 0x00;
        ppu_io.op_hct.write(0xFF);
        ppu_io.op_hct.write(0x01);
        ppu_io.op_vct.write(0xFF);
        ppu_io.op_vct.write(0x01);
        ppu_io.ports[STAT78 as usize] |= 0b0100_000;
        ppu_io
    }

    // TODO: Check for side-effects
    pub fn cpu_write(&mut self, value: u8, addr: u8) {
        // Panic if accessing read-only regs
        if addr > 0x33 {
            panic!("Cpu tried writing to read-only port at {:x}", addr);
        }

        // Match on write-twice regs, default to reg array
        match addr {
            BG1HOFS => self.bg_ofs[0].write(value),
            BG1VOFS => self.bg_ofs[1].write(value),
            BG2HOFS => self.bg_ofs[2].write(value),
            BG2VOFS => self.bg_ofs[3].write(value),
            BG3HOFS => self.bg_ofs[4].write(value),
            BG3VOFS => self.bg_ofs[5].write(value),
            BG4HOFS => self.bg_ofs[6].write(value),
            BG4VOFS => self.bg_ofs[7].write(value),
            M7A     => self.m7[0].write(value),
            M7B     => self.m7[1].write(value),
            M7C     => self.m7[2].write(value),
            M7D     => self.m7[3].write(value),
            M7X     => self.m7[4].write(value),
            M7Y     => self.m7[5].write(value),
            CGDATA  => self.cg_data.write(value),
            _       => self.ports[addr as usize] = value
        }
    }

    // TODO: Check for side-effects
    pub fn cpu_read(&mut self, addr: u8) -> u8 {
        // Panic if accessing write-only regs
        if addr < 0x34 {
            panic!("Cpu tried reading from write-only port at {:x}", addr);
        }

        // Match on read-twice regs, default to reg array
        // TODO: Enum to int matching if it gets back to rust
        match addr {
            RDOAM   => self.rd_oam.read(),
            RDCGRAM => self.rd_cgram.read(),
            OPHCT   => self.op_hct.read(),
            OPVCT   => self.op_vct.read(),
            _       => self.ports[addr as usize]
        }
    }
}

// Port map
const INIDISP: u8 = 0x00;
const OBSEL: u8   = 0x01;
const OAMADDL: u8 = 0x02;
const OAMADDH: u8 = 0x03;
const OAMDATA: u8 = 0x04;
const BGMODE: u8  = 0x05;
const MOSAIC: u8  = 0x06;
const BG1SC: u8   = 0x07;
const BG2SC: u8   = 0x08;
const BG3SC: u8   = 0x09;
const BG4SC: u8   = 0x0A;
const BG12NBA: u8 = 0x0B;
const BG34NBA: u8 = 0x0C;
const BG1HOFS: u8 = 0x0D;
const BG1VOFS: u8 = 0x0E;
const BG2HOFS: u8 = 0x0F;
const BG2VOFS: u8 = 0x10;
const BG3HOFS: u8 = 0x11;
const BG3VOFS: u8 = 0x12;
const BG4HOFS: u8 = 0x13;
const BG4VOFS: u8 = 0x14;
const VMAIN: u8   = 0x15;
const VMADDL: u8  = 0x16;
const VMADDH: u8  = 0x17;
const VMDATAL: u8 = 0x18;
const VMDATAH: u8 = 0x19;
const M7SEL: u8   = 0x1A;
const M7A: u8     = 0x1B;
const M7B: u8     = 0x1C;
const M7C: u8     = 0x1D;
const M7D: u8     = 0x1E;
const M7X: u8     = 0x1F;
const M7Y: u8     = 0x20;
const CGADD: u8   = 0x21;
const CGDATA: u8  = 0x22;
const W12SEL: u8  = 0x23;
const W34SEL: u8  = 0x24;
const WOBJSEL: u8 = 0x25;
const WH0: u8     = 0x26;
const WH1: u8     = 0x27;
const WH2: u8     = 0x28;
const WH3: u8     = 0x29;
const WBGLOG: u8  = 0x2A;
const WOBJLOG: u8 = 0x2B;
const TM: u8      = 0x2C;
const TS: u8      = 0x2D;
const TMW: u8     = 0x2E;
const TSW: u8     = 0x2F;
const CGWSEL: u8  = 0x30;
const CGADSUB: u8 = 0x31;
const COLDATA: u8 = 0x32;
const SETINI: u8  = 0x33;
const MPYL: u8    = 0x34;
const MPYM: u8    = 0x35;
const MPYH: u8    = 0x36;
const SLHV: u8    = 0x37;
const RDOAM: u8   = 0x38;
const RDVRAML: u8 = 0x39;
const RDVRAMH: u8 = 0x3A;
const RDCGRAM: u8 = 0x3B;
const OPHCT: u8   = 0x3C;
const OPVCT: u8   = 0x3D;
const STAT77: u8  = 0x3E;
const STAT78: u8  = 0x3F;

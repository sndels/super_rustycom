use abus::ABus;

enum Interrupt {
    Cop16 =   0xE4,
    Brk16 =   0xE6,
    Abort16 = 0xE8,
    Nmi16 =   0xEA,
    Irq16 =   0xEE,
    Cop8 =    0xF4,
    Abort8 =  0xF8,
    Nmi8 =    0xFA,
    Reset8 =  0xFC,
    IrqBrk8 = 0xFE,
}

enum PFlag {
    C = 0b0000_0001,
    Z = 0b0000_0010,
    I = 0b0000_0100,
    D = 0b0000_1000,
    X = 0b0001_0000,
    M = 0b0010_0000,
    V = 0b0100_0000,
    N = 0b1000_0000,
}

#[derive(Debug)]
pub struct Cpu {
    a:  u16,  // Accumulator
    x:  u16,  // Index register
    y:  u16,  // Index register
    pc: u16,  // Program counter
    s:  u16,  // Stack pointer
    p:  u8,   // Processor status register
    d:  u16,  // Zeropage offset
    pb: u8,   // Data bank
    db: u8,   // Program cunter bank
    e:  bool, // Emulation mode
}

impl Cpu {
    pub fn new(abus: &mut ABus) -> Cpu {
        Cpu {
            a:  0x00,
            x:  0x00,
            y:  0x00,
            pc: abus.read16_le(0x00FF00 + Interrupt::Reset8 as u32), // TODO: This only applies to LoROM
            s:  0x01FF,
            p:  PFlag::M as u8 | PFlag::X as u8 | PFlag::I as u8,
            d : 0x00,
            pb: 0x0,
            db: 0x0,
            e:  true,
        }
    }

    pub fn run(&mut self, abus: &mut ABus) {
        loop {
            let addr = ((self.pb as u32) << 16) + self.pc as u32;
            let opcode = abus.read8(addr);
            self.execute(opcode, addr, abus);
        }
    }

    // TODO: Page wrapping in emulation mode?
    fn execute(&mut self, opcode: u8, addr: u32, abus: &mut ABus) {
        match opcode {
            0x18 => self.op_clc(),
            0x20 => self.op_jsr(addr, abus),
            0x78 => self.op_sei(),
            0x8D => self.op_sta(addr, abus),
            0x9A => self.op_txs(),
            0xA2 => self.op_ldx(addr, abus),
            0xA9 => self.op_lda(addr, abus),
            0xC2 => self.op_rep(addr, abus),
            0xE2 => self.op_sep(addr, abus),
            0xFB => self.op_xce(),
            _ => panic!("Unknown opcode {:X}!", opcode),
        }
    }

    fn op_clc(&mut self) {
        println!("0x{:x} CLC", ((self.pb as u32) << 16) + self.pc as u32);
        self.p &= !(PFlag::C as u8);
        self.pc += 1;
    }

    fn op_jsr(&mut self, addr: u32, abus: &mut ABus) {
        let sub_addr = abus.read16_le(addr + 1);
        println!("0x{:x} JSR {:x}", addr, sub_addr);
        abus.push_stack(self.pc + 2, self.s);
        self.s -= 2;
        self.pc = sub_addr;
    }

    fn op_lda(&mut self, addr: u32, abus: &ABus) {
        if (self.p & (PFlag::M as u8)) == 0 {
            let result = abus.read16_le(addr + 1);
            println!("0x{:x} LDA {:x}", addr, result);
            self.a = result;
            // Set/reset zero-flag
            if result == 0 {
                self.p |= PFlag::Z as u8;
            } else {
                self.p &= !(PFlag::Z as u8);
            }
            // Set/reset negative-flag
            if result & 0x8000 > 0 {
                self.p |= PFlag::N as u8;
            } else {
                self.p &= !(PFlag::N as u8);
            }
            self.pc += 3;
        } else {
            let result = abus.read8(addr + 1) as u16;
            println!("0x{:x} LDA {:x}", addr, result);
            self.a = (self.a & 0xFF00) + result;
            // Set/reset zero-flag
            if result == 0 {
                self.p |= PFlag::Z as u8;
            } else {
                self.p &= !(PFlag::Z as u8);
            }
            // Set/reset negative-flag
            // TODO: Should this be result & 0x80 or self.a & 0x8000?
            if result & 0x80 > 0 {
                self.p |= PFlag::N as u8;
            } else {
                self.p &= !(PFlag::N as u8);
            }
            self.pc += 2;
        }
    }

    fn op_ldx(&mut self, addr: u32, abus: &ABus) {
        if (self.p & (PFlag::X as u8)) == 0 {
            let index = abus.read16_le(addr + 1);
            println!("0x{:x} LDX {:x}", addr, index);
            self.x = index;
            self.pc += 3;
        } else {
            let index = abus.read8(addr + 1) as u16;
            println!("0x{:x} LDX {:x}", addr, index);
            self.x = index;
            self.pc += 2;
        }
    }

    fn op_rep(&mut self, addr: u32, abus: &ABus) {
        let bits = abus.read8(addr + 1);
        println!("0x{:x} REP {:x}", addr, bits);
        self.p &= !bits;
        // Emulation forces M and X to 1
        if self.e {
            self.p |= PFlag::M as u8 | PFlag::X as u8;
        }
        // X = 1 forces XH and YH to 0x00
        if self.p & PFlag::X as u8 > 0 {
            self.x &= 0x00FF;
            self.y &= 0x00FF;
        }
        self.pc += 2;
    }

    fn op_sei(&mut self) {
        println!("0x{:x} SEI", ((self.pb as u32) << 16) + self.pc as u32);
        self.p |= PFlag::I as u8;
        self.pc += 1;
    }

    fn op_sep(&mut self, addr: u32, abus: &ABus) {
        let bits = abus.read8(addr + 1);
        println!("0x{:x} SEP {:x}", addr, bits);
        self.p |= bits;
        self.pc += 2;
    }

    fn op_sta(&mut self, addr: u32, abus: &mut ABus) {
        let read_addr = abus.read16_le(addr + 1);
        println!("0x{:x} STA {:x}", addr, read_addr);
        if (self.p & (PFlag::M as u8)) == 0 {
            abus.write16_le(self.a, ((self.pb as u32) << 16) + read_addr as u32);
        } else {
            abus.write8(self.a as u8, ((self.pb as u32) << 16) + read_addr as u32);
        }
        self.pc += 3;
    }

    fn op_txs(&mut self) {
        println!("0x{:x} TXS", ((self.pb as u32) << 16) + self.pc as u32);
        self.s = self.x;
        self.pc += 1;
    }


    fn op_xce(&mut self) {
        println!("0x{:x} XCE", ((self.pb as u32) << 16) + self.pc as u32);
        self.e = self.p & (PFlag::C as u8) == 1;
        // Emulation forces M and X -flags to 1
        if self.e {
            self.p |= PFlag::M  as u8 | PFlag::X as u8;
            // X = 1 forces XH and YH to 0x00
            self.x &= 0x00FF;
            self.y &= 0x00FF;
        }
        self.pc += 1;
    }

}
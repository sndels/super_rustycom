use abus::ABus;

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
            pc: abus.read16_le(0x00FF00 + RESET8 as u32), // TODO: This only applies to LoROM
            s:  0x01FF,
            p:  P_M | P_X | P_I,
            d : 0x00,
            pb: 0x0,
            db: 0x0,
            e:  true,
        }
    }

    pub fn run(&mut self, abus: &mut ABus) {
        loop {
            let addr = self.get_pb_pc();
            let opcode = abus.read8(addr);
            self.execute(opcode, addr, abus);
        }
    }

    // TODO: Page wrapping in emulation mode?
    // TODO: Wrap flag checks and sets?
    fn execute(&mut self, opcode: u8, addr: u32, abus: &mut ABus) {
        match opcode {
            0x18 => self.op_clc(),
            0x20 => self.op_jsr(addr, abus),
            0x78 => self.op_sei(),
            0x8D => self.op_sta(addr, abus),
            0x9A => self.op_txs(),
            0x9C => self.op_stz(addr, abus),
            0xA2 => self.op_ldx(addr, abus),
            0xA9 => self.op_lda(addr, abus),
            0xC2 => self.op_rep(addr, abus),
            0xE2 => self.op_sep(addr, abus),
            0xFB => self.op_xce(),
            _ => panic!("Unknown opcode {:X}!", opcode),
        }
    }

    fn get_pb_pc(&self) -> u32 { ((self.pb as u32) << 16) + self.pc as u32 }
    fn get_pb_addr(&self, addr: u16) -> u32 { ((self.pb as u32) << 16) + addr as u32 }

    // Instructions

    fn op_clc(&mut self) {
        println!("0x{:x} CLC", self.get_pb_pc());
        self.reset_p_c();
        self.pc += 1;
    }

    fn op_jsr(&mut self, addr: u32, abus: &mut ABus) {
        let sub_addr = abus.read16_le(addr + 1);
        println!("0x{:x} JSR {:x}", addr, sub_addr);
        abus.push_stack(self.pc + 2, self.s);
        self.s -= 2;
        self.pc = sub_addr;
    }

    fn op_lda(&mut self, addr: u32, abus: &mut ABus) {
        if self.get_p_m() {
            let result = abus.read8(addr + 1) as u16;
            println!("0x{:x} LDA {:x}", addr, result);
            self.a = (self.a & 0xFF00) + result;
            // Set/reset zero-flag
            if result == 0 {
                self.set_p_z();
            } else {
                self.reset_p_z();
            }
            // Set/reset negative-flag
            // TODO: Should this be result & 0x80 or self.a & 0x8000?
            if result & 0x80 > 0 {
                self.set_p_n();
            } else {
                self.reset_p_n();
            }
            self.pc += 2;
        } else {
            let result = abus.read16_le(addr + 1);
            println!("0x{:x} LDA {:x}", addr, result);
            self.a = result;
            // Set/reset zero-flag
            if result == 0 {
                self.set_p_z();
            } else {
                self.reset_p_z();
            }
            // Set/reset negative-flag
            if result & 0x8000 > 0 {
                self.set_p_n();
            } else {
                self.reset_p_n();
            }
            self.pc += 3;
        }
    }

    fn op_ldx(&mut self, addr: u32, abus: &mut ABus) {
        if self.get_p_x() {
            let result = abus.read8(addr + 1) as u16;
            println!("0x{:x} LDX {:x}", addr, result);
            self.x = result;
            // Set/reset zero-flag
            if result == 0 {
                self.set_p_z();
            } else {
                self.reset_p_z();
            }
            // Set/reset negative-flag
            if result & 0x80 > 0 {
                self.set_p_n();
            } else {
                self.reset_p_n();
            }
            self.pc += 2;
        } else {
            let result = abus.read16_le(addr + 1);
            println!("0x{:x} LDX {:x}", addr, result);
            self.x = result;
            // Set/reset zero-flag
            if result == 0 {
                self.set_p_z();
            } else {
                self.reset_p_z();
            }
            // Set/reset negative-flag
            if result & 0x8000 > 0 {
                self.set_p_n();
            } else {
                self.reset_p_n();
            }
            self.pc += 3;
        }
    }

    fn op_rep(&mut self, addr: u32, abus: &mut ABus) {
        let bits = abus.read8(addr + 1);
        println!("0x{:x} REP {:x}", addr, bits);
        self.p &= !bits;
        // Emulation forces M and X to 1
        if self.e {
            self.set_p_m();
            self.set_p_x();
        }
        // X = 1 forces XH and YH to 0x00
        if self.get_p_x() {
            self.x &= 0x00FF;
            self.y &= 0x00FF;
        }
        self.pc += 2;
    }

    fn op_sei(&mut self) {
        println!("0x{:x} SEI", self.get_pb_pc());
        self.set_p_i();
        self.pc += 1;
    }

    fn op_sep(&mut self, addr: u32, abus: &mut ABus) {
        let bits = abus.read8(addr + 1);
        println!("0x{:x} SEP {:x}", addr, bits);
        self.p |= bits;
        self.pc += 2;
    }

    fn op_sta(&mut self, addr: u32, abus: &mut ABus) {
        let read_addr = abus.read16_le(addr + 1);
        println!("0x{:x} STA {:x}", addr, read_addr);
        if !self.get_p_m() {
            abus.write16_le(self.a, self.get_pb_addr(read_addr));
        } else {
            abus.write8(self.a as u8, self.get_pb_addr(read_addr));
        }
        self.pc += 3;
    }

    fn op_stz(&mut self, addr: u32, abus: &mut ABus) {
        let read_addr = abus.read16_le(addr + 1);
        println!("0x{:x} STZ {:x}", addr, read_addr);
        if !self.get_p_m() {
            abus.write16_le(0, self.get_pb_addr(read_addr));
        } else {
            abus.write8(0, self.get_pb_addr(read_addr));
        }
        self.pc += 3;
    }

    fn op_txs(&mut self) {
        println!("0x{:x} TXS", self.get_pb_pc());
        if self.e {
            let result = self.x;
            self.s = result + 0x0100;
            // Set/reset zero-flag
            if result == 0 {
                self.set_p_z();
            } else {
                self.reset_p_z();
            }
            // Set/reset negative-flag
            if result & 0x80 > 0 {
                self.set_p_n();
            } else {
                self.reset_p_n();
            }
        } else {
            let result = self.x;
            self.s = result;
            // Set/reset zero-flag
            if result == 0 {
                self.set_p_z();
            } else {
                self.reset_p_z();
            }
            // Set/reset negative-flag
            if result & 0x8000 > 0 {
                self.set_p_n();
            } else {
                self.reset_p_n();
            }
        }
        self.pc += 1;
    }

    fn op_xce(&mut self) {
        println!("0x{:x} XCE", self.get_pb_pc());
        self.e = self.get_p_c();
        // Emulation forces M and X -flags to 1
        if self.e {
            self.set_p_m();
            self.set_p_x();
            // X = 1 forces XH and YH to 0x00
            self.x &= 0x00FF;
            self.y &= 0x00FF;
        }
        self.pc += 1;
    }

    // Flag operations
    fn get_p_c(&self) -> bool { self.p & P_C > 0 }
    fn get_p_z(&self) -> bool { self.p & P_Z > 0 }
    fn get_p_i(&self) -> bool { self.p & P_I > 0 }
    fn get_p_d(&self) -> bool { self.p & P_D > 0 }
    fn get_p_x(&self) -> bool { self.p & P_X > 0 }
    fn get_p_m(&self) -> bool { self.p & P_M > 0 }
    fn get_p_v(&self) -> bool { self.p & P_V > 0 }
    fn get_p_n(&self) -> bool { self.p & P_N > 0 }

    fn set_p_c(&mut self) { self.p |= P_C }
    fn set_p_z(&mut self) { self.p |= P_Z }
    fn set_p_i(&mut self) { self.p |= P_I }
    fn set_p_d(&mut self) { self.p |= P_D }
    fn set_p_x(&mut self) { self.p |= P_X }
    fn set_p_m(&mut self) { self.p |= P_M }
    fn set_p_v(&mut self) { self.p |= P_V }
    fn set_p_n(&mut self) { self.p |= P_N }

    fn reset_p_c(&mut self) { self.p &= !(P_C) }
    fn reset_p_z(&mut self) { self.p &= !(P_Z) }
    fn reset_p_i(&mut self) { self.p &= !(P_I) }
    fn reset_p_d(&mut self) { self.p &= !(P_D) }
    fn reset_p_x(&mut self) { self.p &= !(P_X) }
    fn reset_p_m(&mut self) { self.p &= !(P_M) }
    fn reset_p_v(&mut self) { self.p &= !(P_V) }
    fn reset_p_n(&mut self) { self.p &= !(P_N) }
}

// Interrupts
const COP16: u8   = 0xE4;
const BRK16: u8   = 0xE6;
const ABORT16: u8 = 0xE8;
const NMI16: u8   = 0xEA;
const IRQ16: u8   = 0xEE;
const COP8: u8    = 0xF4;
const ABORT8: u8  = 0xF8;
const NMI8: u8    = 0xFA;
const RESET8: u8  = 0xFC;
const IRQBRK8: u8 = 0xFE;

// P flags
const P_C: u8 = 0b0000_0001;
const P_Z: u8 = 0b0000_0010;
const P_I: u8 = 0b0000_0100;
const P_D: u8 = 0b0000_1000;
const P_X: u8 = 0b0001_0000;
const P_M: u8 = 0b0010_0000;
const P_V: u8 = 0b0100_0000;
const P_N: u8 = 0b1000_0000;

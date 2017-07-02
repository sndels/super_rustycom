use abus::ABus;
use op;

#[derive(Debug)]
pub struct Cpu {
    a:  u16,  // Accumulator
    x:  u16,  // Index register
    y:  u16,  // Index register
    pc: u16,  // Program counter
    s:  u16,  // Stack pointer
    p:  u8,   // Processor status register
    d:  u16,  // Zeropage offset
    pb: u8,   // Program counter bank
    db: u8,   // Data bank
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

    pub fn step(&mut self, abus: &mut ABus) {
        let addr = self.get_pb_pc();
        let opcode = abus.read8(addr);
        self.execute(opcode, addr, abus);
    }

    // TODO: Page wrapping in emulation mode?
    // TODO: Wrap flag checks and sets?
    fn execute(&mut self, opcode: u8, addr: u32, abus: &mut ABus) {
        match opcode {
            op::CLC => {
                self.reset_p_c();
                self.pc = self.pc.wrapping_add(1);
            }
            op::JSR => {
                let sub_addr = abus.fetch_operand_16(addr);
                abus.push_stack(self.pc.wrapping_add(2), self.s);
                self.s = self.s.wrapping_sub(2);
                self.pc = sub_addr;
            }
            op::SEI => {
                self.set_p_i();
                self.pc = self.pc.wrapping_add(1);
            }
            op::STA => {
                let read_addr = abus.fetch_operand_16(addr);
                if !self.get_p_m() {
                    abus.write16_le(self.a, self.get_pb_addr(read_addr));
                } else {
                    abus.write8(self.a as u8, self.get_pb_addr(read_addr));
                }
                self.pc = self.pc.wrapping_add(3);
            }
            op::TXS => {
                if self.e {
                    let result = self.x;
                    self.s = result + 0x0100;
                    self.update_p_z(result);
                    self.update_p_n_8(result as u8);
                } else {
                    let result = self.x;
                    self.s = result;
                    self.update_p_z(result);
                    self.update_p_n_16(result);
                }
                self.pc = self.pc.wrapping_add(1);
            }
            op::STZ => {
                let read_addr = abus.fetch_operand_16(addr);
                if !self.get_p_m() {
                    abus.write16_le(0, self.get_pb_addr(read_addr));
                } else {
                    abus.write8(0, self.get_pb_addr(read_addr));
                }
                self.pc = self.pc.wrapping_add(3);
            }
            op::LDX => {
                if self.get_p_x() {
                    let result = abus.fetch_operand_8(addr) as u16;
                    self.x = result;
                    self.update_p_z(result);
                    self.update_p_n_8(result as u8);
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let result = abus.fetch_operand_16(addr);
                    self.x = result;
                    self.update_p_z(result);
                    self.update_p_n_16(result);
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::LDA => {
                if self.get_p_m() {
                    let result = abus.fetch_operand_8(addr) as u16;
                    self.a = (self.a & 0xFF00) + result;
                    self.update_p_z(result);
                    self.update_p_n_8(result as u8);
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let result = abus.fetch_operand_16(addr);
                    self.a = result;
                    self.update_p_z(result);
                    self.update_p_n_16(result);
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::TAX => {
                let result = self.a;
                if self.get_p_x() {
                    self.x = result & 0x00FF;
                    self.update_p_n_8(result as u8);
                } else {
                    self.x = result;
                    self.update_p_n_16(result);
                }
                self.update_p_z(result);
                self.pc = self.pc.wrapping_add(1);
            }
            op::REP => {
                let bits = abus.fetch_operand_8(addr);
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
                self.pc = self.pc.wrapping_add(2);
            }
            op::BNE => {
                let offset = abus.fetch_operand_8(addr) as i8;
                if !self.get_p_z() {
                    self.pc = self.pc.wrapping_add(2);// TODO: Offset from opcode or end of operand?
                    if offset < 0 {// TODO: Check negative offset, wrapping for u8 or u16?
                        self.pc = self.pc.wrapping_sub(offset.abs() as u16);
                    } else {
                        self.pc += self.pc.wrapping_add(offset as u16);
                    }
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            op::SEP => {
                let bits = abus.fetch_operand_8(addr);
                self.p |= bits;
                self.pc = self.pc.wrapping_add(2);
            }
            op::INX => {
                if self.get_p_x() {
                    self.x = ((self.x as u8).wrapping_add(1)) as u16;
                    let result = self.x as u8;
                    self.update_p_n_8(result);
                } else {
                    self.x = self.x.wrapping_add(1);
                    let result = self.x;
                    self.update_p_n_16(result);
                }
                let result = self.x;
                self.update_p_z(result);
                self.pc = self.pc.wrapping_add(1);
            }
            op::XCE => {
                self.e = self.get_p_c();
                // Emulation forces M and X -flags to 1
                if self.e {
                    self.set_p_m();
                    self.set_p_x();
                    // X = 1 forces XH and YH to 0x00
                    self.x &= 0x00FF;
                    self.y &= 0x00FF;
                }
                self.pc = self.pc.wrapping_add(1);
            }
            _ => panic!("Unknown opcode {:X}!", opcode),
        }
    }

    fn get_pb_start(&self) -> u32 { (self.pb as u32) << 16 }
    pub fn get_pb_pc(&self) -> u32 { ((self.pb as u32) << 16) + self.pc as u32 }
    fn get_pb_addr(&self, addr: u16) -> u32 { ((self.pb as u32) << 16) + addr as u32 }

    pub fn print(&self) {
        println!("A:  {:#01$X}", self.a, 6);
        println!("X:  {:#01$X}", self.x, 6);
        println!("Y:  {:#01$X}", self.y, 6);
        println!("P:  {:01$b}", self.p, 8);
        println!("PB: {:#01$X}", self.pb, 4);
        println!("PC: {:#01$X}", self.pc, 6);
        println!("DB: {:#01$X}", self.db, 4);
        println!("S:  {:#01$X}", self.s, 6);
        println!("D:  {:#01$X}", self.d, 6);
        println!("E:  {}", self.e);
    }

    // Flag operations
    pub fn get_p_c(&self) -> bool { self.p & P_C > 0 }
    pub fn get_p_z(&self) -> bool { self.p & P_Z > 0 }
    pub fn get_p_i(&self) -> bool { self.p & P_I > 0 }
    pub fn get_p_d(&self) -> bool { self.p & P_D > 0 }
    pub fn get_p_x(&self) -> bool { self.p & P_X > 0 }
    pub fn get_p_m(&self) -> bool { self.p & P_M > 0 }
    pub fn get_p_v(&self) -> bool { self.p & P_V > 0 }
    pub fn get_p_n(&self) -> bool { self.p & P_N > 0 }

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

    fn update_p_z(&mut self, result: u16) {
        if result == 0 {
            self.set_p_z();
        } else {
            self.reset_p_z();
        }
    }
    fn update_p_n_8(&mut self, result: u8) {
        if result & 0x80 > 0 {
            self.set_p_n();
        } else {
            self.reset_p_n();
        }
    }

    fn update_p_n_16(&mut self, result: u16) {
        if result & 0x8000 > 0 {
            self.set_p_n();
        } else {
            self.reset_p_n();
        }
    }
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

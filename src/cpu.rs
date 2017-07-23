use abus;
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
            pc: abus.cpu_read_16(0x00FF00 + RESET8 as u32), // TODO: This only applies to LoROM
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
        let opcode = abus.cpu_read_8(addr);
        self.execute(opcode, addr, abus);
    }

    // TODO: Page wrapping in emulation mode?
    fn execute(&mut self, opcode: u8, addr: u32, abus: &mut ABus) {
        match opcode {
            op::CLC => {
                self.reset_p_c();
                self.pc = self.pc.wrapping_add(1);
            }
            op::JSR_20 => {
                let jump_addr = addr_8_16(self.pb, abus.fetch_operand_16(addr));
                let ret_addr = self.pc.wrapping_add(2);
                self.push_16(ret_addr, abus);
                self.pc = jump_addr as u16;
            }
            op::RTS => {
                self.pc = self.pull_16(abus).wrapping_add(1);
            }
            op::SEI => {
                self.set_p_i();
                self.pc = self.pc.wrapping_add(1);
            }
            op::STA_8D => {
                let data_addr = self.abs(addr, abus);
                if !self.get_p_m() {
                    abus.write_16(self.a, data_addr);
                } else {
                    abus.write_8(self.a as u8, data_addr);
                }
                self.pc = self.pc.wrapping_add(3);
            }
            op::STX_8E => {
                let data_addr = self.abs(addr, abus);
                if !self.get_p_x() {
                    abus.write_16(self.x, data_addr);
                } else {
                    abus.write_8(self.x as u8, data_addr);
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
            op::STZ_9C => {
                let data_addr = self.abs(addr, abus);
                if !self.get_p_m() {
                    abus.write_16(0, data_addr);
                } else {
                    abus.write_8(0, data_addr);
                }
                self.pc = self.pc.wrapping_add(3);
            }
            op::LDX_A2 => {
                if self.get_p_x() {
                    let data = self.imm_8(addr, abus) as u16;
                    self.x = data;
                    self.update_p_z(data);
                    self.update_p_n_8(data as u8);
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data = self.imm_16(addr,abus);
                    self.x = data;
                    self.update_p_z(data);
                    self.update_p_n_16(data);
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::LDA_A9 => {
                if self.get_p_m() {
                    let data = self.imm_8(addr, abus) as u16;
                    self.a = (self.a & 0xFF00) + data;
                    self.update_p_z(data);
                    self.update_p_n_8(data as u8);
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data = self.imm_16(addr, abus);
                    self.a = data;
                    self.update_p_z(data);
                    self.update_p_n_16(data);
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
            op::LDX_AE => {
                let data_addr = self.abs(addr, abus);
                if self.get_p_x() {
                    let data = abus.cpu_read_8(data_addr);
                    self.x = data as u16;
                    self.update_p_z(data as u16);
                    self.update_p_n_8(data);
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data = abus.bank_wrapping_cpu_read_16(data_addr);
                    self.x = data;
                    self.update_p_z(data);
                    self.update_p_n_16(data);
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::REP => {
                let bits = self.imm_8(addr, abus);
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
                if !self.get_p_z() {
                    self.pc = self.rel_8(addr, abus);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            op::CPX_E0 => {
                if self.get_p_x() {
                    let data = self.imm_8(addr, abus);
                    let result = (self.x as u8).wrapping_sub(data);// TODO: Matches binary subtraction?
                    self.update_p_n_8(result);
                    self.update_p_z(result as u16);
                    if (self.x as u8) < data {
                        self.reset_p_c();
                    } else {
                        self.set_p_c();
                    }
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data = self.imm_16(addr, abus);
                    let result = self.x.wrapping_sub(data);// TODO: Matches binary subtraction?
                    self.update_p_n_16(result);
                    self.update_p_z(result);
                    if self.x < data {
                        self.reset_p_c();
                    } else {
                        self.set_p_c();
                    }
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::SEP => {
                let bits = self.imm_8(addr, abus);
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
                let tmp = self.e;
                self.e = self.get_p_c();
                if tmp {
                    self.set_p_c();
                } else {
                    self.reset_p_c();
                }
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
            _ => panic!("Unknown opcode ${0:02X} at ${1:02X}:{2:04X}", opcode, addr >> 16, addr & 0xFFF)
        }
    }

    fn get_pb_start(&self) -> u32 { (self.pb as u32) << 16 }
    pub fn get_pb_pc(&self) -> u32 { ((self.pb as u32) << 16) + self.pc as u32 }
    fn get_pb_addr(&self, addr: u16) -> u32 { ((self.pb as u32) << 16) + addr as u32 }

    // Addressing modes
    // TODO: DRY, mismatch funcs and macros?
    fn abs(&self, addr: u32, abus: &mut ABus) -> u32 {// JMP, JSR use PB instead of DB
        addr_8_16(self.db, abus.fetch_operand_16(addr))
    }

    fn abs_x(&self, addr: u32, abus: &mut ABus) -> u32 {
        let operand = abus.fetch_operand_16(addr);
        (addr_8_16(self.db, operand) + self.x as u32) & 0x00FFFFFF
    }

    fn abs_y(&self, addr: u32, abus: &mut ABus) -> u32 {
        let operand = abus.fetch_operand_16(addr);
        (addr_8_16(self.db, operand) + self.y as u32) & 0x00FFFFFF
    }

    fn abs_ptr_16(&self, addr: u32, abus: &mut ABus) -> u32 {
        let pointer = abus.fetch_operand_16(addr) as u32;
        addr_8_16(self.pb, abus.bank_wrapping_cpu_read_16(pointer))
    }

    fn abs_ptr_24(&self, addr: u32, abus: &mut ABus) -> u32 {
        let pointer = abus.fetch_operand_24(addr) as u32;
        abus.bank_wrapping_cpu_read_24(pointer)
    }

    fn abs_ptr_x(&self, addr: u32, abus: &mut ABus) -> u32 {
        let pointer = addr_8_16(self.pb, abus.fetch_operand_16(addr).wrapping_add(self.x));
        addr_8_16(self.pb, abus.bank_wrapping_cpu_read_16(pointer))
    }

    fn dir(&self, addr: u32, abus: &mut ABus) -> u32 {
        self.d.wrapping_add(abus.fetch_operand_8(addr) as u16) as u32
    }

    fn dir_x(&self, addr: u32, abus: &mut ABus) -> u32 {
        if self.e && (self.d & 0xFF) == 0 {
            (self.d | (abus.fetch_operand_8(addr).wrapping_add(self.x as u8) as u16)) as u32
        } else {
            self.d.wrapping_add(abus.fetch_operand_8(addr) as u16).wrapping_add(self.x) as u32
        }
    }

    fn dir_y(&self, addr: u32, abus: &mut ABus) -> u32 {
        if self.e && (self.d & 0xFF) == 0 {
            (self.d | (abus.fetch_operand_8(addr).wrapping_add(self.y as u8) as u16)) as u32
        } else {
            self.d.wrapping_add(abus.fetch_operand_8(addr) as u16).wrapping_add(self.y) as u32
        }
    }

    fn dir_ptr_16(&self, addr: u32, abus: &mut ABus) -> u32 {
        let pointer = self.dir(addr, abus);
        if self.e && (self.d & 0xFF) == 0 {
            let ll = abus.cpu_read_8(pointer);
            let hh = abus.cpu_read_8((pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32);
            addr_8_8_8(self.db, hh, ll)
        } else {
            addr_8_16(self.db, abus.bank_wrapping_cpu_read_16(pointer))
        }
    }

    fn dir_ptr_24(&self, addr: u32, abus: &mut ABus) -> u32 {
        let pointer = self.dir(addr, abus);
        abus.bank_wrapping_cpu_read_24(pointer)
    }

    fn dir_ptr_16_x(&self, addr: u32, abus: &mut ABus) -> u32 {
        let pointer = self.dir_x(addr, abus);
        if self.e && (self.d & 0xFF) == 0 {
            let ll = abus.cpu_read_8(pointer);
            let hh = abus.cpu_read_8((pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32);
            addr_8_8_8(self.db, hh, ll)
        } else {
            addr_8_16(self.db, abus.bank_wrapping_cpu_read_16(pointer))
        }
    }

    fn dir_ptr_16_y(&self, addr: u32, abus: &mut ABus) -> u32 {
        let pointer = self.dir_ptr_16(addr, abus);
        if self.e && (self.d & 0xFF) == 0 {
            let ll = abus.cpu_read_8(pointer);
            let hh = abus.cpu_read_8((pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32);
            (addr_8_8_8(self.db, hh, ll) + self.y as u32) & 0x00FFFFFF
        } else {
            (addr_8_16(self.db, abus.bank_wrapping_cpu_read_16(pointer)) + self.y as u32) & 0x00FFFFFF
        }
    }

    fn dir_ptr_24_y(&self, addr: u32, abus: &mut ABus) -> u32 {
        let pointer = self.dir_ptr_24(addr, abus);
        (abus.bank_wrapping_cpu_read_24(pointer) + self.y as u32) & 0x00FFFFFF
    }

    fn imm_8(&self, addr: u32, abus: &mut ABus) -> u8 {
        abus.fetch_operand_8(addr)
    }

    fn imm_16(&self, addr: u32, abus: &mut ABus) -> u16 {
        abus.fetch_operand_16(addr)
    }

    fn long(&self, addr: u32, abus: &mut ABus) -> u32 {
        abus.fetch_operand_24(addr)
    }

    fn long_x(&self, addr: u32, abus: &mut ABus) -> u32 {
        (abus.fetch_operand_24(addr) + self.x as u32) & 0x00FFFFFF
    }

    fn rel_8(&self, addr: u32, abus: &mut ABus) -> u16 {
        let offset = abus.fetch_operand_8(addr);
        if offset < 0x80 {
            self.pc.wrapping_add(2 + (offset as u16))
        } else {
            self.pc.wrapping_sub(254).wrapping_add(offset as u16)
        }
    }

    fn rel_16(&self, addr: u32, abus: &mut ABus) -> u16 {
        let offset = abus.fetch_operand_16(addr);
        self.pc.wrapping_add(3 + offset)
    }

    fn stack(&self, addr: u32, abus: &mut ABus) -> u32 {
        self.s.wrapping_add(abus.fetch_operand_8(addr) as u16) as u32
    }

    fn stack_ptr_y(&self, addr: u32, abus: &mut ABus) -> u32 {
        let pointer = self.s.wrapping_add(abus.fetch_operand_8(addr) as u16) as u32;
        (addr_8_16(self.db, abus.bank_wrapping_cpu_read_16(pointer)) + self.y as u32) & 0x00FFFFFF
    }

    // Stack operations
    fn push_8(&mut self, value: u8, abus: &mut ABus) {
        abus.write_8(value, self.s as u32);
        self.decrement_s(1);
    }

    fn push_16(&mut self, value: u16, abus: &mut ABus) {
        self.decrement_s(1);
        if self.e {
            abus.page_wrapping_write_16(value, self.s as u32);
        } else {
            abus.bank_wrapping_write_16(value, self.s as u32);
        }
        self.decrement_s(1);
    }

    fn push_24(&mut self, value: u32, abus: &mut ABus) {
        self.decrement_s(2);
        if self.e {
            abus.page_wrapping_write_24(value, self.s as u32);
        } else {
            abus.bank_wrapping_write_24(value, self.s as u32);
        }
        self.decrement_s(1);
    }

    fn pull_8(&mut self, abus: &mut ABus) -> u8{
        self.increment_s(1);
        abus.cpu_read_8(self.s as u32)
    }

    fn pull_16(&mut self, abus: &mut ABus) -> u16{
        self.increment_s(1);
        let value: u16;
        if self.e {
            value = abus.page_wrapping_cpu_read_16(self.s as u32);
        } else {
            value = abus.bank_wrapping_cpu_read_16(self.s as u32);
        }
        self.increment_s(1);
        value
    }

    fn pull_24(&mut self, abus: &mut ABus) -> u32 {
        self.increment_s(1);
        let value: u32;
        if self.e {
            value = abus.page_wrapping_cpu_read_24(self.s as u32);
        } else {
            value = abus.bank_wrapping_cpu_read_24(self.s as u32);
        }
        self.increment_s(2);
        value
    }

    fn decrement_s(&mut self, offset: u8) {
        if self.e {
            self.s = 0x0100 | (self.s as u8).wrapping_sub(offset) as u16;
        } else {
            self.s = self.s.wrapping_sub(offset as u16);
        }
    }

    fn increment_s(&mut self, offset: u8) {
        if self.e {
            self.s = 0x0100 | (self.s as u8).wrapping_add(offset) as u16;
        } else {
            self.s = self.s.wrapping_add(offset as u16);
        }
    }

    pub fn print_registers(&self) {
        println!("A:  ${:04X}", self.a);
        println!("X:  ${:04X}", self.x);
        println!("Y:  ${:04X}", self.y);
        println!("P:  {:08b}", self.p);
        println!("PB: ${:02X}", self.pb);
        println!("DB: ${:02X}", self.db);
        println!("PC: ${:04X}", self.pc);
        println!("S:  ${:04X}", self.s);
        println!("D:  ${:04X}", self.d);
        println!("E:  {}", self.e);
    }

    pub fn print_flags(&self) {
        if self.get_p_c() { println!("Carry"); }
        if self.get_p_z() { println!("Zero"); }
        if self.get_p_i() { println!("Interrupt"); }
        if self.get_p_d() { println!("Decimal"); }
        if self.get_p_x() { println!("Index"); }
        if self.get_p_m() { println!("Memory"); }
        if self.get_p_v() { println!("Overflow"); }
        if self.get_p_n() { println!("Negative"); }
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

#[inline(always)]
fn addr_8_16(bank: u8, byte: u16) -> u32 {
    ((bank as u32) << 16) | byte as u32
}

#[inline(always)]
fn addr_8_8_8(bank: u8, page: u8, byte: u8) -> u32 {
    ((bank as u32) << 16) | ((page as u32) << 8) | byte as u32
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

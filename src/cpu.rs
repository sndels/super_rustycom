use abus::ABus;
use op;

pub struct Cpu {
    a:  u16,         // Accumulator
    x:  u16,         // Index register
    y:  u16,         // Index register
    pc: u16,         // Program counter
    s:  u16,         // Stack pointer
    p:  StatusReg,   // Processor status register
    d:  u16,         // Zeropage offset
    pb: u8,          // Program counter bank
    db: u8,          // Data bank
    e:  bool,        // Emulation mode
}

impl Cpu {
    pub fn new(abus: &mut ABus) -> Cpu {
        Cpu {
            a:  0x00,
            x:  0x00,
            y:  0x00,
            pc: abus.page_wrapping_cpu_read_16(0x00FF00 + RESET8 as u32), // TODO: This only applies to LoROM
            s:  0x01FF,
            p:  StatusReg::new(),
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
                self.p.c = false;
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
                self.p.i = true;
                self.pc = self.pc.wrapping_add(1);
            }
            op::STA_8D => {
                let data_addr = self.abs(addr, abus).0;
                if !self.p.m {
                    abus.addr_wrapping_cpu_write_16(self.a, data_addr);
                } else {
                    abus.cpu_write_8(self.a as u8, data_addr);
                }
                self.pc = self.pc.wrapping_add(3);
            }
            op::STX_8E => {
                let data_addr = self.abs(addr, abus).0;
                if !self.p.x {
                    abus.addr_wrapping_cpu_write_16(self.x, data_addr);
                } else {
                    abus.cpu_write_8(self.x as u8, data_addr);
                }
                self.pc = self.pc.wrapping_add(3);
            }
            op::TXS => {
                if self.e {
                    let result = self.x;
                    self.s = result + 0x0100;
                    self.p.z = result == 0;
                    self.p.n = result > 0x7F;
                } else {
                    let result = self.x;
                    self.s = result;
                    self.p.z = result == 0;
                    self.p.n = result > 0x7FFF;
                }
                self.pc = self.pc.wrapping_add(1);
            }
            op::STZ_9C => {
                let data_addr = self.abs(addr, abus).0;
                if !self.p.m {
                    abus.addr_wrapping_cpu_write_16(0, data_addr);
                } else {
                    abus.cpu_write_8(0, data_addr);
                }
                self.pc = self.pc.wrapping_add(3);
            }
            op::LDX_A2 => {
                if self.p.x {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.cpu_read_8(data_addr.0) as u16;
                    self.x = data;
                    self.p.z = data == 0;
                    self.p.n = data > 0x7F;
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.bank_wrapping_cpu_read_16(data_addr.0);
                    self.x = data;
                    self.p.z = data == 0;
                    self.p.n = data > 0x7FFF;
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::LDA_A9 => {
                if self.p.m {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.cpu_read_8(data_addr.0) as u16;
                    self.a = (self.a & 0xFF00) + data;
                    self.p.z = data == 0;
                    self.p.n = data > 0x7F;
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.bank_wrapping_cpu_read_16(data_addr.0);
                    self.a = data;
                    self.p.z = data == 0;
                    self.p.n = data > 0x7FFF;
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::TAX => {
                let result = self.a;
                if self.p.x {
                    self.x = result & 0x00FF;
                    self.p.n = result > 0x7F;
                } else {
                    self.x = result;
                    self.p.n = result > 0x7FFF;
                }
                self.p.z = result == 0;
                self.pc = self.pc.wrapping_add(1);
            }
            op::LDX_AE => {
                let data_addr = self.abs(addr, abus).0;
                if self.p.x {
                    let data = abus.cpu_read_8(data_addr);
                    self.x = data as u16;
                    self.p.z = data == 0;
                    self.p.n = data > 0x7F;
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data = abus.bank_wrapping_cpu_read_16(data_addr);
                    self.x = data;
                    self.p.z = data == 0;
                    self.p.n = data > 0x7FFF;
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::REP => {
                let data_addr = self.imm(addr, abus);
                let mask = abus.cpu_read_8(data_addr.0);
                self.p.clear_flags(mask);
                // Emulation forces M and X to 1
                if self.e {
                    self.p.m = true;
                    self.p.x = true;
                }
                self.pc = self.pc.wrapping_add(2);
            }
            op::BNE => {
                if !self.p.z {
                    self.pc = self.rel_8(addr, abus).0 as u16;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            op::CPX_E0 => {
                if self.p.x {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.cpu_read_8(data_addr.0);
                    let result = (self.x as u8).wrapping_sub(data);// TODO: Matches binary subtraction?
                    self.p.n = result > 0x7F;
                    self.p.z = result == 0;
                    if (self.x as u8) < data {
                        self.p.c = false;
                    } else {
                        self.p.c = true;
                    }
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.bank_wrapping_cpu_read_16(data_addr.0);
                    let result = self.x.wrapping_sub(data);// TODO: Matches binary subtraction?
                    self.p.n = result > 0x7FFF;
                    self.p.z = result == 0;
                    if self.x < data {
                        self.p.c = false;
                    } else {
                        self.p.c = true;
                    }
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::SEP => {
                let data_addr = self.imm(addr, abus);
                let mask = abus.cpu_read_8(data_addr.0);
                self.p.set_flags(mask);
                if self.p.x {
                    self.x &= 0x00FF;
                    self.y &= 0x00FF;
                 }
                self.pc = self.pc.wrapping_add(2);
            }
            op::INX => {
                if self.p.x {
                    self.x = ((self.x as u8).wrapping_add(1)) as u16;
                    let result = self.x as u8;
                    self.p.n = result > 0x7F;
                } else {
                    self.x = self.x.wrapping_add(1);
                    let result = self.x;
                    self.p.n = result > 0x7FFF;
                }
                let result = self.x;
                self.p.z = result == 0;
                self.pc = self.pc.wrapping_add(1);
            }
            op::XCE => {
                let tmp = self.e;
                if self.p.c{
                    self.set_emumode();
                } else {
                    self.e = false
                }
                if tmp {
                    self.p.c = true;
                } else {
                    self.p.c = false;
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
    fn abs(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {// JMP, JSR use PB instead of DB
        (addr_8_16(self.db, abus.fetch_operand_16(addr)), WrappingMode::AddrSpace)
    }

    fn abs_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let operand = abus.fetch_operand_16(addr);
        ((addr_8_16(self.db, operand) + self.x as u32) & 0x00FFFFFF, WrappingMode::AddrSpace)
    }

    fn abs_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let operand = abus.fetch_operand_16(addr);
        ((addr_8_16(self.db, operand) + self.y as u32) & 0x00FFFFFF, WrappingMode::AddrSpace)
    }

    fn abs_ptr_16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = abus.fetch_operand_16(addr) as u32;
        (addr_8_16(self.pb, abus.bank_wrapping_cpu_read_16(pointer)), WrappingMode::Bank)
    }

    fn abs_ptr_24(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = abus.fetch_operand_24(addr) as u32;
        (abus.bank_wrapping_cpu_read_24(pointer), WrappingMode::Bank)
    }

    fn abs_ptr_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = addr_8_16(self.pb, abus.fetch_operand_16(addr).wrapping_add(self.x));
        (addr_8_16(self.pb, abus.bank_wrapping_cpu_read_16(pointer)), WrappingMode::Bank)
    }

    fn dir(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        // NOTE: Data of "old" instructions with e=1, DL=$00 actually "wrap" page but only access 8bit
        (self.d.wrapping_add(abus.fetch_operand_8(addr) as u16) as u32, WrappingMode::Bank)
    }

    fn dir_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        if self.e && (self.d & 0xFF) == 0 {
            ((self.d | (abus.fetch_operand_8(addr).wrapping_add(self.x as u8) as u16)) as u32,
             WrappingMode::Page)
        } else {
            (self.d.wrapping_add(abus.fetch_operand_8(addr) as u16).wrapping_add(self.x) as u32,
             WrappingMode::Bank)
        }
    }

    fn dir_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        if self.e && (self.d & 0xFF) == 0 {
            ((self.d | (abus.fetch_operand_8(addr).wrapping_add(self.y as u8) as u16)) as u32,
             WrappingMode::Page)
        } else {
            (self.d.wrapping_add(abus.fetch_operand_8(addr) as u16).wrapping_add(self.y) as u32,
             WrappingMode::Bank)
        }
    }

    fn dir_ptr_16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.dir(addr, abus).0;
        if self.e && (self.d & 0xFF) == 0 {
            let ll = abus.cpu_read_8(pointer);
            let hh = abus.cpu_read_8((pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32);
            (addr_8_8_8(self.db, hh, ll), WrappingMode::AddrSpace)
        } else {
            (addr_8_16(self.db, abus.bank_wrapping_cpu_read_16(pointer)), WrappingMode::AddrSpace)
        }
    }

    fn dir_ptr_24(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.dir(addr, abus).0;
        (abus.bank_wrapping_cpu_read_24(pointer), WrappingMode::AddrSpace)
    }

    fn dir_ptr_16_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.dir_x(addr, abus).0;
        if self.e && (self.d & 0xFF) == 0 {
            let ll = abus.cpu_read_8(pointer);
            let hh = abus.cpu_read_8((pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32);
            (addr_8_8_8(self.db, hh, ll), WrappingMode::AddrSpace)
        } else {
            (addr_8_16(self.db, abus.bank_wrapping_cpu_read_16(pointer)), WrappingMode::AddrSpace)
        }
    }

    fn dir_ptr_16_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        ((self.dir_ptr_16(addr, abus).0 + self.y as u32) & 0x00FFFFFF, WrappingMode::AddrSpace)
    }

    fn dir_ptr_24_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        ((self.dir_ptr_24(addr, abus).0 + self.y as u32) & 0x00FFFFFF, WrappingMode::AddrSpace)
    }

    fn imm(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (addr_8_16(self.pb, self.pc.wrapping_add(1)), WrappingMode::Bank)
    }

    fn long(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (abus.fetch_operand_24(addr), WrappingMode::AddrSpace)
    }

    fn long_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        ((abus.fetch_operand_24(addr) + self.x as u32) & 0x00FFFFFF, WrappingMode::AddrSpace)
    }

    fn rel_8(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let offset = abus.fetch_operand_8(addr);
        if offset < 0x80 {
            (addr_8_16(self.pb, self.pc.wrapping_add(2 + (offset as u16))), WrappingMode::Bank)
        } else {
            (addr_8_16(self.pb, self.pc.wrapping_sub(254).wrapping_add(offset as u16)),
             WrappingMode::Bank)
        }
    }

    fn rel_16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let offset = abus.fetch_operand_16(addr);
        (addr_8_16(self.pb, self.pc.wrapping_add(3).wrapping_add(offset)), WrappingMode::Bank)
    }

    fn stack(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (self.s.wrapping_add(abus.fetch_operand_8(addr) as u16) as u32, WrappingMode::Bank)
    }

    fn stack_ptr_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.s.wrapping_add(abus.fetch_operand_8(addr) as u16) as u32;
        ((addr_8_16(self.db, abus.bank_wrapping_cpu_read_16(pointer)) + self.y as u32) & 0x00FFFFFF,
         WrappingMode::AddrSpace)
    }

    // Stack operations
    fn push_8(&mut self, value: u8, abus: &mut ABus) {
        abus.cpu_write_8(value, self.s as u32);
        self.decrement_s(1);
    }

    fn push_16(&mut self, value: u16, abus: &mut ABus) {
        self.decrement_s(1);
        if self.e {
            abus.page_wrapping_cpu_write_16(value, self.s as u32);
        } else {
            abus.bank_wrapping_cpu_write_16(value, self.s as u32);
        }
        self.decrement_s(1);
    }

    fn push_24(&mut self, value: u32, abus: &mut ABus) {
        self.decrement_s(2);
        if self.e {
            abus.page_wrapping_cpu_write_24(value, self.s as u32);
        } else {
            abus.bank_wrapping_cpu_write_24(value, self.s as u32);
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
        println!("P:  {:08b}", self.p.get_value());
        println!("PB: ${:02X}", self.pb);
        println!("DB: ${:02X}", self.db);
        println!("PC: ${:04X}", self.pc);
        println!("S:  ${:04X}", self.s);
        println!("D:  ${:04X}", self.d);
        println!("E:  {}", self.e);
    }

    pub fn print_flags(&self) {
        if self.p.c { println!("Carry"); }
        if self.p.z { println!("Zero"); }
        if self.p.i { println!("Interrupt"); }
        if self.p.d { println!("Decimal"); }
        if self.p.x { println!("Index"); }
        if self.p.m { println!("Memory"); }
        if self.p.v { println!("Overflow"); }
        if self.p.n { println!("Negative"); }
    }

    pub fn get_p_c(&self) -> bool { self.p.c }
    pub fn get_p_z(&self) -> bool { self.p.z }
    pub fn get_p_i(&self) -> bool { self.p.i }
    pub fn get_p_d(&self) -> bool { self.p.d }
    pub fn get_p_x(&self) -> bool { self.p.x }
    pub fn get_p_m(&self) -> bool { self.p.m }
    pub fn get_p_v(&self) -> bool { self.p.v }
    pub fn get_p_n(&self) -> bool { self.p.n }

    fn set_emumode(&mut self) {
        self.e = true;
        self.p.m = true;
        self.p.x = true;
        self.x &= 0x00FF;
        self.y &= 0x00FF;
        self.s = 0x0100 | (self.s & 0x00FF);
    }

    // Instructions
    fn op_adc(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        let carry: u16 = if self.p.c { 1 } else { 0 };
        if self.p.m { // 8-bit accumulator
            let data = abus.cpu_read_8(data_addr.0);
            let result: u8;
            if self.p.d { // BCD arithmetic
                let decimal_acc = bcd_to_dec_8(self.a as u8);
                let decimal_data = bcd_to_dec_8(data);
                let decimal_result = decimal_acc + decimal_data + carry as u8;
                let bcd_result = dec_to_bcd_8(decimal_result % 100);
                if decimal_result > 99 {
                    self.p.c = true;
                } else {
                    self.p.c = false;
                }
                result = bcd_result;
            } else { // Binary arithmetic
                let acc_8 = self.a as u8;
                let result_16 = acc_8 as u16 + (data as u16) + carry;
                if result_16 > 0xFF {
                    self.p.c = true;
                } else {
                    self.p.c = false;
                }
                result = result_16 as u8;
            }
            self.p.n = result > 0x7F;
            self.p.z = result == 0;
            if ((self.a as u8) < 0x80 && data < 0x80 && result > 0x7F) ||
               ((self.a as u8) > 0x7F && data > 0x7F && result < 0x80) {
                self.p.v = true;
            } else {
                self.p.v = false;
            }
            self.a = (self.a & 0xFF00) | (result as u16);
        } else { // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Page      => data = abus.page_wrapping_cpu_read_16(data_addr.0),
                WrappingMode::Bank      => data = abus.bank_wrapping_cpu_read_16(data_addr.0),
                WrappingMode::AddrSpace => data = abus.addr_wrapping_cpu_read_16(data_addr.0),
            }
            let result: u16;
            if self.p.d { // BCD arithmetic
                let decimal_acc = bcd_to_dec_16(self.a);
                let decimal_data = bcd_to_dec_16(data);
                let decimal_result = decimal_acc + decimal_data + carry;
                let bcd_result = dec_to_bcd_16(decimal_result % 10000);
                if decimal_result > 9999 {
                    self.p.c = true;
                } else {
                    self.p.c = false;
                }
                result = bcd_result;
            } else { // Binary arithmetic
                let result_32 = (self.a as u32) + (data as u32) + carry as u32;
                if result_32 > 0xFFFF {
                    self.p.c = true;
                } else {
                    self.p.c = false;
                }
                result = result_32 as u16;
            }
            self.p.n = result > 0x7FFF;
            self.p.z = result == 0;
            if (self.a < 0x8000 && data < 0x8000 && result > 0x7FFF) ||
               (self.a > 0x7FFF && data > 0x7FFF && result < 0x8000) {
                self.p.v = true;
            } else {
                self.p.v = false;
            }
            self.a = result;
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

fn dec_to_bcd_8(value: u8) -> u8 {
    if value > 99 { panic!("{} too large for 8bit BCD", value); }
    ((value / 10) << 4) | (value % 10)
}

fn dec_to_bcd_16(value: u16) -> u16 {
    if value > 9999 { panic!("{} too large for 16bit BCD", value); }
    ((value / 1000) << 12) | (((value % 1000) / 100) << 8) |
    (((value % 100) / 10) << 4) | (value % 10)
}

fn bcd_to_dec_8(value: u8) -> u8 {
    let ll = value & 0x000F;
    let hh = (value >> 4) & 0x000F;
    if hh > 0x9 || ll > 0x9 { panic!("{:02X} is not a valid 8bit BCD", value); }
    hh * 10 + ll
}

fn bcd_to_dec_16(value: u16) -> u16 {
    let ll = value & 0x000F;
    let ml = (value >> 4) & 0x000F;
    let mh = (value >> 8) & 0x000F;
    let hh = (value >> 12) & 0x000F;
    if hh > 0x9 || mh > 0x9 || ml > 0x9 || ll > 0x9 {
        panic!("{:04X} is not a valid 16bit BCD", value);
    }
    hh * 1000 + mh * 100 + ml * 10 + ll
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

#[derive(PartialEq, Debug)]
enum WrappingMode {
    Page,
    Bank,
    AddrSpace
}

#[derive(Clone)]
struct StatusReg {
    n: bool,
    v: bool,
    m: bool,
    x: bool,
    d: bool,
    i: bool,
    z: bool,
    c: bool
}

impl StatusReg {
    pub fn new() -> StatusReg{
        StatusReg{
            n: false,
            v: false,
            m: true,
            x: true,
            d: false,
            i: true,
            z: false,
            c: false,
        }
    }

    pub fn set_flags(&mut self, mask: u8) {
        if mask & P_N > 0 { self.n = true; }
        if mask & P_V > 0 { self.v = true; }
        if mask & P_M > 0 { self.m = true; }
        if mask & P_X > 0 { self.x = true; }
        if mask & P_D > 0 { self.d = true; }
        if mask & P_I > 0 { self.i = true; }
        if mask & P_Z > 0 { self.z = true; }
        if mask & P_C > 0 { self.c = true; }
    }

    pub fn clear_flags(&mut self, mask: u8) {
        if mask & P_N > 0 { self.n = false; }
        if mask & P_V > 0 { self.v = false; }
        if mask & P_M > 0 { self.m = false; }
        if mask & P_X > 0 { self.x = false; }
        if mask & P_D > 0 { self.d = false; }
        if mask & P_I > 0 { self.i = false; }
        if mask & P_Z > 0 { self.z = false; }
        if mask & P_C > 0 { self.c = false; }
    }

    pub fn get_value(&self) -> u8 {
        let mut value: u8 = 0b0000_0000;
        if self.n { value |= P_N; }
        if self.v { value |= P_V; }
        if self.m { value |= P_M; }
        if self.x { value |= P_X; }
        if self.d { value |= P_D; }
        if self.i { value |= P_I; }
        if self.z { value |= P_Z; }
        if self.c { value |= P_C; }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Stack
    #[test]
    fn stack_push() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = Cpu::new(&mut abus);
        cpu.s = 0x01FF;
        cpu.set_emumode();
        // Regular emumode pushes
        cpu.push_8(0x12, &mut abus);
        assert_eq!(0x01FE, cpu.s);
        assert_eq!(0x12, abus.cpu_read_8(0x0001FF));
        cpu.push_16(0x2345, &mut abus);
        assert_eq!(0x01FC, cpu.s);
        assert_eq!(0x2345, abus.page_wrapping_cpu_read_16(0x0001FD));
        cpu.push_24(0x345678, &mut abus);
        assert_eq!(0x01F9, cpu.s);
        assert_eq!(0x345678, abus.page_wrapping_cpu_read_24(0x0001FA));
        abus.bank_wrapping_cpu_write_24(0x000000, 0x0001FA);
        abus.bank_wrapping_cpu_write_24(0x000000, 0x0001FD);
        // Wrapping emumode pushes
        cpu.s = 0x0100;
        cpu.push_8(0x12, &mut abus);
        cpu.push_8(0x34, &mut abus);
        assert_eq!(0x1234, abus.page_wrapping_cpu_read_16(0x0001FF));
        cpu.s = 0x0100;
        cpu.push_16(0x2345, &mut abus);
        assert_eq!(0x2345, abus.page_wrapping_cpu_read_16(0x0001FF));
        cpu.s = 0x0100;
        cpu.push_24(0x345678, &mut abus);
        assert_eq!(0x345678, abus.page_wrapping_cpu_read_24(0x0001FE));
        abus.bank_wrapping_cpu_write_24(0x000000, 0x0001FE);
        // Regular pushes
        cpu.s = 0x01FF;
        cpu.e = false;
        cpu.push_8(0x12, &mut abus);
        assert_eq!(0x01FE, cpu.s);
        assert_eq!(0x12, abus.cpu_read_8(0x0001FF));
        cpu.push_16(0x2345, &mut abus);
        assert_eq!(0x01FC, cpu.s);
        assert_eq!(0x2345, abus.bank_wrapping_cpu_read_16(0x0001FD));
        cpu.push_24(0x345678, &mut abus);
        assert_eq!(0x01F9, cpu.s);
        assert_eq!(0x345678, abus.bank_wrapping_cpu_read_24(0x0001FA));
        abus.bank_wrapping_cpu_write_24(0x000000, 0x0001FA);
        abus.bank_wrapping_cpu_write_24(0x000000, 0x0001FD);
        // Wrapping pushes
        cpu.s = 0x0000;
        cpu.push_8(0x12, &mut abus);
        cpu.push_8(0x34, &mut abus);
        assert_eq!(0xFFFE, cpu.s);
        assert_eq!(0x1234, abus.bank_wrapping_cpu_read_16(0x00FFFF));
        cpu.s = 0x0000;
        cpu.push_16(0x2345, &mut abus);
        assert_eq!(0xFFFE, cpu.s);
        assert_eq!(0x2345, abus.bank_wrapping_cpu_read_16(0x00FFFF));
        cpu.s = 0x0000;
        cpu.push_24(0x345678, &mut abus);
        assert_eq!(0xFFFD, cpu.s);
        assert_eq!(0x345678, abus.bank_wrapping_cpu_read_24(0x00FFFE));
        abus.bank_wrapping_cpu_write_24(0x000000, 0x0FFFE);
    }

    #[test]
    fn stack_pull() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = Cpu::new(&mut abus);
        cpu.s = 0x01FC;
        cpu.set_emumode() ;
        // Regular emumode pulls
        abus.page_wrapping_cpu_write_24(0x123456, 0x0001FD);
        assert_eq!(0x56, cpu.pull_8(&mut abus));
        assert_eq!(0x01FD, cpu.s);
        assert_eq!(0x1234, cpu.pull_16(&mut abus));
        assert_eq!(0x01FF, cpu.s);
        cpu.s = 0x01FC;
        assert_eq!(0x123456, cpu.pull_24(&mut abus));
        assert_eq!(0x01FF, cpu.s);
        abus.page_wrapping_cpu_write_24(0x000000, 0x0001FD);
        // Wrapping emumode pulls
        abus.page_wrapping_cpu_write_24(0x123456, 0x0001FF);
        cpu.s = 0x01FF;
        assert_eq!(0x34, cpu.pull_8(&mut abus));
        assert_eq!(0x0100, cpu.s);
        cpu.s = 0x01FE;
        assert_eq!(0x3456, cpu.pull_16(&mut abus));
        assert_eq!(0x0100, cpu.s);
        cpu.s = 0x01FE;
        assert_eq!(0x123456, cpu.pull_24(&mut abus));
        assert_eq!(0x0101, cpu.s);
        abus.page_wrapping_cpu_write_24(0x000000, 0x0001FF);
        // Regular pulls
        cpu.s = 0x01FC;
        cpu.e = false;
        abus.bank_wrapping_cpu_write_24(0x123456, 0x0001FD);
        assert_eq!(0x56, cpu.pull_8(&mut abus));
        assert_eq!(0x01FD, cpu.s);
        assert_eq!(0x1234, cpu.pull_16(&mut abus));
        assert_eq!(0x01FF, cpu.s);
        cpu.s = 0x01FC;
        assert_eq!(0x123456, cpu.pull_24(&mut abus));
        assert_eq!(0x01FF, cpu.s);
        abus.bank_wrapping_cpu_write_24(0x000000, 0x0001FD);
        // Wrapping pulls
        abus.bank_wrapping_cpu_write_24(0x123456, 0x00FFFF);
        cpu.s = 0xFFFF;
        assert_eq!(0x34, cpu.pull_8(&mut abus));
        assert_eq!(0x0000, cpu.s);
        cpu.s = 0xFFFE;
        assert_eq!(0x3456, cpu.pull_16(&mut abus));
        assert_eq!(0x0000, cpu.s);
        cpu.s = 0xFFFE;
        assert_eq!(0x123456, cpu.pull_24(&mut abus));
        assert_eq!(0x0001, cpu.s);
        abus.bank_wrapping_cpu_write_24(0x000000, 0x00FFFF);
    }

    // Addressing modes
    #[test]
    fn addr_abs() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = Cpu::new(&mut abus);
        cpu.pb = 0x1A;
        cpu.db = 0x1B;
        cpu.pc = 0x0010;
        cpu.x = 0x5678;
        cpu.y = 0xABCD;
        // Absolute
        abus.bank_wrapping_cpu_write_16(0x1234, 0x000011);
        assert_eq!((0x1B1234, WrappingMode::AddrSpace), cpu.abs(cpu.pc as u32, &mut abus));
        // Absolute,X and Absolute,Y
        assert_eq!((0x1B68AC, WrappingMode::AddrSpace), cpu.abs_x(cpu.pc as u32, &mut abus));
        assert_eq!((0x1BBE01, WrappingMode::AddrSpace), cpu.abs_y(cpu.pc as u32, &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x000011);
        // Wrapping
        cpu.db = 0xFF;
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x000011);
        assert_eq!((0x000245, WrappingMode::AddrSpace), cpu.abs_x(cpu.pc as u32, &mut abus));
        assert_eq!((0x00579A, WrappingMode::AddrSpace), cpu.abs_y(cpu.pc as u32, &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x000011);
        // (Absolute) and [Absolute]
        abus.bank_wrapping_cpu_write_16(0x01FF, 0x000011);
        abus.bank_wrapping_cpu_write_24(0xABCDEF, 0x0001FF);
        assert_eq!((0x1ACDEF, WrappingMode::Bank), cpu.abs_ptr_16(cpu.pc as u32, &mut abus));
        assert_eq!((0xABCDEF, WrappingMode::Bank), cpu.abs_ptr_24(cpu.pc as u32, &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x000011);
        abus.bank_wrapping_cpu_write_24(0x000000, 0x0001FF);
        // Wrapping
        abus.bank_wrapping_cpu_write_16(0xFFFF, 0x000011);
        abus.bank_wrapping_cpu_write_24(0xFEDCBA, 0x00FFFF);
        assert_eq!((0x1ADCBA, WrappingMode::Bank), cpu.abs_ptr_16(cpu.pc as u32, &mut abus));
        assert_eq!((0xFEDCBA, WrappingMode::Bank), cpu.abs_ptr_24(cpu.pc as u32, &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x000011);
        abus.bank_wrapping_cpu_write_24(0x000000, 0x00FFFF);
        // (Absolute,X)
        cpu.pb = 0x7E;
        abus.bank_wrapping_cpu_write_16(0x01FF, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x7E5877);
        assert_eq!((0x7EABCD, WrappingMode::Bank), cpu.abs_ptr_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0x0000, 0x7E5877);
        // Wrapping
        abus.bank_wrapping_cpu_write_16(0xA987, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0x1234, 0x7EFFFF);
        assert_eq!((0x7E1234, WrappingMode::Bank), cpu.abs_ptr_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0x0000, 0x7EFFFF);
        abus.bank_wrapping_cpu_write_16(0xA989, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0x5678, 0x7E0001);
        assert_eq!((0x7E5678, WrappingMode::Bank), cpu.abs_ptr_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0x0000, 0x7E0001);
    }

    #[test]
    fn addr_dir() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = Cpu::new(&mut abus);
        cpu.pb = 0x7E;
        cpu.db = 0x7F;
        cpu.pc = 0x0010;
        cpu.d = 0x1234;
        cpu.set_emumode() ;
        // Direct
        abus.cpu_write_8(0x12, 0x7E0011);
        assert_eq!((0x001246, WrappingMode::Bank), cpu.dir(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        // Wrapping
        cpu.d = 0xFFE0;
        abus.cpu_write_8(0xFF, 0x7E0011);
        assert_eq!((0x0000DF, WrappingMode::Bank), cpu.dir(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        // Direct,X and Direct,Y
        // Standard behavior
        cpu.d = 0x1234;
        cpu.x = 0x5678;
        cpu.y = 0xABCD;
        abus.cpu_write_8(0x12, 0x7E0011);
        assert_eq!((0x0068BE, WrappingMode::Bank), cpu.dir_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        assert_eq!((0x00BE13, WrappingMode::Bank), cpu.dir_y(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        // Wrapping
        cpu.d = 0xCDEF;
        assert_eq!((0x002479, WrappingMode::Bank), cpu.dir_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        assert_eq!((0x0079CE, WrappingMode::Bank), cpu.dir_y(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        // Emumode and DL is $00
        cpu.d = 0x1200;
        cpu.x = 0x0034;
        cpu.y = 0x00AB;
        abus.cpu_write_8(0x12, 0x7E0011);
        assert_eq!((0x001246, WrappingMode::Page), cpu.dir_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        assert_eq!((0x0012BD, WrappingMode::Page), cpu.dir_y(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        // Wrapping
        abus.cpu_write_8(0xDE, 0x7E0011);
        assert_eq!((0x001212, WrappingMode::Page), cpu.dir_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        assert_eq!((0x001289, WrappingMode::Page), cpu.dir_y(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        // (Direct)
        cpu.d = 0x1234;
        abus.cpu_write_8(0x56, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x00128A);
        assert_eq!((0x7FABCD, WrappingMode::AddrSpace), cpu.dir_ptr_16(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x00128A);
        // Wrapping
        cpu.d = 0xFFA9;
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x00FFFF);
        assert_eq!((0x7FABCD, WrappingMode::AddrSpace), cpu.dir_ptr_16(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x00FFFF);
        // [Direct]
        cpu.d = 0x1234;
        abus.cpu_write_8(0x56, 0x7E0011);
        abus.bank_wrapping_cpu_write_24(0xABCDEF, 0x00128A);
        assert_eq!((0xABCDEF, WrappingMode::AddrSpace), cpu.dir_ptr_24(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_24(0x000000, 0x00128A);
        // Wrapping
        cpu.d = 0xFFA9;
        abus.bank_wrapping_cpu_write_24(0xABCDEF, 0x00FFFF);
        assert_eq!((0xABCDEF, WrappingMode::AddrSpace), cpu.dir_ptr_24(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_24(0x000000, 0x00FFFF);
        // (Direct,X)
        // Standard behavior
        cpu.d = 0x0123;
        cpu.x = 0x1234;
        abus.cpu_write_8(0x12, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x001369);
        assert_eq!((0x7FABCD, WrappingMode::AddrSpace), cpu.dir_ptr_16_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x001369);
        // Wrapping
        cpu.d = 0xABCD;
        cpu.x = 0x5420;
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x00FFFF);
        assert_eq!((0x7FABCD, WrappingMode::AddrSpace), cpu.dir_ptr_16_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x00FFFF);
        // Emumode and DL is $00
        cpu.d = 0x1200;
        cpu.x = 0x0034;
        abus.cpu_write_8(0x12, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x001246);
        assert_eq!((0x7FABCD, WrappingMode::AddrSpace), cpu.dir_ptr_16_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x001246);
        // Wrapping
        abus.cpu_write_8(0xCB, 0x7E0011);
        abus.page_wrapping_cpu_write_16(0xABCD, 0x0012FF);
        assert_eq!((0x7FABCD, WrappingMode::AddrSpace), cpu.dir_ptr_16_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.page_wrapping_cpu_write_16(0x0000, 0x0012FF);
        // (Direct),Y
        cpu.d = 0x1234;
        cpu.y = 0x2345;
        abus.cpu_write_8(0x56, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x00128A);
        assert_eq!((0x7FCF12, WrappingMode::AddrSpace), cpu.dir_ptr_16_y(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x00128A);
        // Wrapping on pointer and +Y
        cpu.db = 0xFF;
        cpu.d = 0xFF34;
        cpu.y = 0x5678;
        abus.cpu_write_8(0xCB, 0x7E0011);
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x00FFFF);
        assert_eq!((0x000245, WrappingMode::AddrSpace), cpu.dir_ptr_16_y(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x00FFFF);
        // Emumode and DL is $00
        cpu.db = 0xFF;
        cpu.d = 0x1200;
        cpu.y = 0x5678;
        abus.cpu_write_8(0xFF, 0x7E0011);
        abus.page_wrapping_cpu_write_16(0xABCD, 0x0012FF);
        assert_eq!((0x000245, WrappingMode::AddrSpace), cpu.dir_ptr_16_y(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.page_wrapping_cpu_write_16(0x0000, 0x0012FF);
        // [Direct],Y
        cpu.d = 0x1234;
        cpu.y = 0x2345;
        abus.cpu_write_8(0x56, 0x7E0011);
        abus.bank_wrapping_cpu_write_24(0xABCDEF, 0x00128A);
        assert_eq!((0xABF134, WrappingMode::AddrSpace), cpu.dir_ptr_24_y(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_24(0x000000, 0x00128A);
        // Wrapping on pointer and +Y
        cpu.db = 0xFF;
        cpu.d = 0xFF34;
        cpu.y = 0x5678;
        abus.cpu_write_8(0xCB, 0x7E0011);
        abus.bank_wrapping_cpu_write_24(0xFFABCD, 0x00FFFF);
        assert_eq!((0x000245, WrappingMode::AddrSpace), cpu.dir_ptr_24_y(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_24(0x000000, 0x00FFFF);
    }

    #[test]
    fn addr_imm() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = Cpu::new(&mut abus);
        cpu.pb = 0x12;
        cpu.pc = 0x3456;
        assert_eq!(0x123457, cpu.imm(0x123456, &mut abus).0);
        // Wrapping
        cpu.pc = 0xFFFF;
        assert_eq!(0x120000, cpu.imm(0x12FFFF, &mut abus).0);
    }

    #[test]
    fn addr_long() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = Cpu::new(&mut abus);
        cpu.pb = 0x00;
        cpu.pc = 0x0000;
        // Long
        abus.bank_wrapping_cpu_write_24(0xABCDEF, 0x000001);
        assert_eq!((0xABCDEF, WrappingMode::AddrSpace), cpu.long(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        // Long, X
        cpu.x = 0x1234;
        abus.bank_wrapping_cpu_write_24(0xABCDEF, 0x000001);
        assert_eq!((0xABE023, WrappingMode::AddrSpace), cpu.long_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        // Wrapping
        cpu.x = 0x5678;
        abus.bank_wrapping_cpu_write_24(0xFFCDEF, 0x000001);
        assert_eq!((0x002467, WrappingMode::AddrSpace), cpu.long_x(addr_8_16(cpu.pb, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_24(0x000000, 0x000001);
    }

    #[test]
    fn addr_rel() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = Cpu::new(&mut abus);
        cpu.pb = 0x45;
        cpu.pc = 0x0123;
        // Relative8
        abus.cpu_write_8(0x01, 0x000124);
        assert_eq!((0x450126, WrappingMode::Bank), cpu.rel_8(addr_8_16(0x00, cpu.pc), &mut abus));
        abus.cpu_write_8(0x7F, 0x000124);
        assert_eq!((0x4501A4, WrappingMode::Bank), cpu.rel_8(addr_8_16(0x00, cpu.pc), &mut abus));
        abus.cpu_write_8(0xFF, 0x000124);
        assert_eq!((0x450124, WrappingMode::Bank), cpu.rel_8(addr_8_16(0x00, cpu.pc), &mut abus));
        abus.cpu_write_8(0x80, 0x000124);
        assert_eq!((0x4500A5, WrappingMode::Bank), cpu.rel_8(addr_8_16(0x00, cpu.pc), &mut abus));
        abus.cpu_write_8(0x00, 0x000124);
        // Wrapping
        cpu.pc = 0xFFEF;
        abus.cpu_write_8(0x7F, 0x00FFF0);
        assert_eq!((0x450070, WrappingMode::Bank), cpu.rel_8(addr_8_16(0x00, cpu.pc), &mut abus));
        abus.cpu_write_8(0x00, 0x00FFF0);
        cpu.pc = 0x0010;
        abus.cpu_write_8(0x80, 0x000011);
        assert_eq!((0x45FF92, WrappingMode::Bank), cpu.rel_8(addr_8_16(0x00, cpu.pc), &mut abus));
        abus.cpu_write_8(0x00, 0x000011);
        // Relative16
        cpu.pc = 0x0123;
        abus.bank_wrapping_cpu_write_16(0x4567, 0x000124);
        assert_eq!((0x45468D, WrappingMode::Bank), cpu.rel_16(addr_8_16(0x00, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x000124);
        // Wrapping
        cpu.pc = 0xABCD;
        abus.bank_wrapping_cpu_write_16(0x6789, 0x00ABCE);
        assert_eq!((0x451359, WrappingMode::Bank), cpu.rel_16(addr_8_16(0x00, cpu.pc), &mut abus));
        abus.bank_wrapping_cpu_write_16(0x0000, 0x00ABCE);
    }

    #[test]
    fn addr_stack() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = Cpu::new(&mut abus);
        // Stack,S
        cpu.s = 0x0123;
        abus.cpu_write_8(0x45, 0x000001);
        assert_eq!((0x000168, WrappingMode::Bank), cpu.stack(0x000000, &mut abus));
        abus.cpu_write_8(0x00, 0x000001);
        // Wrapping
        cpu.s = 0xFFDE;
        abus.cpu_write_8(0x45, 0x000001);
        assert_eq!((0x000023, WrappingMode::Bank), cpu.stack(0x000000, &mut abus));
        abus.cpu_write_8(0x00, 0x000001);
        // (Stack,S),Y
        cpu.db = 0x9A;
        cpu.s = 0x0123;
        cpu.y = 0x1234;
        abus.cpu_write_8(0x45, 0x000001);
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x000168);
        assert_eq!((0x9ABE01, WrappingMode::AddrSpace), cpu.stack_ptr_y(0x000000, &mut abus));
        abus.cpu_write_8(0x00, 0x000001);
        abus.bank_wrapping_cpu_write_16(0x0000, 0x000168);
        // Wrapping on pointer and +Y
        cpu.db = 0xFF;
        cpu.s = 0xFFBA;
        cpu.y = 0x5678;
        abus.cpu_write_8(0x45, 0x000001);
        abus.bank_wrapping_cpu_write_16(0xABCD, 0x00FFFF);
        assert_eq!((0x000245, WrappingMode::AddrSpace), cpu.stack_ptr_y(0x000000, &mut abus));
        abus.cpu_write_8(0x00, 0x000001);
        abus.bank_wrapping_cpu_write_16(0x0000, 0x00FFFF);
    }

    #[test]
    fn bcd_conversions() {
        assert_eq!(0x99, dec_to_bcd_8(99));
        assert_eq!(0x9999, dec_to_bcd_16(9999));
        assert_eq!(99, bcd_to_dec_8(0x99));
        assert_eq!(9999, bcd_to_dec_16(0x9999))
    }

    #[test]
    fn op_adc() {
        // NOTE: Currently doesn't check actual rule boundaries,
        //       real BCD V flag behaviour should be verified if actually needed
        let mut abus = ABus::new_empty_rom();
        let mut cpu = Cpu::new(&mut abus);
        cpu.e = false;

        // 8bit binary arithmetic
        cpu.a = 0x2345;
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.m = true;
        abus.bank_wrapping_cpu_write_16(0x6789, 0x000000);
        let data_addr = (0x000000, WrappingMode::Page);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x23CE, cpu.a);
        // Wrapping add, C set
        assert_eq!(false, cpu.p.c);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2357, cpu.a);
        assert_eq!(true, cpu.p.c);
        // Add with carry, C cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x23E1, cpu.a);
        assert_eq!(false, cpu.p.c);
        // Z, C set
        abus.cpu_write_8(0x1F, 0x000000);
        assert_eq!(false, cpu.p.z);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2300, cpu.a);
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.z);
        // Z cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2320, cpu.a);
        assert_eq!(false, cpu.p.z);
        // N set
        abus.cpu_write_8(0x7A, 0x000000);
        assert_eq!(false, cpu.p.n);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x239A, cpu.a);
        assert_eq!(true, cpu.p.n);
        // N cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2314, cpu.a);
        assert_eq!(false, cpu.p.n);
        // V set (positive)
        assert_eq!(false, cpu.p.v);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x238F, cpu.a);
        assert_eq!(true, cpu.p.v);
        // V set (negative)
        abus.cpu_write_8(0xCF, 0x000000);
        cpu.p.v = false;
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x235E, cpu.a);
        assert_eq!(true, cpu.p.v);
        // V cleared (positive)
        abus.cpu_write_8(0x20, 0x000000);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x237F, cpu.a);
        assert_eq!(false, cpu.p.v);
        // V cleared (negative)
        cpu.a = 0x23AF;
        cpu.p.v = true;
        abus.cpu_write_8(0xDF, 0x000000);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x238E, cpu.a);
        assert_eq!(false, cpu.p.v);
        abus.bank_wrapping_cpu_write_16(0x0000, 0x000000);

        // 8bit BCD arithmetic
        cpu.a = 0x2345;
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.m = true;
        cpu.p.d = true;
        abus.bank_wrapping_cpu_write_16(0x1234, 0x000000);
        let data_addr = (0x000000, WrappingMode::Page);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2379, cpu.a);
        // Wrapping add, C set
        assert_eq!(false, cpu.p.c);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2313, cpu.a);
        assert_eq!(true, cpu.p.c);
        // Add with carry, C cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2348, cpu.a);
        assert_eq!(false, cpu.p.c);
        // Z, C set
        abus.cpu_write_8(0x52, 0x000000);
        assert_eq!(false, cpu.p.z);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2300, cpu.a);
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        // Z cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2353, cpu.a);
        assert_eq!(false, cpu.p.z);
        // N set
        abus.cpu_write_8(0x34, 0x000000);
        assert_eq!(false, cpu.p.n);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2387, cpu.a);
        assert_eq!(true, cpu.p.n);
        // N cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2321, cpu.a);
        assert_eq!(false, cpu.p.n);
        // V set (positive)
        abus.cpu_write_8(0x74, 0x000000);
        assert_eq!(false, cpu.p.v);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2396, cpu.a);
        assert_eq!(true, cpu.p.v);
        // V set (negative)
        cpu.a = 0x2382;
        abus.cpu_write_8(0x84, 0x000000);
        cpu.p.v = false;
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2366, cpu.a);
        assert_eq!(true, cpu.p.v);
        // V cleared (positive)
        cpu.a = 0x2322;
        abus.cpu_write_8(0x24, 0x000000);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2347, cpu.a);
        assert_eq!(false, cpu.p.v);
        // V cleared (negative)
        cpu.a = 0x2392;
        abus.cpu_write_8(0x94, 0x000000);
        cpu.p.v = true;
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2386, cpu.a);
        assert_eq!(false, cpu.p.v);
        abus.bank_wrapping_cpu_write_16(0x0000, 0x000000);

        // 16bit binary arithmetic
        cpu.a = 0x5678;
        cpu.p.clear_flags(0b1111_1111);
        abus.bank_wrapping_cpu_write_16(0x6789, 0x000000);
        let data_addr = (0x000000, WrappingMode::Page);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0xBE01, cpu.a);
        // Wrapping add, C set
        assert_eq!(false, cpu.p.c);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x258A, cpu.a);
        assert_eq!(true, cpu.p.c);
        // Add with carry, C cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x8D14, cpu.a);
        assert_eq!(false, cpu.p.c);
        // Z, C set
        abus.bank_wrapping_cpu_write_16(0x72EC, 0x000000);
        assert_eq!(false, cpu.p.z);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x0000, cpu.a);
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        // Z cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x72ED, cpu.a);
        assert_eq!(false, cpu.p.z);
        // N set
        assert_eq!(false, cpu.p.n);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0xE5D9, cpu.a);
        assert_eq!(true, cpu.p.n);
        // N cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x58C5, cpu.a);
        assert_eq!(false, cpu.p.n);
        // V set (positive)
        assert_eq!(false, cpu.p.v);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0xCBB2, cpu.a);
        assert_eq!(true, cpu.p.v);
        // V set (negative)
        abus.bank_wrapping_cpu_write_16(0x9432, 0x000000);
        cpu.p.v = false;
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x5FE4, cpu.a);
        assert_eq!(true, cpu.p.v);
        // V cleared (positive)
        abus.bank_wrapping_cpu_write_16(0x1234, 0x000000);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x7219, cpu.a);
        assert_eq!(false, cpu.p.v);
        // V cleared (negative)
        cpu.a = 0xA234;
        abus.bank_wrapping_cpu_write_16(0xDEFF, 0x000000);
        cpu.p.v = true;
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x8133, cpu.a);
        assert_eq!(false, cpu.p.v);
        cpu.p.c = false;
        abus.bank_wrapping_cpu_write_16(0x0000, 0x000000);
        // Data page wrapping
        cpu.a = 0x1234;
        abus.page_wrapping_cpu_write_16(0x2345, 0x0000FF);
        let data_addr = (0x0000FF, WrappingMode::Page);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3579, cpu.a);
        abus.page_wrapping_cpu_write_16(0x0000, 0x0000FF);
        // Data bank wrapping
        cpu.a = 0x1234;
        abus.bank_wrapping_cpu_write_16(0x2345, 0x00FFFF);
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3579, cpu.a);
        abus.bank_wrapping_cpu_write_16(0x0000, 0x00FFFF);
        // Data address space wrapping
        cpu.a = 0x1234;
        abus.addr_wrapping_cpu_write_16(0x2345, 0xFFFFFF);
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3579, cpu.a);
        abus.addr_wrapping_cpu_write_16(0x0000, 0xFFFFFF);

        // 16bit BCD arithmetic
        cpu.a = 0x2345;
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.d = true;
        abus.bank_wrapping_cpu_write_16(0x5678, 0x000000);
        let data_addr = (0x000000, WrappingMode::Page);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x8023, cpu.a);
        // Wrapping add, C set
        assert_eq!(false, cpu.p.c);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3701, cpu.a);
        assert_eq!(true, cpu.p.c);
        // Add with carry, C cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x9380, cpu.a);
        assert_eq!(false, cpu.p.c);
        // Z, C set
        abus.bank_wrapping_cpu_write_16(0x0620, 0x000000);
        assert_eq!(false, cpu.p.z);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x0000, cpu.a);
        assert_eq!(true, cpu.p.z);
        // Z cleared
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x0621, cpu.a);
        assert_eq!(false, cpu.p.z);
        // N set
        abus.bank_wrapping_cpu_write_16(0x8620, 0x000000);
        assert_eq!(false, cpu.p.n);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x9241, cpu.a);
        assert_eq!(true, cpu.p.n);
        // N cleared
        abus.bank_wrapping_cpu_write_16(0x4620, 0x000000);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3861, cpu.a);
        assert_eq!(false, cpu.p.n);
        // V set (positive)
        assert_eq!(false, cpu.p.v);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x8482, cpu.a);
        assert_eq!(true, cpu.p.v);
        // V set (negative)
        abus.bank_wrapping_cpu_write_16(0x8620, 0x000000);
        cpu.p.v = false;
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x7102, cpu.a);
        assert_eq!(true, cpu.p.v);
        // V cleared (positive)
        abus.bank_wrapping_cpu_write_16(0x0620, 0x000000);
        cpu.p.v = true;
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x7723, cpu.a);
        assert_eq!(false, cpu.p.v);
        // V cleared (negative)
        cpu.a = 0x9876;
        abus.bank_wrapping_cpu_write_16(0x9678, 0x000000);
        cpu.p.v = true;
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x9554, cpu.a);
        assert_eq!(false, cpu.p.v);
        cpu.p.c = false;
        abus.bank_wrapping_cpu_write_16(0x0000, 0x000000);
        // Data page wrapping
        cpu.a = 0x1234;
        abus.page_wrapping_cpu_write_16(0x2345, 0x0000FF);
        let data_addr = (0x0000FF, WrappingMode::Page);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3579, cpu.a);
        abus.page_wrapping_cpu_write_16(0x0000, 0x0000FF);
        // Data bank wrapping
        cpu.a = 0x1234;
        abus.bank_wrapping_cpu_write_16(0x2345, 0x00FFFF);
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3579, cpu.a);
        abus.bank_wrapping_cpu_write_16(0x0000, 0x00FFFF);
        // Data address space wrapping
        cpu.a = 0x1234;
        abus.addr_wrapping_cpu_write_16(0x2345, 0xFFFFFF);
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3579, cpu.a);
        abus.addr_wrapping_cpu_write_16(0x0000, 0xFFFFFF);
        }
    }

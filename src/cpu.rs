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
            pc: abus.page_wrapping_cpu_read_16(0x00FF00 + RESET8 as u32), // TODO: This only applies to LoROM
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
                self.clear_p_c();
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
                let data_addr = self.abs(addr, abus).0;
                if !self.get_p_m() {
                    abus.addr_wrapping_cpu_write_16(self.a, data_addr);
                } else {
                    abus.cpu_write_8(self.a as u8, data_addr);
                }
                self.pc = self.pc.wrapping_add(3);
            }
            op::STX_8E => {
                let data_addr = self.abs(addr, abus).0;
                if !self.get_p_x() {
                    abus.addr_wrapping_cpu_write_16(self.x, data_addr);
                } else {
                    abus.cpu_write_8(self.x as u8, data_addr);
                }
                self.pc = self.pc.wrapping_add(3);
            }
            op::TXS => {
                if self.get_emumode() {
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
                let data_addr = self.abs(addr, abus).0;
                if !self.get_p_m() {
                    abus.addr_wrapping_cpu_write_16(0, data_addr);
                } else {
                    abus.cpu_write_8(0, data_addr);
                }
                self.pc = self.pc.wrapping_add(3);
            }
            op::LDX_A2 => {
                if self.get_p_x() {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.cpu_read_8(data_addr.0) as u16;
                    self.x = data;
                    self.update_p_z(data);
                    self.update_p_n_8(data as u8);
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.bank_wrapping_cpu_read_16(data_addr.0);
                    self.x = data;
                    self.update_p_z(data);
                    self.update_p_n_16(data);
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::LDA_A9 => {
                if self.get_p_m() {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.cpu_read_8(data_addr.0) as u16;
                    self.a = (self.a & 0xFF00) + data;
                    self.update_p_z(data);
                    self.update_p_n_8(data as u8);
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.bank_wrapping_cpu_read_16(data_addr.0);
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
                let data_addr = self.abs(addr, abus).0;
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
                let data_addr = self.imm(addr, abus);
                let bits = abus.cpu_read_8(data_addr.0);
                self.p &= !bits;
                // Emulation forces M and X to 1
                if self.get_emumode() {
                    self.set_p_m();
                    self.set_p_x();
                }
                self.pc = self.pc.wrapping_add(2);
            }
            op::BNE => {
                if !self.get_p_z() {
                    self.pc = self.rel_8(addr, abus).0 as u16;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            op::CPX_E0 => {
                if self.get_p_x() {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.cpu_read_8(data_addr.0);
                    let result = (self.x as u8).wrapping_sub(data);// TODO: Matches binary subtraction?
                    self.update_p_n_8(result);
                    self.update_p_z(result as u16);
                    if (self.x as u8) < data {
                        self.clear_p_c();
                    } else {
                        self.set_p_c();
                    }
                    self.pc = self.pc.wrapping_add(2);
                } else {
                    let data_addr = self.imm(addr, abus);
                    let data = abus.bank_wrapping_cpu_read_16(data_addr.0);
                    let result = self.x.wrapping_sub(data);// TODO: Matches binary subtraction?
                    self.update_p_n_16(result);
                    self.update_p_z(result);
                    if self.x < data {
                        self.clear_p_c();
                    } else {
                        self.set_p_c();
                    }
                    self.pc = self.pc.wrapping_add(3);
                }
            }
            op::SEP => {
                let data_addr = self.imm(addr, abus);
                let bits = abus.cpu_read_8(data_addr.0);
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
                if self.get_p_c(){
                    self.set_emumode();
                } else {
                    self.clear_emumode();
                }
                if tmp {
                    self.set_p_c();
                } else {
                    self.clear_p_c();
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
        if self.get_emumode() && (self.d & 0xFF) == 0 {
            ((self.d | (abus.fetch_operand_8(addr).wrapping_add(self.x as u8) as u16)) as u32,
             WrappingMode::Page)
        } else {
            (self.d.wrapping_add(abus.fetch_operand_8(addr) as u16).wrapping_add(self.x) as u32,
             WrappingMode::Bank)
        }
    }

    fn dir_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        if self.get_emumode() && (self.d & 0xFF) == 0 {
            ((self.d | (abus.fetch_operand_8(addr).wrapping_add(self.y as u8) as u16)) as u32,
             WrappingMode::Page)
        } else {
            (self.d.wrapping_add(abus.fetch_operand_8(addr) as u16).wrapping_add(self.y) as u32,
             WrappingMode::Bank)
        }
    }

    fn dir_ptr_16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.dir(addr, abus).0;
        if self.get_emumode() && (self.d & 0xFF) == 0 {
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
        if self.get_emumode() && (self.d & 0xFF) == 0 {
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
        if self.get_emumode() {
            abus.page_wrapping_cpu_write_16(value, self.s as u32);
        } else {
            abus.bank_wrapping_cpu_write_16(value, self.s as u32);
        }
        self.decrement_s(1);
    }

    fn push_24(&mut self, value: u32, abus: &mut ABus) {
        self.decrement_s(2);
        if self.get_emumode() {
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
        if self.get_emumode() {
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
        if self.get_emumode() {
            value = abus.page_wrapping_cpu_read_24(self.s as u32);
        } else {
            value = abus.bank_wrapping_cpu_read_24(self.s as u32);
        }
        self.increment_s(2);
        value
    }

    fn decrement_s(&mut self, offset: u8) {
        if self.get_emumode() {
            self.s = 0x0100 | (self.s as u8).wrapping_sub(offset) as u16;
        } else {
            self.s = self.s.wrapping_sub(offset as u16);
        }
    }

    fn increment_s(&mut self, offset: u8) {
        if self.get_emumode() {
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
    pub fn get_emumode(&self) -> bool { self.e }

    fn set_p_c(&mut self) { self.p |= P_C }
    fn set_p_z(&mut self) { self.p |= P_Z }
    fn set_p_i(&mut self) { self.p |= P_I }
    fn set_p_d(&mut self) { self.p |= P_D }
    fn set_p_x(&mut self) {
        self.p |= P_X;
        // X = 1 forces XH and YH to 0x00
        self.x &= 0x00FF;
        self.y &= 0x00FF;
    }
    fn set_p_m(&mut self) { self.p |= P_M }
    fn set_p_v(&mut self) { self.p |= P_V }
    fn set_p_n(&mut self) { self.p |= P_N }
    fn set_emumode(&mut self) {
        self.e = true;
        self.set_p_m();
        self.set_p_x();
        self.s = 0x0100 | (self.s & 0x00FF);
    }

    fn clear_p_c(&mut self) { self.p &= !(P_C) }
    fn clear_p_z(&mut self) { self.p &= !(P_Z) }
    fn clear_p_i(&mut self) { self.p &= !(P_I) }
    fn clear_p_d(&mut self) { self.p &= !(P_D) }
    fn clear_p_x(&mut self) { self.p &= !(P_X) }
    fn clear_p_m(&mut self) { self.p &= !(P_M) }
    fn clear_p_v(&mut self) { self.p &= !(P_V) }
    fn clear_p_n(&mut self) { self.p &= !(P_N) }
    fn clear_emumode(&mut self) { self.e = false }

    fn update_p_z(&mut self, result: u16) {
        if result == 0 {
            self.set_p_z();
        } else {
            self.clear_p_z();
        }
    }

    fn update_p_n_8(&mut self, result: u8) {
        if result > 0x7F {
            self.set_p_n();
        } else {
            self.clear_p_n();
        }
    }

    fn update_p_n_16(&mut self, result: u16) {
        if result > 0x7FFF {
            self.set_p_n();
        } else {
            self.clear_p_n();
        }
    }

    // Instructions
    fn op_adc(&mut self, data_addr: (u32, WrappingMode), abus: &mut ABus) {
        if self.get_p_m() { // 8-bit accumulator
            let data = abus.cpu_read_8(data_addr.0);
            let result: u8;
            if self.get_p_c() { // BCD arithmetic
                let decimal_acc = bcd_to_dec_8(self.a as u8);
                let decimal_data = bcd_to_dec_8(data);
                let decimal_result = if self.get_p_c() { decimal_acc + decimal_data + 1 }
                                     else              { decimal_acc + decimal_data };
                let bcd_result = dec_to_bcd_8(decimal_result % 100);
                if decimal_result > 99 {
                    self.set_p_c();
                } else {
                    self.clear_p_c();
                }
                result = bcd_result;
            } else { // Binary arithmetic
                let acc_8 = self.a as u8;
                let result_16 = if self.get_p_c() { acc_8 as u16 + (data as u16) + 1 }
                                else              { acc_8 as u16 + (data as u16) };
                if result_16 > 0xFF {
                    self.set_p_c();
                } else {
                    self.clear_p_c();
                }
                result = result_16 as u8;
            }
            self.update_p_n_8(result);
            self.update_p_z(result as u16);
            if (self.a < 0x80 && data < 0x80 && result > 0x7F) ||
               (self.a > 0x7F && data > 0x7F && result < 0x80) {
                self.set_p_v();
            } else {
                self.clear_p_v();
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
            if self.get_p_c() { // BCD arithmetic
                let decimal_acc = bcd_to_dec_16(self.a);
                let decimal_data = bcd_to_dec_16(data);
                let decimal_result = if self.get_p_c() { decimal_acc + decimal_data + 1 }
                                     else              { decimal_acc + decimal_data };
                let bcd_result = dec_to_bcd_16(decimal_result % 10000);
                if decimal_result > 9999 {
                    self.set_p_c();
                } else {
                    self.clear_p_c();
                }
                result = bcd_result;
            } else { // Binary arithmetic
                let result_32 = if self.get_p_c() { (self.a as u32) + (data as u32) + 1 }
                                else              { (self.a as u32) + (data as u32) };
                if result_32 > 0xFFFF {
                    self.set_p_c();
                } else {
                    self.clear_p_c();
                }
                result = result_32 as u16;
            }
            self.update_p_n_16(result as u16);
            self.update_p_z(result as u16);
            if (self.a < 0x8000 && data < 0x8000 && result > 0x7FFF) ||
               (self.a > 0x7FFF && data > 0x7FFF && result < 0x8000) {
                self.set_p_v();
            } else {
                self.clear_p_v();
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
        cpu.clear_emumode() ;
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
        cpu.clear_emumode() ;
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
}

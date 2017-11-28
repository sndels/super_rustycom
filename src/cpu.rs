use abus::ABus;
use mmap;
use op;

pub struct W65C816S {
    a: u16,        // Accumulator
    x: u16,        // Index register
    y: u16,        // Index register
    pc: u16,       // Program counter
    s: u16,        // Stack pointer
    p: StatusReg,  // Processor status register
    d: u16,        // Zeropage offset
    pb: u8,        // Program counter bank
    db: u8,        // Data bank
    e: bool,       // Emulation mode
    stopped: bool, // Stopped until an interrupt
    waiting: bool, // Waiting for an interrupt
}

impl W65C816S {
    pub fn new(abus: &mut ABus) -> W65C816S {
        W65C816S {
            a: 0x00,
            x: 0x00,
            y: 0x00,
            pc: mmap::page_wrapping_cpu_read16(abus, RESET8),
            s: 0x01FF,
            p: StatusReg::new(),
            d: 0x00,
            pb: 0x0,
            db: 0x0,
            e: true,
            stopped: false,
            waiting: false,
        }
    }

    pub fn step(&mut self, abus: &mut ABus) {
        let addr = self.current_address();
        let opcode = mmap::cpu_read8(abus, addr);
        self.execute(opcode, addr, abus);
    }

    pub fn a(&self) -> u16 { self.a }
    pub fn x(&self) -> u16 { self.x }
    pub fn y(&self) -> u16 { self.y }
    pub fn pb(&self) -> u8 { self.pb }
    pub fn db(&self) -> u8 { self.db }
    pub fn pc(&self) -> u16 { self.pc }
    pub fn s(&self) -> u16 { self.s }
    pub fn d(&self) -> u16 { self.d }
    pub fn e(&self) -> bool { self.e }
    pub fn p_c(&self) -> bool { self.p.c }
    pub fn p_z(&self) -> bool { self.p.z }
    pub fn p_i(&self) -> bool { self.p.i }
    pub fn p_d(&self) -> bool { self.p.d }
    pub fn p_x(&self) -> bool { self.p.x }
    pub fn p_m(&self) -> bool { self.p.m }
    pub fn p_v(&self) -> bool { self.p.v }
    pub fn p_n(&self) -> bool { self.p.n }
    pub fn current_address(&self) -> u32 { ((self.pb as u32) << 16) + self.pc as u32 }

    fn execute(&mut self, opcode: u8, addr: u32, abus: &mut ABus) {
        // DRY macros
        macro_rules! op {
            ($addressing:ident, $op_func:ident, $op_length:expr) => ({
                let data_addr = self.$addressing(addr, abus);
                self.$op_func(&data_addr, abus);
                self.pc = self.pc.wrapping_add($op_length);
            });
        }

        macro_rules! inc_dec {
            ($cond:expr, $reg_ref:expr, $op:ident) => ({
                if $cond {
                    $reg_ref = ($reg_ref & 0xFF00) | ($reg_ref as u8).$op(1) as u16;
                    self.p.n = $reg_ref as u8 > 0x7F;
                    self.p.z = $reg_ref as u8 == 0;
                } else {
                    $reg_ref = $reg_ref.$op(1);
                    self.p.n = $reg_ref > 0x7FFF;
                    self.p.z = $reg_ref == 0;
                }
                self.pc = self.pc.wrapping_add(1);
            })
        }

        macro_rules! branch {
            ($condition:expr) => ({
               if $condition {
                    self.pc = self.rel8(addr, abus).0 as u16;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            })
        }

        macro_rules! cl_se {
            ($flag:expr, $value:expr) => ({
                $flag = $value;
                self.pc = self.pc.wrapping_add(1);
            })
        }

        macro_rules! push_eff {
            ($addressing:ident, $op_length:expr) => ({
                let data_addr = self.$addressing(addr, abus);
                let data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0);
                self.push16(data, abus);
                self.pc = self.pc.wrapping_add($op_length);
            })
        }

        macro_rules! push_reg {
            ($cond:expr, $reg_ref:expr) => ({
                if $cond {
                    let value = $reg_ref as u8;
                    self.push8(value, abus);
                } else {
                    let value = $reg_ref as u16;
                    self.push16(value, abus);
                }
                self.pc = self.pc.wrapping_add(1);
            })
        }

        macro_rules! pull_reg {
            ($cond:expr, $reg_ref:expr) => ({
                if $cond {
                    let value = self.pull8(abus);
                    $reg_ref = ($reg_ref & 0xFF00) | value as u16;
                    self.p.n = value > 0x7F;
                    self.p.z = value == 0;
                } else {
                    let value = self.pull16(abus);
                    $reg_ref = value;
                    self.p.n = value > 0x7FFF;
                    self.p.z = value == 0;
                }
                self.pc = self.pc.wrapping_add(1);
            })
        }

        macro_rules! transfer {
            ($cond:expr, $src_reg:expr, $dst_reg:expr) => ({
                if $cond {
                    $dst_reg = ($dst_reg & 0xFF00) | ($src_reg & 0x00FF);
                    self.p.n = $src_reg as u8 > 0x7F;
                    self.p.z = $src_reg as u8 == 0;
                } else {
                    $dst_reg = $src_reg;
                    self.p.n = $src_reg > 0x7FFF;
                    self.p.z = $src_reg == 0;
                }
                self.pc = self.pc.wrapping_add(1);
            })
        }

        match opcode {
            op::ADC_61 => op!(dir_x_ptr16, op_adc, 2),
            op::ADC_63 => op!(stack, op_adc, 2),
            op::ADC_65 => op!(dir, op_adc, 2),
            op::ADC_67 => op!(dir_ptr24, op_adc, 2),
            op::ADC_69 => op!(imm, op_adc, 3 - self.p.m as u16),
            op::ADC_6D => op!(abs, op_adc, 3),
            op::ADC_6F => op!(long, op_adc, 4),
            op::ADC_71 => op!(dir_ptr16_y, op_adc, 2),
            op::ADC_72 => op!(dir_ptr16, op_adc, 2),
            op::ADC_73 => op!(stack_ptr16_y, op_adc, 2),
            op::ADC_75 => op!(dir_x, op_adc, 2),
            op::ADC_77 => op!(dir_ptr24_y, op_adc, 2),
            op::ADC_79 => op!(abs_y, op_adc, 3),
            op::ADC_7D => op!(abs_x, op_adc, 3),
            op::ADC_7F => op!(long_x, op_adc, 4),
            op::SBC_E1 => op!(dir_x_ptr16, op_sbc, 2),
            op::SBC_E3 => op!(stack, op_sbc, 2),
            op::SBC_E5 => op!(dir, op_sbc, 2),
            op::SBC_E7 => op!(dir_ptr24, op_sbc, 2),
            op::SBC_E9 => op!(imm, op_sbc, 3 - self.p.m as u16),
            op::SBC_ED => op!(abs, op_sbc, 3),
            op::SBC_EF => op!(long, op_sbc, 4),
            op::SBC_F1 => op!(dir_ptr16_y, op_sbc, 2),
            op::SBC_F2 => op!(dir_ptr16, op_sbc, 2),
            op::SBC_F3 => op!(stack_ptr16_y, op_sbc, 2),
            op::SBC_F5 => op!(dir_x, op_sbc, 2),
            op::SBC_F7 => op!(dir_ptr24_y, op_sbc, 2),
            op::SBC_F9 => op!(abs_y, op_sbc, 3),
            op::SBC_FD => op!(abs_x, op_sbc, 3),
            op::SBC_FF => op!(long_x, op_sbc, 4),
            op::CMP_C1 => op!(dir_x_ptr16, op_cmp, 2),
            op::CMP_C3 => op!(stack, op_cmp, 2),
            op::CMP_C5 => op!(dir, op_cmp, 2),
            op::CMP_C7 => op!(dir_ptr24, op_cmp, 2),
            op::CMP_C9 => op!(imm, op_cmp, 3 - self.p.m as u16),
            op::CMP_CD => op!(abs, op_cmp, 3),
            op::CMP_CF => op!(long, op_cmp, 4),
            op::CMP_D1 => op!(dir_ptr16_y, op_cmp, 2),
            op::CMP_D2 => op!(dir_ptr16, op_cmp, 2),
            op::CMP_D3 => op!(stack_ptr16_y, op_cmp, 2),
            op::CMP_D5 => op!(dir_x, op_cmp, 2),
            op::CMP_D7 => op!(dir_ptr24_y, op_cmp, 2),
            op::CMP_D9 => op!(abs_y, op_cmp, 3),
            op::CMP_DD => op!(abs_x, op_cmp, 3),
            op::CMP_DF => op!(long_x, op_cmp, 4),
            op::CPX_E0 => op!(imm, op_cpx, 3 - self.p.x as u16),
            op::CPX_E4 => op!(dir, op_cpx, 2),
            op::CPX_EC => op!(abs, op_cpx, 3),
            op::CPY_C0 => op!(imm, op_cpy, 3 - self.p.x as u16),
            op::CPY_C4 => op!(dir, op_cpy, 2),
            op::CPY_CC => op!(abs, op_cpy, 3),
            op::DEC_3A => inc_dec!(self.p.m, *&mut self.a, wrapping_sub),
            op::DEC_C6 => op!(dir, op_dec, 2),
            op::DEC_CE => op!(abs, op_dec, 3),
            op::DEC_D6 => op!(dir_x, op_dec, 2),
            op::DEC_DE => op!(abs_x, op_dec, 3),
            op::DEX => inc_dec!(self.p.x, *&mut self.x, wrapping_sub),
            op::DEY => inc_dec!(self.p.x, *&mut self.y, wrapping_sub),
            op::INC_1A => inc_dec!(self.p.m, *&mut self.a, wrapping_add),
            op::INC_E6 => op!(dir, op_inc, 2),
            op::INC_EE => op!(abs, op_inc, 3),
            op::INC_F6 => op!(dir_x, op_inc, 2),
            op::INC_FE => op!(abs_x, op_inc, 3),
            op::INX => inc_dec!(self.p.x, *&mut self.x, wrapping_add),
            op::INY => inc_dec!(self.p.x, *&mut self.y, wrapping_add),
            op::AND_21 => op!(dir_x_ptr16, op_and, 2),
            op::AND_23 => op!(stack, op_and, 2),
            op::AND_25 => op!(dir, op_and, 2),
            op::AND_27 => op!(dir_ptr24, op_and, 2),
            op::AND_29 => op!(imm, op_and, 3 - self.p.m as u16),
            op::AND_2D => op!(abs, op_and, 3),
            op::AND_2F => op!(long, op_and, 4),
            op::AND_31 => op!(dir_ptr16_y, op_and, 2),
            op::AND_32 => op!(dir_ptr16, op_and, 2),
            op::AND_33 => op!(stack_ptr16_y, op_and, 2),
            op::AND_35 => op!(dir_x, op_and, 2),
            op::AND_37 => op!(dir_ptr24_y, op_and, 2),
            op::AND_39 => op!(abs_y, op_and, 3),
            op::AND_3D => op!(abs_x, op_and, 3),
            op::AND_3F => op!(long_x, op_and, 4),
            op::EOR_41 => op!(dir_x_ptr16, op_eor, 2),
            op::EOR_43 => op!(stack, op_eor, 2),
            op::EOR_45 => op!(dir, op_eor, 2),
            op::EOR_47 => op!(dir_ptr24, op_eor, 2),
            op::EOR_49 => op!(imm, op_eor, 3 - self.p.m as u16),
            op::EOR_4D => op!(abs, op_eor, 3),
            op::EOR_4F => op!(long, op_eor, 4),
            op::EOR_51 => op!(dir_ptr16_y, op_eor, 2),
            op::EOR_52 => op!(dir_ptr16, op_eor, 2),
            op::EOR_53 => op!(stack_ptr16_y, op_eor, 2),
            op::EOR_55 => op!(dir_x, op_eor, 2),
            op::EOR_57 => op!(dir_ptr24_y, op_eor, 2),
            op::EOR_59 => op!(abs_y, op_eor, 3),
            op::EOR_5D => op!(abs_x, op_eor, 3),
            op::EOR_5F => op!(long_x, op_eor, 4),
            op::ORA_01 => op!(dir_x_ptr16, op_ora, 2),
            op::ORA_03 => op!(stack, op_ora, 2),
            op::ORA_05 => op!(dir, op_ora, 2),
            op::ORA_07 => op!(dir_ptr24, op_ora, 2),
            op::ORA_09 => op!(imm, op_ora, 3 - self.p.m as u16),
            op::ORA_0D => op!(abs, op_ora, 3),
            op::ORA_0F => op!(long, op_ora, 4),
            op::ORA_11 => op!(dir_ptr16_y, op_ora, 2),
            op::ORA_12 => op!(dir_ptr16, op_ora, 2),
            op::ORA_13 => op!(stack_ptr16_y, op_ora, 2),
            op::ORA_15 => op!(dir_x, op_ora, 2),
            op::ORA_17 => op!(dir_ptr24_y, op_ora, 2),
            op::ORA_19 => op!(abs_y, op_ora, 3),
            op::ORA_1D => op!(abs_x, op_ora, 3),
            op::ORA_1F => op!(long_x, op_ora, 4),
            op::BIT_24 => op!(dir, op_bit, 2),
            op::BIT_2C => op!(abs, op_bit, 3),
            op::BIT_34 => op!(dir_x, op_bit, 2),
            op::BIT_3C => op!(abs_x, op_bit, 3),
            op::BIT_89 => op!(imm, op_bit, 3 - self.p.m as u16),
            op::TRB_14 => op!(dir, op_trb, 2),
            op::TRB_1C => op!(abs, op_trb, 3),
            op::TSB_04 => op!(dir, op_tsb, 2),
            op::TSB_0C => op!(abs, op_tsb, 3),
            op::ASL_06 => op!(dir, op_asl, 2),
            op::ASL_0A => {
                if self.p.m {
                    let old_a = self.a as u8;
                    self.a = (self.a & 0xFF) | self.arithmetic_shift_left8(old_a) as u16;
                } else {
                    let old_a = self.a;
                    self.a = self.arithmetic_shift_left16(old_a);
                }
                self.pc = self.pc.wrapping_add(1);
            }
            op::ASL_0E => op!(abs, op_asl, 3),
            op::ASL_16 => op!(dir_x, op_asl, 2),
            op::ASL_1E => op!(abs_x, op_asl, 3),
            op::LSR_46 => op!(dir, op_lsr, 2),
            op::LSR_4A => {
                if self.p.m {
                    let old_a = self.a as u8;
                    self.a = (self.a & 0xFF) | self.logical_shift_right8(old_a) as u16;
                } else {
                    let old_a = self.a;
                    self.a = self.logical_shift_right16(old_a);
                }
                self.pc = self.pc.wrapping_add(1);
            }
            op::LSR_4E => op!(abs, op_lsr, 3),
            op::LSR_56 => op!(dir_x, op_lsr, 2),
            op::LSR_5E => op!(abs_x, op_lsr, 3),
            op::ROL_26 => op!(dir, op_rol, 2),
            op::ROL_2A => {
                if self.p.m {
                    let old_a = self.a as u8;
                    self.a = (self.a & 0xFF) | self.rotate_left8(old_a) as u16;
                } else {
                    let old_a = self.a;
                    self.a = self.rotate_left16(old_a);
                }
                self.pc = self.pc.wrapping_add(1);
            }
            op::ROL_2E => op!(abs, op_rol, 3),
            op::ROL_36 => op!(dir_x, op_rol, 2),
            op::ROL_3E => op!(abs_x, op_rol, 3),
            op::ROR_66 => op!(dir, op_ror, 2),
            op::ROR_6A => {
                if self.p.m {
                    let old_a = self.a as u8;
                    self.a = (self.a & 0xFF) | self.rotate_right8(old_a) as u16;
                } else {
                    let old_a = self.a;
                    self.a = self.rotate_right16(old_a);
                }
                self.pc = self.pc.wrapping_add(1);
            }
            op::ROR_6E => op!(abs, op_ror, 3),
            op::ROR_76 => op!(dir_x, op_ror, 2),
            op::ROR_7E => op!(abs_x, op_ror, 3),
            op::BCC => branch!(!self.p.c),
            op::BCS => branch!(self.p.c),
            op::BEQ => branch!(self.p.z),
            op::BMI => branch!(self.p.n),
            op::BNE => branch!(!self.p.z),
            op::BPL => branch!(!self.p.n),
            op::BRA => self.pc = self.rel8(addr, abus).0 as u16,
            op::BVC => branch!(!self.p.v),
            op::BVS => branch!(self.p.v),
            op::BRL => self.pc = self.rel16(addr, abus).0 as u16,
            op::JMP_4C => self.pc = self.abs(addr, abus).0 as u16,
            op::JMP_5C => {
                let jump_addr = self.long(addr, abus).0;
                self.pb = (jump_addr >> 16) as u8;
                self.pc = jump_addr as u16;
            }
            op::JMP_6C => self.pc = self.abs_ptr16(addr, abus).0 as u16,
            op::JMP_7C => self.pc = self.abs_x_ptr16(addr, abus).0 as u16,
            op::JMP_DC => {
                let jump_addr = self.abs_ptr24(addr, abus).0;
                self.pb = (jump_addr >> 16) as u8;
                self.pc = jump_addr as u16;
            }
            op::JSL => {
                let jump_addr = self.long(addr, abus).0;
                let pb = self.pb;
                self.push8(pb, abus);
                let pc = self.pc;
                self.push16(pc.wrapping_add(3), abus);
                self.pb = (jump_addr >> 16) as u8;
                self.pc = jump_addr as u16;
            }
            op::JSR_20 => {
                let jump_addr = self.abs(addr, abus).0;
                let return_addr = self.pc.wrapping_add(2);
                self.push16(return_addr, abus);
                self.pc = jump_addr as u16;
            }
            op::JSR_FC => {
                let jump_addr = self.abs_x_ptr16(addr, abus).0;
                let return_addr = self.pc.wrapping_add(2);
                self.push16(return_addr, abus);
                self.pc = jump_addr as u16;
            }
            op::RTL => {
                self.pc = self.pull16(abus).wrapping_add(1);
                self.pb = self.pull8(abus);
            }
            op::RTS => self.pc = self.pull16(abus).wrapping_add(1),
            op::BRK => {
                if self.e {
                    let pc = self.pc;
                    self.push16(pc.wrapping_add(2), abus);
                    let p = self.p.value();
                    self.push8(p | P_X, abus); // B(RK)-flag is set to distinguish BRK from IRQ
                    self.pb = 0x00;
                    self.pc = mmap::page_wrapping_cpu_read16(abus, IRQBRK8);
                } else {
                    let pb = self.pb;
                    self.push8(pb, abus);
                    let pc = self.pc;
                    self.push16(pc.wrapping_add(2), abus);
                    let p = self.p.value();
                    self.push8(p, abus);
                    self.pb = 0x00;
                    self.pc = mmap::page_wrapping_cpu_read16(abus, BRK16);
                }
                self.p.i = true;
                self.p.d = false;
            }
            op::COP => {
                if self.e {
                    let pc = self.pc;
                    self.push16(pc.wrapping_add(2), abus);
                    let p = self.p.value();
                    self.push8(p, abus);
                    self.pb = 0x00;
                    self.pc = mmap::page_wrapping_cpu_read16(abus, COP8);
                } else {
                    let pb = self.pb;
                    self.push8(pb, abus);
                    let pc = self.pc;
                    self.push16(pc.wrapping_add(2), abus);
                    let p = self.p.value();
                    self.push8(p, abus);
                    self.pb = 0x00;
                    self.pc = mmap::page_wrapping_cpu_read16(abus, COP16);
                }
                self.p.i = true;
                self.p.d = false;
            }
            op::RTI => {
                let p = self.pull8(abus);
                self.p.set_value(p);
                let pc = self.pull16(abus);
                self.pc = pc;
                if !self.e {
                    let pb = self.pull8(abus);
                    self.pb = pb;
                }
            }
            op::CLC => cl_se!(*&mut self.p.c, false),
            op::CLD => cl_se!(*&mut self.p.d, false),
            op::CLI => cl_se!(*&mut self.p.i, false),
            op::CLV => cl_se!(*&mut self.p.v, false),
            op::SEC => cl_se!(*&mut self.p.c, true),
            op::SED => cl_se!(*&mut self.p.d, true),
            op::SEI => cl_se!(*&mut self.p.i, true),
            op::REP => {
                let data_addr = self.imm(addr, abus);
                let mask = mmap::cpu_read8(abus, data_addr.0);
                self.p.clear_flags(mask);
                // Emulation forces M and X to 1
                if self.e {
                    self.p.m = true;
                    self.p.x = true;
                }
                self.pc = self.pc.wrapping_add(2);
            }
            op::SEP => {
                let data_addr = self.imm(addr, abus);
                let mask = mmap::cpu_read8(abus, data_addr.0);
                self.p.set_flags(mask);
                if self.p.x {
                    self.x &= 0x00FF;
                    self.y &= 0x00FF;
                }
                self.pc = self.pc.wrapping_add(2);
            }
            op::LDA_A1 => op!(dir_x_ptr16, op_lda, 2),
            op::LDA_A3 => op!(stack, op_lda, 2),
            op::LDA_A5 => op!(dir, op_lda, 2),
            op::LDA_A7 => op!(dir_ptr24, op_lda, 2),
            op::LDA_A9 => op!(imm, op_lda, 3 - self.p.m as u16),
            op::LDA_AD => op!(abs, op_lda, 3),
            op::LDA_AF => op!(long, op_lda, 4),
            op::LDA_B1 => op!(dir_ptr16_y, op_lda, 2),
            op::LDA_B2 => op!(dir_ptr16, op_lda, 2),
            op::LDA_B3 => op!(stack_ptr16_y, op_lda, 2),
            op::LDA_B5 => op!(dir_x, op_lda, 2),
            op::LDA_B7 => op!(dir_ptr24_y, op_lda, 2),
            op::LDA_B9 => op!(abs_y, op_lda, 3),
            op::LDA_BD => op!(abs_x, op_lda, 3),
            op::LDA_BF => op!(long_x, op_lda, 4),
            op::LDX_A2 => op!(imm, op_ldx, 3 - self.p.x as u16),
            op::LDX_A6 => op!(dir, op_ldx, 2),
            op::LDX_AE => op!(abs, op_ldx, 3),
            op::LDX_B6 => op!(dir_y, op_ldx, 2),
            op::LDX_BE => op!(abs_y, op_ldx, 3),
            op::LDY_A0 => op!(imm, op_ldy, 3 - self.p.x as u16),
            op::LDY_A4 => op!(dir, op_ldy, 2),
            op::LDY_AC => op!(abs, op_ldy, 3),
            op::LDY_B4 => op!(dir_x, op_ldy, 2),
            op::LDY_BC => op!(abs_x, op_ldy, 3),
            op::STA_81 => op!(dir_x_ptr16, op_sta, 2),
            op::STA_83 => op!(stack, op_sta, 2),
            op::STA_85 => op!(dir, op_sta, 2),
            op::STA_87 => op!(dir_ptr24, op_sta, 2),
            op::STA_8D => op!(abs, op_sta, 3),
            op::STA_8F => op!(long, op_sta, 4),
            op::STA_91 => op!(dir_ptr16_y, op_sta, 2),
            op::STA_92 => op!(dir_ptr16, op_sta, 2),
            op::STA_93 => op!(stack_ptr16_y, op_sta, 2),
            op::STA_95 => op!(dir_x, op_sta, 2),
            op::STA_97 => op!(dir_ptr24_y, op_sta, 2),
            op::STA_99 => op!(abs_y, op_sta, 3),
            op::STA_9D => op!(abs_x, op_sta, 3),
            op::STA_9F => op!(long_x, op_sta, 4),
            op::STX_86 => op!(dir, op_stx, 2),
            op::STX_8E => op!(abs, op_stx, 3),
            op::STX_96 => op!(dir_y, op_stx, 2),
            op::STY_84 => op!(dir, op_sty, 2),
            op::STY_8C => op!(abs, op_sty, 3),
            op::STY_94 => op!(dir_x, op_sty, 2),
            op::STZ_64 => op!(dir, op_stz, 2),
            op::STZ_74 => op!(dir_x, op_stz, 2),
            op::STZ_9C => op!(abs, op_stz, 3),
            op::STZ_9E => op!(abs_x, op_stz, 3),
            op::MVN => op!(src_dest, op_mvn, 0), // Op "loops" until move is complete
            op::MVP => op!(src_dest, op_mvp, 0), // Op "loops" until move is complete
            op::NOP => self.pc = self.pc.wrapping_add(1),
            op::WDM => self.pc = self.pc.wrapping_add(2),
            op::PEA => push_eff!(imm, 3),
            op::PEI => push_eff!(dir, 2),
            op::PER => push_eff!(imm, 3),
            op::PHA => push_reg!(self.p.m, *&self.a),
            op::PHX => push_reg!(self.p.x, *&self.x),
            op::PHY => push_reg!(self.p.x, *&self.y),
            op::PLA => pull_reg!(self.p.m, *&mut self.a),
            op::PLX => pull_reg!(self.p.x, *&mut self.x),
            op::PLY => pull_reg!(self.p.x, *&mut self.y),
            op::PHB => push_reg!(true, *&self.db),
            op::PHD => push_reg!(false, *&self.d),
            op::PHK => push_reg!(true, *&self.pb),
            op::PHP => push_reg!(true, *&self.p.value()),
            op::PLB => {
                let value = self.pull8(abus);
                self.db = value;
                self.p.n = value > 0x7F;
                self.p.z = value == 0;
                self.pc = self.pc.wrapping_add(1);
            }
            op::PLD => pull_reg!(false, *&mut self.d),
            op::PLP => {
                let value = self.pull8(abus);
                self.p.set_value(value);
                self.pc = self.pc.wrapping_add(1);
            }
            op::STP => self.stopped = true,
            op::WAI => self.waiting = true,
            op::TAX => transfer!(self.p.x, *&self.a, *&mut self.x),
            op::TAY => transfer!(self.p.x, *&self.a, *&mut self.y),
            op::TSX => transfer!(self.p.x, *&self.s, *&mut self.x),
            op::TXA => transfer!(self.p.m, *&self.x, *&mut self.a),
            op::TXS => {
                self.s = if self.e {
                    0x0100 | (self.x & 0x00FF)
                } else {
                    self.x
                };
                self.pc = self.pc.wrapping_add(1);
            }
            op::TXY => transfer!(self.p.x, *&self.x, *&mut self.y),
            op::TYA => transfer!(self.p.m, *&self.y, *&mut self.a),
            op::TYX => transfer!(self.p.x, *&self.y, *&mut self.x),
            op::TCD => transfer!(false, *&self.a, *&mut self.d),
            op::TCS => {
                self.s = if self.e {
                    0x0100 | (self.a & 0x00FF)
                } else {
                    self.a
                };
                self.pc = self.pc.wrapping_add(1);
            }
            op::TDC => transfer!(false, *&self.d, *&mut self.a),
            op::TSC => transfer!(false, *&self.s, *&mut self.a),
            op::XBA => {
                self.a = (self.a << 8) | (self.a >> 8);
                self.p.n = self.a as u8 > 0x7F;
                self.p.z = self.a as u8 > 0;
            }
            op::XCE => {
                let tmp = self.e;
                if self.p.c {
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
            _ => panic!(
                "Unknown opcode ${0:02X} at ${1:02X}:{2:04X}",
                opcode,
                addr >> 16,
                addr & 0xFFF
            ),
        }
    }

    // Addressing modes
    // TODO: DRY, mismatch funcs and macros?
    pub fn abs(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        // JMP, JSR use PB instead of DB
        (
            addr_8_16(self.db, mmap::fetch_operand16(abus, addr)),
            WrappingMode::AddrSpace,
        )
    }

    pub fn abs_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let operand = mmap::fetch_operand16(abus, addr);
        (
            (addr_8_16(self.db, operand) + self.x as u32) & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    pub fn abs_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let operand = mmap::fetch_operand16(abus, addr);
        (
            (addr_8_16(self.db, operand) + self.y as u32) & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    pub fn abs_ptr16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = mmap::fetch_operand16(abus, addr) as u32;
        (
            addr_8_16(self.pb, mmap::bank_wrapping_cpu_read16(abus, pointer)),
            WrappingMode::Bank,
        )
    }

    pub fn abs_ptr24(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = mmap::fetch_operand24(abus, addr) as u32;
        (
            mmap::bank_wrapping_cpu_read24(abus, pointer),
            WrappingMode::Bank,
        )
    }

    pub fn abs_x_ptr16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = addr_8_16(
            self.pb,
            mmap::fetch_operand16(abus, addr).wrapping_add(self.x),
        );
        (
            addr_8_16(self.pb, mmap::bank_wrapping_cpu_read16(abus, pointer)),
            WrappingMode::Bank,
        )
    }

    pub fn dir(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        // NOTE: Data of "old" instructions with e=1, DL=$00 "wrap" page but only access 8bit
        (
            self.d.wrapping_add(mmap::fetch_operand8(abus, addr) as u16) as u32,
            WrappingMode::Bank,
        )
    }

    pub fn dir_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        if self.e && (self.d & 0xFF) == 0 {
            (
                (self.d | (mmap::fetch_operand8(abus, addr).wrapping_add(self.x as u8) as u16))
                    as u32,
                WrappingMode::Page,
            )
        } else {
            (
                self.d
                    .wrapping_add(mmap::fetch_operand8(abus, addr) as u16)
                    .wrapping_add(self.x) as u32,
                WrappingMode::Bank,
            )
        }
    }

    pub fn dir_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        if self.e && (self.d & 0xFF) == 0 {
            (
                (self.d | (mmap::fetch_operand8(abus, addr).wrapping_add(self.y as u8) as u16))
                    as u32,
                WrappingMode::Page,
            )
        } else {
            (
                self.d
                    .wrapping_add(mmap::fetch_operand8(abus, addr) as u16)
                    .wrapping_add(self.y) as u32,
                WrappingMode::Bank,
            )
        }
    }

    pub fn dir_ptr16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.dir(addr, abus).0;
        if self.e && (self.d & 0xFF) == 0 {
            let ll = mmap::cpu_read8(abus, pointer);
            let hh = mmap::cpu_read8(
                abus,
                (pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32,
            );
            (addr_8_8_8(self.db, hh, ll), WrappingMode::AddrSpace)
        } else {
            (
                addr_8_16(self.db, mmap::bank_wrapping_cpu_read16(abus, pointer)),
                WrappingMode::AddrSpace,
            )
        }
    }

    pub fn dir_ptr24(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.dir(addr, abus).0;
        (
            mmap::bank_wrapping_cpu_read24(abus, pointer),
            WrappingMode::AddrSpace,
        )
    }

    pub fn dir_x_ptr16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.dir_x(addr, abus).0;
        if self.e && (self.d & 0xFF) == 0 {
            let ll = mmap::cpu_read8(abus, pointer);
            let hh = mmap::cpu_read8(
                abus,
                (pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32,
            );
            (addr_8_8_8(self.db, hh, ll), WrappingMode::AddrSpace)
        } else {
            (
                addr_8_16(self.db, mmap::bank_wrapping_cpu_read16(abus, pointer)),
                WrappingMode::AddrSpace,
            )
        }
    }

    pub fn dir_ptr16_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (
            (self.dir_ptr16(addr, abus).0 + self.y as u32) & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    pub fn dir_ptr24_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (
            (self.dir_ptr24(addr, abus).0 + self.y as u32) & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    #[allow(unused_variables)]
    pub fn imm(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (
            addr_8_16(self.pb, self.pc.wrapping_add(1)),
            WrappingMode::Bank,
        )
    }

    pub fn long(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (mmap::fetch_operand24(abus, addr), WrappingMode::AddrSpace)
    }

    pub fn long_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (
            (mmap::fetch_operand24(abus, addr) + self.x as u32) & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    pub fn rel8(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let offset = mmap::fetch_operand8(abus, addr);
        if offset < 0x80 {
            (
                addr_8_16(self.pb, self.pc.wrapping_add(2 + (offset as u16))),
                WrappingMode::Bank,
            )
        } else {
            (
                addr_8_16(
                    self.pb,
                    self.pc.wrapping_sub(254).wrapping_add(offset as u16),
                ),
                WrappingMode::Bank,
            )
        }
    }

    pub fn rel16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let offset = mmap::fetch_operand16(abus, addr);
        (
            addr_8_16(self.pb, self.pc.wrapping_add(3).wrapping_add(offset)),
            WrappingMode::Bank,
        )
    }

    pub fn src_dest(&self, addr: u32, abus: &mut ABus) -> (u32, u32) {
        let operand = mmap::fetch_operand16(abus, addr);
        (
            addr_8_16(operand as u8, self.x),
            addr_8_16((operand >> 8) as u8, self.y),
        )
    }

    pub fn stack(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (
            self.s.wrapping_add(mmap::fetch_operand8(abus, addr) as u16) as u32,
            WrappingMode::Bank,
        )
    }

    pub fn stack_ptr16_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.s.wrapping_add(mmap::fetch_operand8(abus, addr) as u16) as u32;
        (
            (addr_8_16(self.db, mmap::bank_wrapping_cpu_read16(abus, pointer)) + self.y as u32)
                & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    // Stack operations
    fn push8(&mut self, value: u8, abus: &mut ABus) {
        mmap::cpu_write8(abus, value, self.s as u32);
        self.decrement_s(1);
    }

    fn push16(&mut self, value: u16, abus: &mut ABus) {
        self.decrement_s(1);
        if self.e {
            mmap::page_wrapping_cpu_write16(abus, value, self.s as u32);
        } else {
            mmap::bank_wrapping_cpu_write16(abus, value, self.s as u32);
        }
        self.decrement_s(1);
    }

    #[allow(dead_code)]
    fn push24(&mut self, value: u32, abus: &mut ABus) {
        self.decrement_s(2);
        if self.e {
            mmap::page_wrapping_cpu_write24(abus, value, self.s as u32);
        } else {
            mmap::bank_wrapping_cpu_write24(abus, value, self.s as u32);
        }
        self.decrement_s(1);
    }

    fn pull8(&mut self, abus: &mut ABus) -> u8 {
        self.increment_s(1);
        mmap::cpu_read8(abus, self.s as u32)
    }

    fn pull16(&mut self, abus: &mut ABus) -> u16 {
        self.increment_s(1);
        let value: u16;
        if self.e {
            value = mmap::page_wrapping_cpu_read16(abus, self.s as u32);
        } else {
            value = mmap::bank_wrapping_cpu_read16(abus, self.s as u32);
        }
        self.increment_s(1);
        value
    }

    #[allow(dead_code)]
    fn pull24(&mut self, abus: &mut ABus) -> u32 {
        self.increment_s(1);
        let value: u32;
        if self.e {
            value = mmap::page_wrapping_cpu_read24(abus, self.s as u32);
        } else {
            value = mmap::bank_wrapping_cpu_read24(abus, self.s as u32);
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

    fn set_emumode(&mut self) {
        self.e = true;
        self.p.m = true;
        self.p.x = true;
        self.x &= 0x00FF;
        self.y &= 0x00FF;
        self.s = 0x0100 | (self.s & 0x00FF);
    }

    // Algorithms
    fn add_sub8(&mut self, lhs: u8, mut rhs: u8, subtraction: bool) -> u8 {
        let result8: u8;
        if self.p.d {
            // BCD arithmetic
            // TODO: Lower level arithmetic
            let dec_lhs = bcd_to_dec8(lhs);
            let dec_rhs = bcd_to_dec8(rhs);
            let dec_result: u8;
            if subtraction {
                dec_result = if dec_lhs < dec_rhs {
                    dec_lhs + 99 - dec_rhs + self.p.c as u8
                } else {
                    dec_lhs - dec_rhs - 1 + self.p.c as u8
                };
                self.p.c = lhs >= rhs;
                rhs = !rhs; // For V to work as "intended"
            } else {
                dec_result = dec_lhs + dec_rhs + self.p.c as u8;
                self.p.c = dec_result > 99;
            }
            result8 = dec_to_bcd8(dec_result % 100);
        } else {
            // Binary arithmetic
            if subtraction {
                rhs = !rhs;
            }
            let result16 = (lhs as u16) + (rhs as u16) + (self.p.c as u16);
            self.p.c = result16 > 0xFF;
            result8 = result16 as u8;
        }
        self.p.n = result8 > 0x7F;
        self.p.z = result8 == 0;
        self.p.v = (lhs < 0x80 && rhs < 0x80 && result8 > 0x7F)
            || (lhs > 0x7F && rhs > 0x7F && result8 < 0x80);
        result8
    }

    fn add_sub16(&mut self, lhs: u16, mut rhs: u16, subtraction: bool) -> u16 {
        let result16: u16;
        if self.p.d {
            // BCD arithmetic
            // TODO: Lower level arithmetic
            let dec_lhs = bcd_to_dec16(lhs);
            let dec_rhs = bcd_to_dec16(rhs);
            let dec_result: u16;
            if subtraction {
                dec_result = if dec_lhs < dec_rhs {
                    dec_lhs + 9999 - dec_rhs + self.p.c as u16
                } else {
                    dec_lhs - dec_rhs - 1 + self.p.c as u16
                };
                self.p.c = lhs >= rhs;
                rhs = !rhs; // For V to work as "intended"
            } else {
                dec_result = dec_lhs + dec_rhs + self.p.c as u16;
                self.p.c = dec_result > 9999;
            }
            result16 = dec_to_bcd16(dec_result % 10000);
        } else {
            // Binary arithmetic
            if subtraction {
                rhs = !rhs;
            }
            let result32 = (lhs as u32) + (rhs as u32) + (self.p.c as u32);
            self.p.c = result32 > 0xFFFF;
            result16 = result32 as u16;
        }
        self.p.n = result16 > 0x7FFF;
        self.p.z = result16 == 0;
        self.p.v = (lhs < 0x8000 && rhs < 0x8000 && result16 > 0x7FFF)
            || (lhs > 0x7FFF && rhs > 0x7FFF && result16 < 0x8000);
        result16
    }

    fn compare8(&mut self, lhs: u8, rhs: u8) {
        let result: u16 = lhs as u16 + !rhs as u16 + 1;
        self.p.n = result as u8 > 0x7F;
        self.p.z = result as u8 == 0;
        self.p.c = result > 0xFF;
    }

    fn compare16(&mut self, lhs: u16, rhs: u16) {
        let result: u32 = lhs as u32 + !rhs as u32 + 1;
        self.p.n = result as u16 > 0x7FFF;
        self.p.z = result as u16 == 0;
        self.p.c = result > 0xFFFF;
    }

    fn arithmetic_shift_left8(&mut self, data: u8) -> u8 {
        self.p.c = data & 0x80 > 0;
        let result = data << 1;
        self.p.n = result > 0x7F;
        self.p.z = result == 0;
        result
    }

    fn arithmetic_shift_left16(&mut self, data: u16) -> u16 {
        self.p.c = data & 0x8000 > 0;
        let result = data << 1;
        self.p.n = result > 0x7FFF;
        self.p.z = result == 0;
        result
    }

    fn logical_shift_right8(&mut self, data: u8) -> u8 {
        self.p.c = data & 0x01 > 0;
        let result = data >> 1;
        self.p.n = result > 0x7F;
        self.p.z = result == 0;
        result
    }

    fn logical_shift_right16(&mut self, data: u16) -> u16 {
        self.p.c = data & 0x0001 > 0;
        let result = data >> 1;
        self.p.n = result > 0x7FFF;
        self.p.z = result == 0;
        result
    }

    fn rotate_left8(&mut self, data: u8) -> u8 {
        let c = data & 0x80 > 0;
        let result = (data << 1) | self.p.c as u8;
        self.p.c = c;
        self.p.n = result > 0x7F;
        self.p.z = result == 0;
        result
    }

    fn rotate_left16(&mut self, data: u16) -> u16 {
        let c = data & 0x8000 > 0;
        let result = (data << 1) | self.p.c as u16;
        self.p.c = c;
        self.p.n = result > 0x7FFF;
        self.p.z = result == 0;
        result
    }

    fn rotate_right8(&mut self, data: u8) -> u8 {
        let c = data & 0x01 > 0;
        let result = (data >> 1) | ((self.p.c as u8) << 7);
        self.p.c = c;
        self.p.n = result > 0x7F;
        self.p.z = result == 0;
        result
    }

    fn rotate_right16(&mut self, data: u16) -> u16 {
        let c = data & 0x0001 > 0;
        let result = (data >> 1) | ((self.p.c as u16) << 15);
        self.p.c = c;
        self.p.n = result > 0x7FFF;
        self.p.z = result == 0;
        result
    }

    // Instructions
    fn op_adc(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let acc8 = self.a as u8;
            let data = mmap::cpu_read8(abus, data_addr.0);
            self.a = (self.a & 0xFF00) | (self.add_sub8(acc8, data, false) as u16);
        } else {
            // 16-bit accumulator
            let acc16 = self.a;
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.a = self.add_sub16(acc16, data, false);
        }
    }

    fn op_sbc(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let acc8 = self.a as u8;
            let data = mmap::cpu_read8(abus, data_addr.0);
            self.a = (self.a & 0xFF00) | (self.add_sub8(acc8, data, true) as u16);
        } else {
            // 16-bit accumulator
            let acc16 = self.a;
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.a = self.add_sub16(acc16, data, true);
        }
    }

    fn op_cmp(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let acc8 = self.a as u8;
            let data = mmap::cpu_read8(abus, data_addr.0);
            self.compare8(acc8, data);
        } else {
            // 16-bit accumulator
            let acc16 = self.a;
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.compare16(acc16, data);
        }
    }

    fn op_cpx(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            // 8-bit accumulator
            let x8 = self.x as u8;
            let data = mmap::cpu_read8(abus, data_addr.0);
            self.compare8(x8, data);
        } else {
            // 16-bit accumulator
            let x16 = self.x;
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.compare16(x16, data);
        }
    }

    fn op_cpy(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            // 8-bit accumulator
            let y8 = self.y as u8;
            let data = mmap::cpu_read8(abus, data_addr.0);
            self.compare8(y8, data);
        } else {
            // 16-bit accumulator
            let y16 = self.y;
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.compare16(y16, data);
        }
    }

    fn op_dec(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0).wrapping_sub(1);
            self.p.n = data > 0x7F;
            self.p.z = data == 0;
            mmap::cpu_write8(abus, data, data_addr.0);
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => {
                    data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0).wrapping_sub(1);
                    mmap::bank_wrapping_cpu_write16(abus, data, data_addr.0);
                }
                WrappingMode::AddrSpace => {
                    data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0).wrapping_sub(1);
                    mmap::addr_wrapping_cpu_write16(abus, data, data_addr.0);
                }
                WrappingMode::Page => unreachable!(),
            }
            self.p.n = data > 0x7FFF;
            self.p.z = data == 0;
        }
    }

    fn op_inc(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0).wrapping_add(1);
            self.p.n = data > 0x7F;
            self.p.z = data == 0;
            mmap::cpu_write8(abus, data, data_addr.0);
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => {
                    data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0).wrapping_add(1);
                    mmap::bank_wrapping_cpu_write16(abus, data, data_addr.0);
                }
                WrappingMode::AddrSpace => {
                    data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0).wrapping_add(1);
                    mmap::addr_wrapping_cpu_write16(abus, data, data_addr.0);
                }
                WrappingMode::Page => unreachable!(),
            }
            self.p.n = data > 0x7FFF;
            self.p.z = data == 0;
        }
    }

    fn op_and(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0);
            self.a = (self.a & 0xFF00) | (self.a as u8 & data) as u16;
            self.p.n = self.a as u8 > 0x7F;
            self.p.z = self.a as u8 == 0;
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.a = self.a & data;
            self.p.n = self.a > 0x7FFF;
            self.p.z = self.a == 0;
        }
    }

    fn op_eor(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0);
            self.a = (self.a & 0xFF00) | (self.a as u8 ^ data) as u16;
            self.p.n = self.a as u8 > 0x7F;
            self.p.z = self.a as u8 == 0;
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.a = self.a ^ data;
            self.p.n = self.a > 0x7FFF;
            self.p.z = self.a == 0;
        }
    }

    fn op_ora(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0);
            self.a = (self.a & 0xFF00) | (self.a as u8 | data) as u16;
            self.p.n = self.a as u8 > 0x7F;
            self.p.z = self.a as u8 == 0;
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.a = self.a | data;
            self.p.n = self.a > 0x7FFF;
            self.p.z = self.a == 0;
        }
    }

    fn op_bit(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0);
            let result = self.a as u8 & data;
            self.p.n = data > 0x7F;
            self.p.v = data & 0x40 > 0;
            self.p.z = result == 0;
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            let result = self.a & data;
            self.p.n = data > 0x7FFF;
            self.p.v = data & 0x4000 > 0;
            self.p.z = result == 0;
        }
    }

    fn op_trb(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0);
            mmap::cpu_write8(abus, data & !(self.a as u8), data_addr.0);
            self.p.z = self.a as u8 & data == 0;
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => {
                    data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::bank_wrapping_cpu_write16(abus, data & !self.a, data_addr.0)
                }
                WrappingMode::AddrSpace => {
                    data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::addr_wrapping_cpu_write16(abus, data & !self.a, data_addr.0)
                }
                WrappingMode::Page => unreachable!(),
            }
            self.p.z = self.a & data == 0;
        }
    }

    fn op_tsb(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0);
            mmap::cpu_write8(abus, data | (self.a as u8), data_addr.0);
            self.p.z = self.a as u8 & data == 0;
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => {
                    data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::bank_wrapping_cpu_write16(abus, data | self.a, data_addr.0)
                }
                WrappingMode::AddrSpace => {
                    data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::addr_wrapping_cpu_write16(abus, data | self.a, data_addr.0)
                }
                WrappingMode::Page => unreachable!(),
            }
            self.p.z = self.a & data == 0;
        }
    }

    fn op_asl(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0);
            mmap::cpu_write8(abus, self.arithmetic_shift_left8(data), data_addr.0);
        } else {
            // 16-bit accumulator
            match data_addr.1 {
                WrappingMode::Bank => {
                    let data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::bank_wrapping_cpu_write16(
                        abus,
                        self.arithmetic_shift_left16(data),
                        data_addr.0,
                    )
                }
                WrappingMode::AddrSpace => {
                    let data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::addr_wrapping_cpu_write16(
                        abus,
                        self.arithmetic_shift_left16(data),
                        data_addr.0,
                    )
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    fn op_lsr(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0);
            mmap::cpu_write8(abus, self.logical_shift_right8(data), data_addr.0);
        } else {
            // 16-bit accumulator
            match data_addr.1 {
                WrappingMode::Bank => {
                    let data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::bank_wrapping_cpu_write16(
                        abus,
                        self.logical_shift_right16(data),
                        data_addr.0,
                    )
                }
                WrappingMode::AddrSpace => {
                    let data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::addr_wrapping_cpu_write16(
                        abus,
                        self.logical_shift_right16(data),
                        data_addr.0,
                    )
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    fn op_rol(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0);
            mmap::cpu_write8(abus, self.rotate_left8(data), data_addr.0);
        } else {
            // 16-bit accumulator
            match data_addr.1 {
                WrappingMode::Bank => {
                    let data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::bank_wrapping_cpu_write16(abus, self.rotate_left16(data), data_addr.0)
                }
                WrappingMode::AddrSpace => {
                    let data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::addr_wrapping_cpu_write16(abus, self.rotate_left16(data), data_addr.0)
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    fn op_ror(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = mmap::cpu_read8(abus, data_addr.0);
            mmap::cpu_write8(abus, self.rotate_right8(data), data_addr.0);
        } else {
            // 16-bit accumulator
            match data_addr.1 {
                WrappingMode::Bank => {
                    let data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::bank_wrapping_cpu_write16(abus, self.rotate_right16(data), data_addr.0)
                }
                WrappingMode::AddrSpace => {
                    let data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0);
                    mmap::addr_wrapping_cpu_write16(abus, self.rotate_right16(data), data_addr.0)
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    fn op_lda(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            let data = mmap::cpu_read8(abus, data_addr.0) as u16;
            self.a = (self.a & 0xFF00) | data;
            self.p.n = data > 0x7F;
            self.p.z = data == 0;
        } else {
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.a = data;
            self.p.n = data > 0x7FFF;
            self.p.z = data == 0;
        }
    }

    fn op_ldx(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            let data = mmap::cpu_read8(abus, data_addr.0) as u16;
            self.x = (self.x & 0xFF00) | data;
            self.p.n = data > 0x7F;
            self.p.z = data == 0;
        } else {
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.x = data;
            self.p.n = data > 0x7FFF;
            self.p.z = data == 0;
        }
    }

    fn op_ldy(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            let data = mmap::cpu_read8(abus, data_addr.0) as u16;
            self.y = (self.y & 0xFF00) | data;
            self.p.n = data > 0x7F;
            self.p.z = data == 0;
        } else {
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => data = mmap::bank_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::AddrSpace => data = mmap::addr_wrapping_cpu_read16(abus, data_addr.0),
                WrappingMode::Page => unreachable!(),
            }
            self.y = data;
            self.p.n = data > 0x7FFF;
            self.p.z = data == 0;
        }
    }

    fn op_sta(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            mmap::cpu_write8(abus, self.a as u8, data_addr.0);
        } else {
            match data_addr.1 {
                WrappingMode::Bank => mmap::bank_wrapping_cpu_write16(abus, self.a, data_addr.0),
                WrappingMode::AddrSpace => {
                    mmap::addr_wrapping_cpu_write16(abus, self.a, data_addr.0)
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    fn op_stx(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            mmap::cpu_write8(abus, self.x as u8, data_addr.0);
        } else {
            match data_addr.1 {
                WrappingMode::Bank => mmap::bank_wrapping_cpu_write16(abus, self.x, data_addr.0),
                WrappingMode::AddrSpace => {
                    mmap::addr_wrapping_cpu_write16(abus, self.x, data_addr.0)
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    fn op_sty(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            mmap::cpu_write8(abus, self.y as u8, data_addr.0);
        } else {
            match data_addr.1 {
                WrappingMode::Bank => mmap::bank_wrapping_cpu_write16(abus, self.y, data_addr.0),
                WrappingMode::AddrSpace => {
                    mmap::addr_wrapping_cpu_write16(abus, self.y, data_addr.0)
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    fn op_stz(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            mmap::cpu_write8(abus, 0x00, data_addr.0);
        } else {
            match data_addr.1 {
                WrappingMode::Bank => mmap::bank_wrapping_cpu_write16(abus, 0x0000, data_addr.0),
                WrappingMode::AddrSpace => {
                    mmap::addr_wrapping_cpu_write16(abus, 0x0000, data_addr.0)
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    fn op_mvn(&mut self, data_addrs: &(u32, u32), abus: &mut ABus) {
        let data = mmap::cpu_read8(abus, data_addrs.0);
        mmap::cpu_write8(abus, data, data_addrs.1);
        self.a = self.a.wrapping_sub(1);
        self.x = if self.p.x {
            (self.x as u8).wrapping_add(1) as u16
        } else {
            self.x.wrapping_add(1)
        };
        self.y = if self.p.x {
            (self.y as u8).wrapping_add(1) as u16
        } else {
            self.y.wrapping_add(1)
        };
        self.db = (data_addrs.1 >> 16) as u8;
        if self.a == 0xFFFF {
            self.pc = self.pc.wrapping_add(3);
        }
    }

    fn op_mvp(&mut self, data_addrs: &(u32, u32), abus: &mut ABus) {
        let data = mmap::cpu_read8(abus, data_addrs.0);
        mmap::cpu_write8(abus, data, data_addrs.1);
        self.a = self.a.wrapping_sub(1);
        self.x = if self.p.x {
            (self.x as u8).wrapping_sub(1) as u16
        } else {
            self.x.wrapping_sub(1)
        };
        self.y = if self.p.x {
            (self.y as u8).wrapping_sub(1) as u16
        } else {
            self.y.wrapping_sub(1)
        };
        self.db = (data_addrs.1 >> 16) as u8;
        if self.a == 0xFFFF {
            self.pc = self.pc.wrapping_add(3);
        }
    }
}

#[inline(always)]
fn addr_8_16(bank: u8, byte: u16) -> u32 { ((bank as u32) << 16) | byte as u32 }

#[inline(always)]
fn addr_8_8_8(bank: u8, page: u8, byte: u8) -> u32 {
    ((bank as u32) << 16) | ((page as u32) << 8) | byte as u32
}

fn dec_to_bcd8(value: u8) -> u8 {
    if value > 99 {
        panic!("{} too large for 8bit BCD", value);
    }
    ((value / 10) << 4) | (value % 10)
}

fn dec_to_bcd16(value: u16) -> u16 {
    if value > 9999 {
        panic!("{} too large for 16bit BCD", value);
    }
    ((value / 1000) << 12) | (((value % 1000) / 100) << 8) | (((value % 100) / 10) << 4)
        | (value % 10)
}

fn bcd_to_dec8(value: u8) -> u8 {
    let ll = value & 0x000F;
    let hh = (value >> 4) & 0x000F;
    if hh > 0x9 || ll > 0x9 {
        panic!("{:02X} is not a valid 8bit BCD", value);
    }
    hh * 10 + ll
}

fn bcd_to_dec16(value: u16) -> u16 {
    let ll = value & 0x000F;
    let ml = (value >> 4) & 0x000F;
    let mh = (value >> 8) & 0x000F;
    let hh = (value >> 12) & 0x000F;
    if hh > 0x9 || mh > 0x9 || ml > 0x9 || ll > 0x9 {
        panic!("{:04X} is not a valid 16bit BCD", value);
    }
    hh * 1000 + mh * 100 + ml * 10 + ll
}

// Interrupt vectors
const COP16: u32 = 0x00FFE4;
const BRK16: u32 = 0x00FFE6;
const ABORT16: u32 = 0x00FFE8;
const NMI16: u32 = 0x00FFEA;
const IRQ16: u32 = 0x00FFEE;
const COP8: u32 = 0x00FFF4;
const ABORT8: u32 = 0x00FFF8;
const NMI8: u32 = 0x00FFFA;
const RESET8: u32 = 0x00FFFC;
const IRQBRK8: u32 = 0x00FFFE;

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
pub enum WrappingMode {
    Page,
    Bank,
    AddrSpace,
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
    c: bool,
}

impl StatusReg {
    pub fn new() -> StatusReg {
        StatusReg {
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

    fn set_flags(&mut self, mask: u8) {
        if mask & P_N > 0 {
            self.n = true;
        }
        if mask & P_V > 0 {
            self.v = true;
        }
        if mask & P_M > 0 {
            self.m = true;
        }
        if mask & P_X > 0 {
            self.x = true;
        }
        if mask & P_D > 0 {
            self.d = true;
        }
        if mask & P_I > 0 {
            self.i = true;
        }
        if mask & P_Z > 0 {
            self.z = true;
        }
        if mask & P_C > 0 {
            self.c = true;
        }
    }

    fn clear_flags(&mut self, mask: u8) {
        if mask & P_N > 0 {
            self.n = false;
        }
        if mask & P_V > 0 {
            self.v = false;
        }
        if mask & P_M > 0 {
            self.m = false;
        }
        if mask & P_X > 0 {
            self.x = false;
        }
        if mask & P_D > 0 {
            self.d = false;
        }
        if mask & P_I > 0 {
            self.i = false;
        }
        if mask & P_Z > 0 {
            self.z = false;
        }
        if mask & P_C > 0 {
            self.c = false;
        }
    }

    fn value(&self) -> u8 {
        let mut value: u8 = 0b0000_0000;
        if self.n {
            value |= P_N;
        }
        if self.v {
            value |= P_V;
        }
        if self.m {
            value |= P_M;
        }
        if self.x {
            value |= P_X;
        }
        if self.d {
            value |= P_D;
        }
        if self.i {
            value |= P_I;
        }
        if self.z {
            value |= P_Z;
        }
        if self.c {
            value |= P_C;
        }
        value
    }

    fn set_value(&mut self, value: u8) {
        self.n = value & P_N > 0;
        self.v = value & P_V > 0;
        self.m = value & P_M > 0;
        self.x = value & P_X > 0;
        self.d = value & P_D > 0;
        self.i = value & P_I > 0;
        self.z = value & P_Z > 0;
        self.c = value & P_C > 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Stack
    #[test]
    fn stack_push() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.s = 0x01FF;
        cpu.set_emumode();
        // Regular emumode pushes
        cpu.push8(0x12, &mut abus);
        assert_eq!(0x01FE, cpu.s);
        assert_eq!(0x12, mmap::cpu_read8(&mut abus, 0x0001FF));
        cpu.push16(0x2345, &mut abus);
        assert_eq!(0x01FC, cpu.s);
        assert_eq!(0x2345, mmap::page_wrapping_cpu_read16(&mut abus, 0x0001FD));
        cpu.push24(0x345678, &mut abus);
        assert_eq!(0x01F9, cpu.s);
        assert_eq!(
            0x345678,
            mmap::page_wrapping_cpu_read24(&mut abus, 0x0001FA)
        );
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x0001FA);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x0001FD);
        // Wrapping emumode pushes
        cpu.s = 0x0100;
        cpu.push8(0x12, &mut abus);
        cpu.push8(0x34, &mut abus);
        assert_eq!(0x1234, mmap::page_wrapping_cpu_read16(&mut abus, 0x0001FF));
        cpu.s = 0x0100;
        cpu.push16(0x2345, &mut abus);
        assert_eq!(0x2345, mmap::page_wrapping_cpu_read16(&mut abus, 0x0001FF));
        cpu.s = 0x0100;
        cpu.push24(0x345678, &mut abus);
        assert_eq!(
            0x345678,
            mmap::page_wrapping_cpu_read24(&mut abus, 0x0001FE)
        );
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x0001FE);
        // Regular pushes
        cpu.s = 0x01FF;
        cpu.e = false;
        cpu.push8(0x12, &mut abus);
        assert_eq!(0x01FE, cpu.s);
        assert_eq!(0x12, mmap::cpu_read8(&mut abus, 0x0001FF));
        cpu.push16(0x2345, &mut abus);
        assert_eq!(0x01FC, cpu.s);
        assert_eq!(0x2345, mmap::bank_wrapping_cpu_read16(&mut abus, 0x0001FD));
        cpu.push24(0x345678, &mut abus);
        assert_eq!(0x01F9, cpu.s);
        assert_eq!(
            0x345678,
            mmap::bank_wrapping_cpu_read24(&mut abus, 0x0001FA)
        );
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x0001FA);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x0001FD);
        // Wrapping pushes
        cpu.s = 0x0000;
        cpu.push8(0x12, &mut abus);
        cpu.push8(0x34, &mut abus);
        assert_eq!(0xFFFE, cpu.s);
        assert_eq!(0x1234, mmap::bank_wrapping_cpu_read16(&mut abus, 0x00FFFF));
        cpu.s = 0x0000;
        cpu.push16(0x2345, &mut abus);
        assert_eq!(0xFFFE, cpu.s);
        assert_eq!(0x2345, mmap::bank_wrapping_cpu_read16(&mut abus, 0x00FFFF));
        cpu.s = 0x0000;
        cpu.push24(0x345678, &mut abus);
        assert_eq!(0xFFFD, cpu.s);
        assert_eq!(
            0x345678,
            mmap::bank_wrapping_cpu_read24(&mut abus, 0x00FFFE)
        );
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x0FFFE);
    }

    #[test]
    fn stack_pull() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.s = 0x01FC;
        cpu.set_emumode();
        // Regular emumode pulls
        mmap::page_wrapping_cpu_write24(&mut abus, 0x123456, 0x0001FD);
        assert_eq!(0x56, cpu.pull8(&mut abus));
        assert_eq!(0x01FD, cpu.s);
        assert_eq!(0x1234, cpu.pull16(&mut abus));
        assert_eq!(0x01FF, cpu.s);
        cpu.s = 0x01FC;
        assert_eq!(0x123456, cpu.pull24(&mut abus));
        assert_eq!(0x01FF, cpu.s);
        mmap::page_wrapping_cpu_write24(&mut abus, 0x000000, 0x0001FD);
        // Wrapping emumode pulls
        mmap::page_wrapping_cpu_write24(&mut abus, 0x123456, 0x0001FF);
        cpu.s = 0x01FF;
        assert_eq!(0x34, cpu.pull8(&mut abus));
        assert_eq!(0x0100, cpu.s);
        cpu.s = 0x01FE;
        assert_eq!(0x3456, cpu.pull16(&mut abus));
        assert_eq!(0x0100, cpu.s);
        cpu.s = 0x01FE;
        assert_eq!(0x123456, cpu.pull24(&mut abus));
        assert_eq!(0x0101, cpu.s);
        mmap::page_wrapping_cpu_write24(&mut abus, 0x000000, 0x0001FF);
        // Regular pulls
        cpu.s = 0x01FC;
        cpu.e = false;
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x123456, 0x0001FD);
        assert_eq!(0x56, cpu.pull8(&mut abus));
        assert_eq!(0x01FD, cpu.s);
        assert_eq!(0x1234, cpu.pull16(&mut abus));
        assert_eq!(0x01FF, cpu.s);
        cpu.s = 0x01FC;
        assert_eq!(0x123456, cpu.pull24(&mut abus));
        assert_eq!(0x01FF, cpu.s);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x0001FD);
        // Wrapping pulls
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x123456, 0x00FFFF);
        cpu.s = 0xFFFF;
        assert_eq!(0x34, cpu.pull8(&mut abus));
        assert_eq!(0x0000, cpu.s);
        cpu.s = 0xFFFE;
        assert_eq!(0x3456, cpu.pull16(&mut abus));
        assert_eq!(0x0000, cpu.s);
        cpu.s = 0xFFFE;
        assert_eq!(0x123456, cpu.pull24(&mut abus));
        assert_eq!(0x0001, cpu.s);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x00FFFF);
    }

    // Addressing modes
    #[test]
    fn addr_abs() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.pb = 0x1A;
        cpu.db = 0x1B;
        cpu.pc = 0x0010;
        cpu.x = 0x5678;
        cpu.y = 0xABCD;
        // Absolute
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1234, 0x000011);
        assert_eq!(
            (0x1B1234, WrappingMode::AddrSpace),
            cpu.abs(cpu.pc as u32, &mut abus)
        );
        // Absolute,X and Absolute,Y
        assert_eq!(
            (0x1B68AC, WrappingMode::AddrSpace),
            cpu.abs_x(cpu.pc as u32, &mut abus)
        );
        assert_eq!(
            (0x1BBE01, WrappingMode::AddrSpace),
            cpu.abs_y(cpu.pc as u32, &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x000011);
        // Wrapping
        cpu.db = 0xFF;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x000011);
        assert_eq!(
            (0x000245, WrappingMode::AddrSpace),
            cpu.abs_x(cpu.pc as u32, &mut abus)
        );
        assert_eq!(
            (0x00579A, WrappingMode::AddrSpace),
            cpu.abs_y(cpu.pc as u32, &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x000011);
        // (Absolute) and [Absolute]
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x01FF, 0x000011);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0xABCDEF, 0x0001FF);
        assert_eq!(
            (0x1ACDEF, WrappingMode::Bank),
            cpu.abs_ptr16(cpu.pc as u32, &mut abus)
        );
        assert_eq!(
            (0xABCDEF, WrappingMode::Bank),
            cpu.abs_ptr24(cpu.pc as u32, &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x000011);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x0001FF);
        // Wrapping
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFF, 0x000011);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0xFEDCBA, 0x00FFFF);
        assert_eq!(
            (0x1ADCBA, WrappingMode::Bank),
            cpu.abs_ptr16(cpu.pc as u32, &mut abus)
        );
        assert_eq!(
            (0xFEDCBA, WrappingMode::Bank),
            cpu.abs_ptr24(cpu.pc as u32, &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x000011);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x00FFFF);
        // (Absolute,X)
        cpu.pb = 0x7E;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x01FF, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x7E5877);
        assert_eq!(
            (0x7EABCD, WrappingMode::Bank),
            cpu.abs_x_ptr16(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x7E5877);
        // Wrapping
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xA987, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1234, 0x7EFFFF);
        assert_eq!(
            (0x7E1234, WrappingMode::Bank),
            cpu.abs_x_ptr16(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x7EFFFF);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xA989, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x5678, 0x7E0001);
        assert_eq!(
            (0x7E5678, WrappingMode::Bank),
            cpu.abs_x_ptr16(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x7E0001);
    }

    #[test]
    fn addr_dir() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.pb = 0x7E;
        cpu.db = 0x7F;
        cpu.pc = 0x0010;
        cpu.d = 0x1234;
        cpu.set_emumode();
        // Direct
        mmap::cpu_write8(&mut abus, 0x12, 0x7E0011);
        assert_eq!(
            (0x001246, WrappingMode::Bank),
            cpu.dir(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        // Wrapping
        cpu.d = 0xFFE0;
        mmap::cpu_write8(&mut abus, 0xFF, 0x7E0011);
        assert_eq!(
            (0x0000DF, WrappingMode::Bank),
            cpu.dir(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        // Direct,X and Direct,Y
        // Standard behavior
        cpu.d = 0x1234;
        cpu.x = 0x5678;
        cpu.y = 0xABCD;
        mmap::cpu_write8(&mut abus, 0x12, 0x7E0011);
        assert_eq!(
            (0x0068BE, WrappingMode::Bank),
            cpu.dir_x(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        assert_eq!(
            (0x00BE13, WrappingMode::Bank),
            cpu.dir_y(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        // Wrapping
        cpu.d = 0xCDEF;
        assert_eq!(
            (0x002479, WrappingMode::Bank),
            cpu.dir_x(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        assert_eq!(
            (0x0079CE, WrappingMode::Bank),
            cpu.dir_y(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        // Emumode and DL is $00
        cpu.d = 0x1200;
        cpu.x = 0x0034;
        cpu.y = 0x00AB;
        mmap::cpu_write8(&mut abus, 0x12, 0x7E0011);
        assert_eq!(
            (0x001246, WrappingMode::Page),
            cpu.dir_x(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        assert_eq!(
            (0x0012BD, WrappingMode::Page),
            cpu.dir_y(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        // Wrapping
        mmap::cpu_write8(&mut abus, 0xDE, 0x7E0011);
        assert_eq!(
            (0x001212, WrappingMode::Page),
            cpu.dir_x(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        assert_eq!(
            (0x001289, WrappingMode::Page),
            cpu.dir_y(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        // (Direct)
        cpu.d = 0x1234;
        mmap::cpu_write8(&mut abus, 0x56, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x00128A);
        assert_eq!(
            (0x7FABCD, WrappingMode::AddrSpace),
            cpu.dir_ptr16(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00128A);
        // Wrapping
        cpu.d = 0xFFA9;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x00FFFF);
        assert_eq!(
            (0x7FABCD, WrappingMode::AddrSpace),
            cpu.dir_ptr16(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00FFFF);
        // [Direct]
        cpu.d = 0x1234;
        mmap::cpu_write8(&mut abus, 0x56, 0x7E0011);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0xABCDEF, 0x00128A);
        assert_eq!(
            (0xABCDEF, WrappingMode::AddrSpace),
            cpu.dir_ptr24(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x00128A);
        // Wrapping
        cpu.d = 0xFFA9;
        mmap::bank_wrapping_cpu_write24(&mut abus, 0xABCDEF, 0x00FFFF);
        assert_eq!(
            (0xABCDEF, WrappingMode::AddrSpace),
            cpu.dir_ptr24(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x00FFFF);
        // (Direct,X)
        // Standard behavior
        cpu.d = 0x0123;
        cpu.x = 0x1234;
        mmap::cpu_write8(&mut abus, 0x12, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x001369);
        assert_eq!(
            (0x7FABCD, WrappingMode::AddrSpace),
            cpu.dir_x_ptr16(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x001369);
        // Wrapping
        cpu.d = 0xABCD;
        cpu.x = 0x5420;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x00FFFF);
        assert_eq!(
            (0x7FABCD, WrappingMode::AddrSpace),
            cpu.dir_x_ptr16(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00FFFF);
        // Emumode and DL is $00
        cpu.d = 0x1200;
        cpu.x = 0x0034;
        mmap::cpu_write8(&mut abus, 0x12, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x001246);
        assert_eq!(
            (0x7FABCD, WrappingMode::AddrSpace),
            cpu.dir_x_ptr16(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x001246);
        // Wrapping
        mmap::cpu_write8(&mut abus, 0xCB, 0x7E0011);
        mmap::page_wrapping_cpu_write16(&mut abus, 0xABCD, 0x0012FF);
        assert_eq!(
            (0x7FABCD, WrappingMode::AddrSpace),
            cpu.dir_x_ptr16(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::page_wrapping_cpu_write16(&mut abus, 0x0000, 0x0012FF);
        // (Direct),Y
        cpu.d = 0x1234;
        cpu.y = 0x2345;
        mmap::cpu_write8(&mut abus, 0x56, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x00128A);
        assert_eq!(
            (0x7FCF12, WrappingMode::AddrSpace),
            cpu.dir_ptr16_y(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00128A);
        // Wrapping on pointer and +Y
        cpu.db = 0xFF;
        cpu.d = 0xFF34;
        cpu.y = 0x5678;
        mmap::cpu_write8(&mut abus, 0xCB, 0x7E0011);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x00FFFF);
        assert_eq!(
            (0x000245, WrappingMode::AddrSpace),
            cpu.dir_ptr16_y(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00FFFF);
        // Emumode and DL is $00
        cpu.db = 0xFF;
        cpu.d = 0x1200;
        cpu.y = 0x5678;
        mmap::cpu_write8(&mut abus, 0xFF, 0x7E0011);
        mmap::page_wrapping_cpu_write16(&mut abus, 0xABCD, 0x0012FF);
        assert_eq!(
            (0x000245, WrappingMode::AddrSpace),
            cpu.dir_ptr16_y(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::page_wrapping_cpu_write16(&mut abus, 0x0000, 0x0012FF);
        // [Direct],Y
        cpu.d = 0x1234;
        cpu.y = 0x2345;
        mmap::cpu_write8(&mut abus, 0x56, 0x7E0011);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0xABCDEF, 0x00128A);
        assert_eq!(
            (0xABF134, WrappingMode::AddrSpace),
            cpu.dir_ptr24_y(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x00128A);
        // Wrapping on pointer and +Y
        cpu.db = 0xFF;
        cpu.d = 0xFF34;
        cpu.y = 0x5678;
        mmap::cpu_write8(&mut abus, 0xCB, 0x7E0011);
        mmap::bank_wrapping_cpu_write24(&mut abus, 0xFFABCD, 0x00FFFF);
        assert_eq!(
            (0x000245, WrappingMode::AddrSpace),
            cpu.dir_ptr24_y(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x00FFFF);
    }

    #[test]
    fn addr_imm() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
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
        let mut cpu = W65C816S::new(&mut abus);
        cpu.pb = 0x00;
        cpu.pc = 0x0000;
        // Long
        mmap::bank_wrapping_cpu_write24(&mut abus, 0xABCDEF, 0x000001);
        assert_eq!(
            (0xABCDEF, WrappingMode::AddrSpace),
            cpu.long(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        // Long, X
        cpu.x = 0x1234;
        mmap::bank_wrapping_cpu_write24(&mut abus, 0xABCDEF, 0x000001);
        assert_eq!(
            (0xABE023, WrappingMode::AddrSpace),
            cpu.long_x(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        // Wrapping
        cpu.x = 0x5678;
        mmap::bank_wrapping_cpu_write24(&mut abus, 0xFFCDEF, 0x000001);
        assert_eq!(
            (0x002467, WrappingMode::AddrSpace),
            cpu.long_x(addr_8_16(cpu.pb, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write24(&mut abus, 0x000000, 0x000001);
    }

    #[test]
    fn addr_rel() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.pb = 0x45;
        cpu.pc = 0x0123;
        // Relative8
        mmap::cpu_write8(&mut abus, 0x01, 0x000124);
        assert_eq!(
            (0x450126, WrappingMode::Bank),
            cpu.rel8(addr_8_16(0x00, cpu.pc), &mut abus)
        );
        mmap::cpu_write8(&mut abus, 0x7F, 0x000124);
        assert_eq!(
            (0x4501A4, WrappingMode::Bank),
            cpu.rel8(addr_8_16(0x00, cpu.pc), &mut abus)
        );
        mmap::cpu_write8(&mut abus, 0xFF, 0x000124);
        assert_eq!(
            (0x450124, WrappingMode::Bank),
            cpu.rel8(addr_8_16(0x00, cpu.pc), &mut abus)
        );
        mmap::cpu_write8(&mut abus, 0x80, 0x000124);
        assert_eq!(
            (0x4500A5, WrappingMode::Bank),
            cpu.rel8(addr_8_16(0x00, cpu.pc), &mut abus)
        );
        mmap::cpu_write8(&mut abus, 0x00, 0x000124);
        // Wrapping
        cpu.pc = 0xFFEF;
        mmap::cpu_write8(&mut abus, 0x7F, 0x00FFF0);
        assert_eq!(
            (0x450070, WrappingMode::Bank),
            cpu.rel8(addr_8_16(0x00, cpu.pc), &mut abus)
        );
        mmap::cpu_write8(&mut abus, 0x00, 0x00FFF0);
        cpu.pc = 0x0010;
        mmap::cpu_write8(&mut abus, 0x80, 0x000011);
        assert_eq!(
            (0x45FF92, WrappingMode::Bank),
            cpu.rel8(addr_8_16(0x00, cpu.pc), &mut abus)
        );
        mmap::cpu_write8(&mut abus, 0x00, 0x000011);
        // Relative16
        cpu.pc = 0x0123;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x4567, 0x000124);
        assert_eq!(
            (0x45468D, WrappingMode::Bank),
            cpu.rel16(addr_8_16(0x00, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x000124);
        // Wrapping
        cpu.pc = 0xABCD;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x6789, 0x00ABCE);
        assert_eq!(
            (0x451359, WrappingMode::Bank),
            cpu.rel16(addr_8_16(0x00, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00ABCE);
    }

    #[test]
    fn addr_src_dest() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.pc = 0x0123;
        cpu.x = 0xABCD;
        cpu.y = 0xBCDE;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x4567, 0x000124);
        assert_eq!(
            (0x67ABCD, 0x45BCDE),
            cpu.src_dest(addr_8_16(0x00, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x000124);
        // Wrapping
        cpu.pc = 0xFFFE;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x4567, 0x00FFFF);
        assert_eq!(
            (0x67ABCD, 0x45BCDE),
            cpu.src_dest(addr_8_16(0x00, cpu.pc), &mut abus)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00FFFF);
    }

    #[test]
    fn addr_stack() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // Stack,S
        cpu.s = 0x0123;
        mmap::cpu_write8(&mut abus, 0x45, 0x000001);
        assert_eq!(
            (0x000168, WrappingMode::Bank),
            cpu.stack(0x000000, &mut abus)
        );
        mmap::cpu_write8(&mut abus, 0x00, 0x000001);
        // Wrapping
        cpu.s = 0xFFDE;
        mmap::cpu_write8(&mut abus, 0x45, 0x000001);
        assert_eq!(
            (0x000023, WrappingMode::Bank),
            cpu.stack(0x000000, &mut abus)
        );
        mmap::cpu_write8(&mut abus, 0x00, 0x000001);
        // (Stack,S),Y
        cpu.db = 0x9A;
        cpu.s = 0x0123;
        cpu.y = 0x1234;
        mmap::cpu_write8(&mut abus, 0x45, 0x000001);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x000168);
        assert_eq!(
            (0x9ABE01, WrappingMode::AddrSpace),
            cpu.stack_ptr16_y(0x000000, &mut abus)
        );
        mmap::cpu_write8(&mut abus, 0x00, 0x000001);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x000168);
        // Wrapping on pointer and +Y
        cpu.db = 0xFF;
        cpu.s = 0xFFBA;
        cpu.y = 0x5678;
        mmap::cpu_write8(&mut abus, 0x45, 0x000001);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xABCD, 0x00FFFF);
        assert_eq!(
            (0x000245, WrappingMode::AddrSpace),
            cpu.stack_ptr16_y(0x000000, &mut abus)
        );
        mmap::cpu_write8(&mut abus, 0x00, 0x000001);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00FFFF);
    }

    #[test]
    fn bcd_conversions() {
        assert_eq!(0x99, dec_to_bcd8(99));
        assert_eq!(0x9999, dec_to_bcd16(9999));
        assert_eq!(99, bcd_to_dec8(0x99));
        assert_eq!(9999, bcd_to_dec16(0x9999))
    }

    #[test]
    fn add_8() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.e = false;

        // Binary arithmetic
        cpu.p.clear_flags(0b1111_1111);
        assert_eq!(0x35, cpu.add_sub8(0x12, 0x23, false));
        // Wrapping, C set
        assert_eq!(false, cpu.p.c);
        assert_eq!(0x01, cpu.add_sub8(0x02, 0xFF, false));
        assert_eq!(true, cpu.p.c);
        // Add with carry, C cleared
        assert_eq!(0x01, cpu.add_sub8(0x00, 0x00, false));
        assert_eq!(false, cpu.p.c);
        // Z, C set
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x00, cpu.add_sub8(0x01, 0xFF, false));
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        // Z cleared
        assert_eq!(0x36, cpu.add_sub8(0x12, 0x23, false));
        assert_eq!(false, cpu.p.z);
        // N set
        assert_eq!(false, cpu.p.n);
        assert_eq!(0x9B, cpu.add_sub8(0x45, 0x56, false));
        assert_eq!(true, cpu.p.n);
        // N cleared
        assert_eq!(0x35, cpu.add_sub8(0x12, 0x23, false));
        assert_eq!(false, cpu.p.n);
        // V set (positive)
        assert_eq!(false, cpu.p.v);
        assert_eq!(0x9B, cpu.add_sub8(0x45, 0x56, false));
        assert_eq!(true, cpu.p.v);
        // V set (negative)
        cpu.p.v = false;
        assert_eq!(0x67, cpu.add_sub8(0xAB, 0xBC, false));
        assert_eq!(true, cpu.p.v);
        // V cleared (positive)
        assert_eq!(true, cpu.p.v);
        assert_eq!(0x36, cpu.add_sub8(0x12, 0x23, false));
        assert_eq!(false, cpu.p.v);
        // V cleared (negative)
        cpu.p.v = true;
        assert_eq!(0xEB, cpu.add_sub8(0xED, 0xFE, false));
        assert_eq!(false, cpu.p.v);

        // BCD arithmetic
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.d = true;
        assert_eq!(0x35, cpu.add_sub8(0x12, 0x23, false));
        // Wrapping, C set
        assert_eq!(false, cpu.p.c);
        assert_eq!(0x01, cpu.add_sub8(0x02, 0x99, false));
        assert_eq!(true, cpu.p.c);
        // Add with carry, C cleared
        assert_eq!(0x01, cpu.add_sub8(0x00, 0x00, false));
        assert_eq!(false, cpu.p.c);
        // Z, C set
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x00, cpu.add_sub8(0x01, 0x99, false));
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        // Z cleared
        assert_eq!(0x36, cpu.add_sub8(0x12, 0x23, false));
        assert_eq!(false, cpu.p.z);
        // N set
        assert_eq!(false, cpu.p.n);
        assert_eq!(0x97, cpu.add_sub8(0x65, 0x32, false));
        assert_eq!(true, cpu.p.n);
        // N cleared
        assert_eq!(0x35, cpu.add_sub8(0x12, 0x23, false));
        assert_eq!(false, cpu.p.n);
        // V set (positive)
        assert_eq!(false, cpu.p.v);
        assert_eq!(0x89, cpu.add_sub8(0x65, 0x24, false));
        assert_eq!(true, cpu.p.v);
        // V set (negative)
        cpu.p.v = false;
        assert_eq!(0x61, cpu.add_sub8(0x80, 0x81, false));
        assert_eq!(true, cpu.p.v);
        // V cleared (positive)
        assert_eq!(0x70, cpu.add_sub8(0x45, 0x24, false));
        assert_eq!(false, cpu.p.v);
        // V cleared (negative)
        cpu.p.v = true;
        assert_eq!(0x95, cpu.add_sub8(0x98, 0x97, false));
        assert_eq!(false, cpu.p.v);
    }

    #[test]
    fn add_16() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.e = false;

        // Binary arithmetic
        cpu.p.clear_flags(0b1111_1111);
        assert_eq!(0x3579, cpu.add_sub16(0x1234, 0x2345, false));
        // Wrapping, C set
        assert_eq!(false, cpu.p.c);
        assert_eq!(0x0001, cpu.add_sub16(0x0002, 0xFFFF, false));
        assert_eq!(true, cpu.p.c);
        // Add with carry, C cleared
        assert_eq!(0x0001, cpu.add_sub16(0x0000, 0x0000, false));
        assert_eq!(false, cpu.p.c);
        // Z, C set
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x0000, cpu.add_sub16(0x0001, 0xFFFF, false));
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        // Z cleared
        assert_eq!(0x357A, cpu.add_sub16(0x1234, 0x2345, false));
        assert_eq!(false, cpu.p.z);
        // N set
        assert_eq!(false, cpu.p.n);
        assert_eq!(0xBE01, cpu.add_sub16(0x5678, 0x6789, false));
        assert_eq!(true, cpu.p.n);
        // N cleared
        assert_eq!(0x3579, cpu.add_sub16(0x1234, 0x2345, false));
        assert_eq!(false, cpu.p.n);
        // V set (positive)
        assert_eq!(false, cpu.p.v);
        assert_eq!(0xBE01, cpu.add_sub16(0x5678, 0x6789, false));
        assert_eq!(true, cpu.p.v);
        // V set (negative)
        cpu.p.v = false;
        assert_eq!(0x68AB, cpu.add_sub16(0xABCD, 0xBCDE, false));
        assert_eq!(true, cpu.p.v);
        // V cleared (positive)
        assert_eq!(0x357A, cpu.add_sub16(0x1234, 0x2345, false));
        assert_eq!(false, cpu.p.v);
        // V cleared (negative)
        cpu.p.v = true;
        assert_eq!(0xECA7, cpu.add_sub16(0xEDCB, 0xFEDC, false));
        assert_eq!(false, cpu.p.v);

        // BCD arithmetic
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.d = true;
        assert_eq!(0x3579, cpu.add_sub16(0x1234, 0x2345, false));
        // Wrapping, C set
        assert_eq!(false, cpu.p.c);
        assert_eq!(0x0001, cpu.add_sub16(0x0002, 0x9999, false));
        assert_eq!(true, cpu.p.c);
        // Add with carry, C cleared
        assert_eq!(0x0001, cpu.add_sub16(0x0000, 0x0000, false));
        assert_eq!(false, cpu.p.c);
        // Z, C set
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x0000, cpu.add_sub16(0x0001, 0x9999, false));
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        // Z cleared
        assert_eq!(0x3580, cpu.add_sub16(0x1234, 0x2345, false));
        assert_eq!(false, cpu.p.z);
        // N set
        assert_eq!(false, cpu.p.n);
        assert_eq!(0x9753, cpu.add_sub16(0x6543, 0x3210, false));
        assert_eq!(true, cpu.p.n);
        // N cleared
        assert_eq!(0x3579, cpu.add_sub16(0x1234, 0x2345, false));
        assert_eq!(false, cpu.p.n);
        // V set (positive)
        assert_eq!(false, cpu.p.v);
        assert_eq!(0x9753, cpu.add_sub16(0x6543, 0x3210, false));
        assert_eq!(true, cpu.p.v);
        // V set (negative)
        cpu.p.v = false;
        assert_eq!(0x6283, cpu.add_sub16(0x8091, 0x8192, false));
        assert_eq!(true, cpu.p.v);
        // V cleared (positive)
        assert_eq!(0x6913, cpu.add_sub16(0x4567, 0x2345, false));
        assert_eq!(false, cpu.p.v);
        // V cleared (negative)
        cpu.p.v = true;
        assert_eq!(0x9617, cpu.add_sub16(0x9864, 0x9753, false));
        assert_eq!(false, cpu.p.v);
    }

    #[test]
    fn sub_8() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.e = false;

        // Binary arithmetic, C set as lhs >= rhs
        cpu.p.clear_flags(0b1111_1111);
        assert_eq!(0x98, cpu.add_sub8(0xFE, 0x65, true));
        assert_eq!(true, cpu.p.c);
        // Wrapping sub with carry, C cleared
        assert_eq!(0xEF, cpu.add_sub8(0x12, 0x23, true));
        assert_eq!(false, cpu.p.c);
        // Z set
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x00, cpu.add_sub8(0x12, 0x11, true));
        assert_eq!(true, cpu.p.z);
        // Z cleared
        assert_eq!(0xEF, cpu.add_sub8(0x12, 0x23, true));
        assert_eq!(false, cpu.p.z);
        // N set
        cpu.p.n = false;
        assert_eq!(0xEE, cpu.add_sub8(0x12, 0x23, true));
        assert_eq!(true, cpu.p.n);
        // N cleared
        assert_eq!(0x10, cpu.add_sub8(0x23, 0x12, true));
        assert_eq!(false, cpu.p.n);
        // V set (positive - negative)
        assert_eq!(0xFF, cpu.add_sub8(0x7F, 0x80, true));
        assert_eq!(true, cpu.p.v);
        // V set (negative - positive)
        cpu.p.n = false;
        assert_eq!(0x6F, cpu.add_sub8(0xEF, 0x7F, true));
        assert_eq!(true, cpu.p.v);
        // V cleared (positive - negative)
        assert_eq!(0x47, cpu.add_sub8(0x45, 0xFE, true));
        assert_eq!(false, cpu.p.v);
        // V cleared (negative - positive)
        cpu.p.n = true;
        assert_eq!(0xB8, cpu.add_sub8(0xFE, 0x45, true));
        assert_eq!(false, cpu.p.v);

        // BCD arithmetic, C set
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.d = true;
        assert_eq!(0x32, cpu.add_sub8(0x98, 0x65, true));
        assert_eq!(true, cpu.p.c);
        // Wrapping sub with carry, C cleared
        assert_eq!(0x89, cpu.add_sub8(0x12, 0x23, true));
        assert_eq!(false, cpu.p.c);
        // Z set
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x00, cpu.add_sub8(0x12, 0x11, true));
        assert_eq!(true, cpu.p.z);
        // Z cleared
        assert_eq!(0x89, cpu.add_sub8(0x12, 0x23, true));
        assert_eq!(false, cpu.p.z);
        // N set
        cpu.p.n = false;
        assert_eq!(0x88, cpu.add_sub8(0x12, 0x23, true));
        assert_eq!(true, cpu.p.n);
        // N cleared
        assert_eq!(0x10, cpu.add_sub8(0x23, 0x12, true));
        assert_eq!(false, cpu.p.n);
        // V set (positive - negative)
        assert_eq!(0x99, cpu.add_sub8(0x79, 0x80, true));
        assert_eq!(true, cpu.p.v);
        // V set (negative - positive)
        cpu.p.n = false;
        assert_eq!(0x00, cpu.add_sub8(0x80, 0x79, true));
        assert_eq!(true, cpu.p.v);
        // V cleared (positive - negative)
        assert_eq!(0x04, cpu.add_sub8(0x01, 0x97, true));
        assert_eq!(false, cpu.p.v);
        // V cleared (negative - positive)
        cpu.p.n = true;
        assert_eq!(0x95, cpu.add_sub8(0x97, 0x01, true));
        assert_eq!(false, cpu.p.v);
    }

    #[test]
    fn sub_16() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.e = false;

        // Binary arithmetic, C set as lhs >= rhs
        cpu.p.clear_flags(0b1111_1111);
        assert_eq!(0x9998, cpu.add_sub16(0xFEDC, 0x6543, true));
        assert_eq!(true, cpu.p.c);
        // Wrapping sub with carry, C cleared
        assert_eq!(0xEEEF, cpu.add_sub16(0x1234, 0x2345, true));
        assert_eq!(false, cpu.p.c);
        // Z set
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x00, cpu.add_sub16(0x1234, 0x1233, true));
        assert_eq!(true, cpu.p.z);
        // Z cleared
        assert_eq!(0xEEEF, cpu.add_sub16(0x1234, 0x2345, true));
        assert_eq!(false, cpu.p.z);
        // N set
        cpu.p.n = false;
        assert_eq!(0xEEEE, cpu.add_sub16(0x1234, 0x2345, true));
        assert_eq!(true, cpu.p.n);
        // N cleared
        assert_eq!(0x1110, cpu.add_sub16(0x2345, 0x1234, true));
        assert_eq!(false, cpu.p.n);
        // V set (positive - negative)
        assert_eq!(0xFFFE, cpu.add_sub16(0x7FFF, 0x8001, true));
        assert_eq!(true, cpu.p.v);
        // V set (negative - positive)
        cpu.p.n = false;
        assert_eq!(0x6FFF, cpu.add_sub16(0xEFFF, 0x7FFF, true));
        assert_eq!(true, cpu.p.v);
        // V cleared (positive - negative)
        assert_eq!(0x4624, cpu.add_sub16(0x4523, 0xFEFF, true));
        assert_eq!(false, cpu.p.v);
        // V cleared (negative - positive)
        cpu.p.n = true;
        assert_eq!(0xB9DB, cpu.add_sub16(0xFEFF, 0x4523, true));
        assert_eq!(false, cpu.p.v);

        // BCD arithmetic, C set
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.d = true;
        assert_eq!(0x3332, cpu.add_sub16(0x9876, 0x6543, true));
        assert_eq!(true, cpu.p.c);
        // Wrapping sub with carry, C cleared
        assert_eq!(0x8889, cpu.add_sub16(0x1234, 0x2345, true));
        assert_eq!(false, cpu.p.c);
        // Z set
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x0000, cpu.add_sub16(0x1234, 0x1233, true));
        assert_eq!(true, cpu.p.z);
        // Z cleared
        assert_eq!(0x8889, cpu.add_sub16(0x1234, 0x2345, true));
        assert_eq!(false, cpu.p.z);
        // N set
        cpu.p.n = false;
        assert_eq!(0x8888, cpu.add_sub16(0x1234, 0x2345, true));
        assert_eq!(true, cpu.p.n);
        // N cleared
        assert_eq!(0x1110, cpu.add_sub16(0x2345, 0x1234, true));
        assert_eq!(false, cpu.p.n);
        // V set (positive - negative)
        assert_eq!(0x9998, cpu.add_sub16(0x7999, 0x8001, true));
        assert_eq!(true, cpu.p.v);
        // V set (negative - positive)
        cpu.p.n = false;
        assert_eq!(0x0001, cpu.add_sub16(0x8001, 0x7999, true));
        assert_eq!(true, cpu.p.v);
        // V cleared (positive - negative)
        assert_eq!(0x0247, cpu.add_sub16(0x0123, 0x9876, true));
        assert_eq!(false, cpu.p.v);
        // V cleared (negative - positive)
        cpu.p.n = true;
        assert_eq!(0x9752, cpu.add_sub16(0x9876, 0x0123, true));
        assert_eq!(false, cpu.p.v);
    }

    #[test]
    fn compare8() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.compare8(0x12, 0x12);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        cpu.compare8(0x12, 0x11);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        cpu.compare8(0x12, 0x13);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(false, cpu.p.c);
    }

    #[test]
    fn compare16() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.compare16(0x1234, 0x1234);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        cpu.compare16(0x1234, 0x1233);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        cpu.compare16(0x1233, 0x1234);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(false, cpu.p.c);
    }

    #[test]
    fn arithmetic_shift_left8() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        assert_eq!(0xFE, cpu.arithmetic_shift_left8(0xFF));
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x7E, cpu.arithmetic_shift_left8(0x3F));
        assert_eq!(false, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x00, cpu.arithmetic_shift_left8(0x80));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
    }

    #[test]
    fn arithmetic_shift_left16() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        assert_eq!(0xFFFE, cpu.arithmetic_shift_left16(0xFFFF));
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x7FFE, cpu.arithmetic_shift_left16(0x3FFF));
        assert_eq!(false, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x0000, cpu.arithmetic_shift_left16(0x8000));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
    }

    #[test]
    fn logical_shift_right8() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        assert_eq!(0x7F, cpu.logical_shift_right8(0xFF));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x7F, cpu.logical_shift_right8(0xFE));
        assert_eq!(false, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x00, cpu.logical_shift_right8(0x01));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
    }

    #[test]
    fn logical_shift_right16() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        assert_eq!(0x7FFF, cpu.logical_shift_right16(0xFFFF));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x7FFF, cpu.logical_shift_right16(0xFFFE));
        assert_eq!(false, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x0000, cpu.logical_shift_right16(0x0001));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
    }

    #[test]
    fn rotate_left8() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        assert_eq!(0xFE, cpu.rotate_left8(0xFF));
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x7F, cpu.rotate_left8(0x3F));
        assert_eq!(false, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x00, cpu.rotate_left8(0x80));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
    }

    #[test]
    fn rotate_left16() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        assert_eq!(0xFFFE, cpu.rotate_left16(0xFFFF));
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x7FFF, cpu.rotate_left16(0x3FFF));
        assert_eq!(false, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x0000, cpu.rotate_left16(0x8000));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
    }

    #[test]
    fn rotate_right8() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        assert_eq!(0x7F, cpu.rotate_right8(0xFF));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0xFF, cpu.rotate_right8(0xFE));
        assert_eq!(false, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x00, cpu.rotate_right8(0x01));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
    }

    #[test]
    fn rotate_right16() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        assert_eq!(0x7FFF, cpu.rotate_right16(0xFFFF));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0xFFFF, cpu.rotate_right16(0xFFFE));
        assert_eq!(false, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(0x0000, cpu.rotate_right16(0x0001));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
    }

    #[test]
    fn op_adc() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);

        // 8bit binary arithmetic
        cpu.a = 0x2345;
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.m = true;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x6789, 0x000000);
        let data_addr = (0x000000, WrappingMode::Page);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x23CE, cpu.a);

        // 8bit BCD arithmetic
        cpu.a = 0x2345;
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.m = true;
        cpu.p.d = true;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1234, 0x000000);
        let data_addr = (0x000000, WrappingMode::Page);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x2379, cpu.a);

        // 16bit binary arithmetic
        cpu.a = 0x5678;
        cpu.p.clear_flags(0b1111_1111);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x6789, 0x000000);
        let data_addr = (0x000000, WrappingMode::Bank);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0xBE01, cpu.a);
        // Data bank wrapping
        cpu.a = 0x1234;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x2345, 0x00FFFF);
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3579, cpu.a);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00FFFF);
        // Data address space wrapping
        cpu.a = 0x1234;
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x2345, 0xFFFFFF);
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3579, cpu.a);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, 0xFFFFFF);

        // 16bit BCD arithmetic
        cpu.a = 0x2345;
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.d = true;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x5678, 0x000000);
        let data_addr = (0x000000, WrappingMode::Bank);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x8023, cpu.a);
        // Data bank wrapping
        cpu.a = 0x1234;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x2345, 0x00FFFF);
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3579, cpu.a);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00FFFF);
        // Data address space wrapping
        cpu.a = 0x1234;
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x2345, 0xFFFFFF);
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        cpu.op_adc(&data_addr, &mut abus);
        assert_eq!(0x3579, cpu.a);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, 0xFFFFFF);
    }

    #[test]
    fn op_sbc() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);

        // 8bit binary arithmetic
        cpu.a = 0x23CE;
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.m = true;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x6789, 0x000000);
        let data_addr = (0x000000, WrappingMode::Page);
        cpu.op_sbc(&data_addr, &mut abus);
        assert_eq!(0x2344, cpu.a);

        // 8bit BCD arithmetic
        cpu.a = 0x2379;
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.m = true;
        cpu.p.d = true;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1234, 0x000000);
        let data_addr = (0x000000, WrappingMode::Page);
        cpu.op_sbc(&data_addr, &mut abus);
        assert_eq!(0x2344, cpu.a);

        // 16bit binary arithmetic
        cpu.a = 0x0001;
        cpu.p.clear_flags(0b1111_1111);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x2003, 0x000000);
        let data_addr = (0x000000, WrappingMode::Bank);
        cpu.op_sbc(&data_addr, &mut abus);
        assert_eq!(0xDFFD, cpu.a);
        // Data bank wrapping
        cpu.a = 0x3579;
        cpu.p.clear_flags(0b1111_1111);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x2345, 0x00FFFF);
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        cpu.op_sbc(&data_addr, &mut abus);
        assert_eq!(0x1233, cpu.a);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00FFFF);
        // Data address space wrapping
        cpu.a = 0x3579;
        cpu.p.clear_flags(0b1111_1111);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x2345, 0xFFFFFF);
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        cpu.op_sbc(&data_addr, &mut abus);
        assert_eq!(0x1233, cpu.a);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, 0xFFFFFF);

        // 16bit BCD arithmetic
        cpu.a = 0x8023;
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.d = true;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x5678, 0x000000);
        let data_addr = (0x000000, WrappingMode::Bank);
        cpu.op_sbc(&data_addr, &mut abus);
        assert_eq!(0x2344, cpu.a);
        // Data bank wrapping
        cpu.a = 0x3579;
        cpu.p.clear_flags(0b1111_1111);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x2345, 0x00FFFF);
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        cpu.op_sbc(&data_addr, &mut abus);
        assert_eq!(0x1233, cpu.a);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, 0x00FFFF);
        // Data address space wrapping
        cpu.a = 0x3579;
        cpu.p.clear_flags(0b1111_1111);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x2345, 0xFFFFFF);
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        cpu.op_sbc(&data_addr, &mut abus);
        assert_eq!(0x1233, cpu.a);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, 0xFFFFFF);
    }

    #[test]
    fn op_cmp() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8-bit
        cpu.a = 0x1234;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::cpu_write8(&mut abus, 0x34, data_addr.0);
        cpu.op_cmp(&data_addr, &mut abus);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        // 16-bit, data bank wrapping
        cpu.p.m = false;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1134, data_addr.0);
        cpu.op_cmp(&data_addr, &mut abus);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x1234, data_addr.0);
        cpu.op_cmp(&data_addr, &mut abus);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_cpx() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8-bit
        cpu.x = 0x1234;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::cpu_write8(&mut abus, 0x34, data_addr.0);
        cpu.op_cpx(&data_addr, &mut abus);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        // 16-bit, data bank wrapping
        cpu.p.x = false;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1134, data_addr.0);
        cpu.op_cpx(&data_addr, &mut abus);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x1234, data_addr.0);
        cpu.op_cpx(&data_addr, &mut abus);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_cpy() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8-bit
        cpu.y = 0x1234;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::cpu_write8(&mut abus, 0x34, data_addr.0);
        cpu.op_cpy(&data_addr, &mut abus);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        // 16-bit, data bank wrapping
        cpu.p.x = false;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1134, data_addr.0);
        cpu.op_cpy(&data_addr, &mut abus);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x1234, data_addr.0);
        cpu.op_cpy(&data_addr, &mut abus);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        assert_eq!(true, cpu.p.c);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_dec() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        let data_addr = (0x00FFFF, WrappingMode::Page);
        mmap::cpu_write8(&mut abus, 0x02, data_addr.0);
        cpu.op_dec(&data_addr, &mut abus);
        assert_eq!(0x01, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.op_dec(&data_addr, &mut abus);
        assert_eq!(0x00, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        cpu.op_dec(&data_addr, &mut abus);
        assert_eq!(0xFF, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::cpu_write8(&mut abus, 0x80, data_addr.0);
        cpu.op_dec(&data_addr, &mut abus);
        assert_eq!(0x7F, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        // 16bit, data bank wrapping
        cpu.p.m = false;
        let data_addr = (data_addr.0, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0101, data_addr.0);
        cpu.op_dec(&data_addr, &mut abus);
        assert_eq!(
            0x0100,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.op_dec(&data_addr, &mut abus);
        assert_eq!(
            0x00FF,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0001, data_addr.0);
        cpu.op_dec(&data_addr, &mut abus);
        assert_eq!(
            0x0000,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        cpu.op_dec(&data_addr, &mut abus);
        assert_eq!(
            0xFFFF,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x8000, data_addr.0);
        cpu.op_dec(&data_addr, &mut abus);
        assert_eq!(
            0x7FFF,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data addr space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0200, data_addr.0);
        cpu.op_dec(&data_addr, &mut abus);
        assert_eq!(
            0x01FF,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_inc() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        let data_addr = (0x00FFFF, WrappingMode::Page);
        mmap::cpu_write8(&mut abus, 0xFE, data_addr.0);
        cpu.op_inc(&data_addr, &mut abus);
        assert_eq!(0xFF, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.op_inc(&data_addr, &mut abus);
        assert_eq!(0x00, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        cpu.op_inc(&data_addr, &mut abus);
        assert_eq!(0x01, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::cpu_write8(&mut abus, 0x7F, data_addr.0);
        cpu.op_inc(&data_addr, &mut abus);
        assert_eq!(0x80, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        // 16bit, data bank wrapping
        cpu.p.m = false;
        let data_addr = (data_addr.0, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFEFE, data_addr.0);
        cpu.op_inc(&data_addr, &mut abus);
        assert_eq!(
            0xFEFF,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.op_inc(&data_addr, &mut abus);
        assert_eq!(
            0xFF00,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        cpu.op_inc(&data_addr, &mut abus);
        assert_eq!(
            0x0000,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        cpu.op_inc(&data_addr, &mut abus);
        assert_eq!(
            0x0001,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x7FFF, data_addr.0);
        cpu.op_inc(&data_addr, &mut abus);
        assert_eq!(
            0x8000,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x01FF, data_addr.0);
        cpu.op_inc(&data_addr, &mut abus);
        assert_eq!(
            0x0200,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_and() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.a = 0x4567;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFEDC, data_addr.0);
        cpu.op_and(&data_addr, &mut abus);
        assert_eq!(0x4544, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0x4597;
        cpu.op_and(&data_addr, &mut abus);
        assert_eq!(0x4594, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0x4523;
        cpu.op_and(&data_addr, &mut abus);
        assert_eq!(0x4500, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        // 16bit, data bank wrapping
        cpu.p.m = false;
        cpu.a = 0x4567;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFEDC, data_addr.0);
        cpu.op_and(&data_addr, &mut abus);
        assert_eq!(0x4444, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0x9876;
        cpu.op_and(&data_addr, &mut abus);
        assert_eq!(0x9854, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0x0123;
        cpu.op_and(&data_addr, &mut abus);
        assert_eq!(0x0000, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        cpu.a = 0x4567;
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFEDC, data_addr.0);
        cpu.op_and(&data_addr, &mut abus);
        assert_eq!(0x4444, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_eor() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.a = 0xBA98;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFEDC, data_addr.0);
        cpu.op_eor(&data_addr, &mut abus);
        assert_eq!(0xBA44, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0x4567;
        cpu.op_eor(&data_addr, &mut abus);
        assert_eq!(0x45BB, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0x45DC;
        cpu.op_eor(&data_addr, &mut abus);
        assert_eq!(0x4500, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        // 16bit, data bank wrapping
        cpu.p.m = false;
        cpu.a = 0xBA98;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFEDC, data_addr.0);
        cpu.op_eor(&data_addr, &mut abus);
        assert_eq!(0x4444, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0x4567;
        cpu.op_eor(&data_addr, &mut abus);
        assert_eq!(0xBBBB, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0xFEDC;
        cpu.op_eor(&data_addr, &mut abus);
        assert_eq!(0x0000, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        cpu.a = 0xBA98;
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFEDC, data_addr.0);
        cpu.op_eor(&data_addr, &mut abus);
        assert_eq!(0x4444, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_ora() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.a = 0x1234;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x4567, data_addr.0);
        cpu.op_ora(&data_addr, &mut abus);
        assert_eq!(0x1277, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0x45AB;
        cpu.op_ora(&data_addr, &mut abus);
        assert_eq!(0x45EF, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0x4500;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1200, data_addr.0);
        cpu.op_ora(&data_addr, &mut abus);
        assert_eq!(0x4500, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        // 16bit, data bank wrapping
        cpu.p.m = false;
        cpu.a = 0x1234;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x4567, data_addr.0);
        cpu.op_ora(&data_addr, &mut abus);
        assert_eq!(0x5777, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0xFEDC;
        cpu.op_ora(&data_addr, &mut abus);
        assert_eq!(0xFFFF, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        cpu.a = 0x0000;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        cpu.op_ora(&data_addr, &mut abus);
        assert_eq!(0x0000, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        // Data address space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        cpu.a = 0x1234;
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x4567, data_addr.0);
        cpu.op_ora(&data_addr, &mut abus);
        assert_eq!(0x5777, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_bit() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.a = 0x1234;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFEDC, data_addr.0);
        cpu.op_bit(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(true, cpu.p.v);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFEAC, data_addr.0);
        cpu.op_bit(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.v);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFE7C, data_addr.0);
        cpu.op_bit(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.v);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFE88, data_addr.0);
        cpu.op_bit(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.v);
        assert_eq!(true, cpu.p.z);
        // 16bit, data bank wrapping
        cpu.p.m = false;
        cpu.a = 0x1234;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFEDC, data_addr.0);
        cpu.op_bit(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(true, cpu.p.v);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xAEDC, data_addr.0);
        cpu.op_bit(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.v);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x7EDC, data_addr.0);
        cpu.op_bit(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.v);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xE988, data_addr.0);
        cpu.op_bit(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(true, cpu.p.v);
        assert_eq!(true, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFEDC, data_addr.0);
        cpu.op_bit(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(true, cpu.p.v);
        assert_eq!(false, cpu.p.z);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_trb() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.a = 0x1234;
        cpu.p.z = true;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        // Right bits cleared, Z cleared
        cpu.op_trb(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(
            0xFFCB,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(false, cpu.p.z);
        // Z set
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFCB, data_addr.0);
        cpu.op_trb(&data_addr, &mut abus);
        assert_eq!(
            0xFFCB,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.z);
        // 16bit
        cpu.p.m = false;
        cpu.p.z = true;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        // Right bits cleared, Z cleared, data bank wrapping
        cpu.op_trb(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(
            0xEDCB,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(false, cpu.p.z);
        // Z set
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xEDCB, data_addr.0);
        cpu.op_trb(&data_addr, &mut abus);
        assert_eq!(
            0xEDCB,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        cpu.op_trb(&data_addr, &mut abus);
        assert_eq!(
            0xEDCB,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(false, cpu.p.z);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_tsb() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.a = 0x1234;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Right bits set, Z set
        cpu.op_tsb(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(
            0x0034,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.z);
        // Z cleared
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        cpu.op_tsb(&data_addr, &mut abus);
        assert_eq!(
            0xFFFF,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(false, cpu.p.z);
        // 16bit
        cpu.p.m = false;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Right bits set, Z set, data bank wrapping
        cpu.op_tsb(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        assert_eq!(
            0x1234,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.z);
        // Z cleared
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        cpu.op_tsb(&data_addr, &mut abus);
        assert_eq!(
            0xFFFF,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        cpu.op_tsb(&data_addr, &mut abus);
        assert_eq!(
            0x1234,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.z);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_asl() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.p.c = true;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        cpu.op_asl(&data_addr, &mut abus);
        assert_eq!(0xFE, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        // 16bit, data bank wrapping
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.c = true;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        cpu.op_asl(&data_addr, &mut abus);
        assert_eq!(
            0xFFFE,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address wrapping
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.c = true;
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        cpu.op_asl(&data_addr, &mut abus);
        assert_eq!(
            0xFFFE,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_lsr() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.p.c = true;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        cpu.op_lsr(&data_addr, &mut abus);
        assert_eq!(0x7F, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        // 16bit, data bank wrapping
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.c = true;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        cpu.op_lsr(&data_addr, &mut abus);
        assert_eq!(
            0x7FFF,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address wrapping
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.c = true;
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFFFF, data_addr.0);
        cpu.op_lsr(&data_addr, &mut abus);
        assert_eq!(
            0x7FFF,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.c);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_rol() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.p.c = true;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFE, data_addr.0);
        cpu.op_rol(&data_addr, &mut abus);
        assert_eq!(0xFD, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        // 16bit, data bank wrapping
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.c = true;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFFFE, data_addr.0);
        cpu.op_rol(&data_addr, &mut abus);
        assert_eq!(
            0xFFFD,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address wrapping
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.c = true;
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFFFE, data_addr.0);
        cpu.op_rol(&data_addr, &mut abus);
        assert_eq!(
            0xFFFD,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_ror() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.p.c = true;
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_ror(&data_addr, &mut abus);
        assert_eq!(0xBF, mmap::cpu_read8(&mut abus, data_addr.0));
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        // 16bit, data bank wrapping
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.c = true;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x7FFF, data_addr.0);
        cpu.op_ror(&data_addr, &mut abus);
        assert_eq!(
            0xBFFF,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data address wrapping
        cpu.p.clear_flags(0b1111_1111);
        cpu.p.c = true;
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x7FFF, data_addr.0);
        cpu.op_ror(&data_addr, &mut abus);
        assert_eq!(
            0xBFFF,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        assert_eq!(true, cpu.p.c);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_lda() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.a = 0x1234;
        let data_addr = (0x000000, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_lda(&data_addr, &mut abus);
        assert_eq!(0x127F, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF8F, data_addr.0);
        cpu.op_lda(&data_addr, &mut abus);
        assert_eq!(0x128F, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF00, data_addr.0);
        cpu.op_lda(&data_addr, &mut abus);
        assert_eq!(0x1200, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        // 16bit
        cpu.a = 0x1234;
        cpu.p.m = false;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_lda(&data_addr, &mut abus);
        assert_eq!(0xFF7F, cpu.a);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x7FFF, data_addr.0);
        cpu.op_lda(&data_addr, &mut abus);
        assert_eq!(0x7FFF, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        cpu.op_lda(&data_addr, &mut abus);
        assert_eq!(0x0000, cpu.a);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        // Data bank wrapping
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1234, data_addr.0);
        cpu.op_lda(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.a);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data addr space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x2345, data_addr.0);
        cpu.op_lda(&data_addr, &mut abus);
        assert_eq!(0x2345, cpu.a);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_ldx() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.x = 0x0000;
        let data_addr = (0x000000, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_ldx(&data_addr, &mut abus);
        assert_eq!(0x007F, cpu.x);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF8F, data_addr.0);
        cpu.op_ldx(&data_addr, &mut abus);
        assert_eq!(0x008F, cpu.x);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF00, data_addr.0);
        cpu.op_ldx(&data_addr, &mut abus);
        assert_eq!(0x0000, cpu.x);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        // 16bit
        cpu.x = 0x1234;
        cpu.p.x = false;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_ldx(&data_addr, &mut abus);
        assert_eq!(0xFF7F, cpu.x);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x7FFF, data_addr.0);
        cpu.op_ldx(&data_addr, &mut abus);
        assert_eq!(0x7FFF, cpu.x);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        cpu.op_ldx(&data_addr, &mut abus);
        assert_eq!(0x0000, cpu.x);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        // Data bank wrapping
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1234, data_addr.0);
        cpu.op_ldx(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.x);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data addr space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x2345, data_addr.0);
        cpu.op_ldx(&data_addr, &mut abus);
        assert_eq!(0x2345, cpu.x);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_ldy() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.y = 0x0000;
        let data_addr = (0x000000, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_ldy(&data_addr, &mut abus);
        assert_eq!(0x007F, cpu.y);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF8F, data_addr.0);
        cpu.op_ldy(&data_addr, &mut abus);
        assert_eq!(0x008F, cpu.y);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF00, data_addr.0);
        cpu.op_ldy(&data_addr, &mut abus);
        assert_eq!(0x0000, cpu.y);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        // 16bit
        cpu.y = 0x1234;
        cpu.p.x = false;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_ldy(&data_addr, &mut abus);
        assert_eq!(0xFF7F, cpu.y);
        assert_eq!(true, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x7FFF, data_addr.0);
        cpu.op_ldy(&data_addr, &mut abus);
        assert_eq!(0x7FFF, cpu.y);
        assert_eq!(false, cpu.p.n);
        assert_eq!(false, cpu.p.z);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        cpu.op_ldy(&data_addr, &mut abus);
        assert_eq!(0x0000, cpu.y);
        assert_eq!(false, cpu.p.n);
        assert_eq!(true, cpu.p.z);
        // Data bank wrapping
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x1234, data_addr.0);
        cpu.op_ldy(&data_addr, &mut abus);
        assert_eq!(0x1234, cpu.y);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data addr space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x2345, data_addr.0);
        cpu.op_ldy(&data_addr, &mut abus);
        assert_eq!(0x2345, cpu.y);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_sta() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.a = 0x1234;
        let data_addr = (0x000000, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_sta(&data_addr, &mut abus);
        assert_eq!(
            0xFF34,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        // 16bit
        cpu.p.m = false;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_sta(&data_addr, &mut abus);
        assert_eq!(
            0x1234,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data bank wrapping
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_sta(&data_addr, &mut abus);
        assert_eq!(
            0x1234,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data addr space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_sta(&data_addr, &mut abus);
        assert_eq!(
            0x1234,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_stx() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.x = 0x0034;
        let data_addr = (0x000000, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_stx(&data_addr, &mut abus);
        assert_eq!(
            0xFF34,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        // 16bit
        cpu.p.x = false;
        cpu.x = 0x1234;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_stx(&data_addr, &mut abus);
        assert_eq!(
            0x1234,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data bank wrapping
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_stx(&data_addr, &mut abus);
        assert_eq!(
            0x1234,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data addr space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_stx(&data_addr, &mut abus);
        assert_eq!(
            0x1234,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_sty() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        cpu.y = 0x0034;
        let data_addr = (0x000000, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_sty(&data_addr, &mut abus);
        assert_eq!(
            0xFF34,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        // 16bit
        cpu.p.x = false;
        cpu.y = 0x1234;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_sty(&data_addr, &mut abus);
        assert_eq!(
            0x1234,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data bank wrapping
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_sty(&data_addr, &mut abus);
        assert_eq!(
            0x1234,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data addr space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_sty(&data_addr, &mut abus);
        assert_eq!(
            0x1234,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::addr_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
    }

    #[test]
    fn op_stz() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        // 8bit
        let data_addr = (0x000000, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_stz(&data_addr, &mut abus);
        assert_eq!(
            0xFF00,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        // 16bit
        cpu.p.m = false;
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_stz(&data_addr, &mut abus);
        assert_eq!(
            0x0000,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        mmap::bank_wrapping_cpu_write16(&mut abus, 0x0000, data_addr.0);
        // Data bank wrapping
        let data_addr = (0x00FFFF, WrappingMode::Bank);
        mmap::bank_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_sta(&data_addr, &mut abus);
        assert_eq!(
            0x0000,
            mmap::bank_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
        // Data addr space wrapping
        let data_addr = (0xFFFFFF, WrappingMode::AddrSpace);
        mmap::addr_wrapping_cpu_write16(&mut abus, 0xFF7F, data_addr.0);
        cpu.op_stz(&data_addr, &mut abus);
        assert_eq!(
            0x0000,
            mmap::addr_wrapping_cpu_read16(&mut abus, data_addr.0)
        );
    }

    #[test]
    fn op_mvn() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.p.x = false;
        cpu.pc = 0x0000;
        cpu.db = 0x0000;
        cpu.a = 0x0000;
        cpu.x = 0x0000;
        cpu.y = 0x0000;
        // Simple move
        let data_addrs = (0x010000, 0x020000);
        mmap::cpu_write8(&mut abus, 0x12, data_addrs.0);
        cpu.op_mvn(&data_addrs, &mut abus);
        assert_eq!(0x12, mmap::cpu_read8(&mut abus, data_addrs.1));
        assert_eq!(0xFFFF, cpu.a);
        assert_eq!(0x0001, cpu.x);
        assert_eq!(0x0001, cpu.y);
        assert_eq!(0x02, cpu.db);
        // X, Y wrapping
        cpu.x = 0xFFFF;
        cpu.y = 0xFFFF;
        cpu.op_mvn(&data_addrs, &mut abus);
        assert_eq!(0x0000, cpu.x);
        assert_eq!(0x0000, cpu.y);
        // "Looping"
        cpu.a = 0x0005;
        cpu.x = 0x0000;
        cpu.y = 0x0000;
        while cpu.a != 0xFFFF {
            cpu.op_mvn(&data_addrs, &mut abus);
        }
        assert_eq!(0x0006, cpu.x);
        assert_eq!(0x0006, cpu.y);
    }

    #[test]
    fn op_mvp() {
        let mut abus = ABus::new_empty_rom();
        let mut cpu = W65C816S::new(&mut abus);
        cpu.p.x = false;
        cpu.pc = 0x0000;
        cpu.db = 0x0000;
        cpu.a = 0x0000;
        cpu.x = 0xFFFF;
        cpu.y = 0xFFFF;
        // Simple move
        let data_addrs = (0x010000, 0x020000);
        mmap::cpu_write8(&mut abus, 0x12, data_addrs.0);
        cpu.op_mvp(&data_addrs, &mut abus);
        assert_eq!(0x12, mmap::cpu_read8(&mut abus, data_addrs.1));
        assert_eq!(0xFFFF, cpu.a);
        assert_eq!(0xFFFE, cpu.x);
        assert_eq!(0xFFFE, cpu.y);
        assert_eq!(0x02, cpu.db);
        // X, Y wrapping
        cpu.x = 0x0000;
        cpu.y = 0x0000;
        cpu.op_mvp(&data_addrs, &mut abus);
        assert_eq!(0xFFFF, cpu.x);
        assert_eq!(0xFFFF, cpu.y);
        // "Looping"
        cpu.a = 0x0005;
        while cpu.a != 0xFFFF {
            cpu.op_mvp(&data_addrs, &mut abus);
        }
        assert_eq!(0xFFF9, cpu.x);
        assert_eq!(0xFFF9, cpu.y);
    }
}

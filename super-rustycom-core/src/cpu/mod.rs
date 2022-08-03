mod op;

use crate::abus::ABus;
use log::error;

/// The cpu core in Ricoh 5A22 powering the Super Nintendo
pub struct W65C816S {
    /// Accumulator
    ///
    /// Whole register denoted with `A`, high byte with `AH` and low byte with `AL`
    a: u16,
    /// Index register X
    ///
    /// Whole register denoted with `X`, high byte `XH` and low byte with `XL`
    x: u16,
    /// Index register Y
    ///
    /// Whole register denoted with `Y`, high byte `YH` and low byte with `YL`
    y: u16,
    /// Program counter
    ///
    /// Whole register denoted with `PC`, high byte `PCH` and low byte with `PCL`
    pc: u16,
    /// Stack pointer
    ///
    /// Whole register denoted with `S`, high byte `SH` and low byte with `SL`
    s: u16,
    /// Processor status register
    ///
    /// Denoted with `P` and contains flags
    /// `N`egative
    ///
    /// O`V`erflow
    ///
    /// `M`emory and accumulator width
    ///
    /// Inde`X` register width
    ///
    /// `D`ecimal mode
    ///
    /// `I`nterrupt
    ///
    /// `Z`ero
    ///
    /// `C`arry
    p: StatusReg,
    /// Direct page register
    ///
    /// Whole register denoted with `D`, high byte `DH` and low byte with `DL`
    d: u16,
    /// Program bank register
    ///
    /// Denoted with `PB`
    pb: u8,
    /// Data bank register
    db: u8,
    /// Emulation flag
    e: bool,
    /// `true` if stopped until interrupted
    stopped: bool,
    /// `true` if waiting until interrupted
    waiting: bool,
}

impl W65C816S {
    /// Initializes a new instance with default values
    ///
    /// The processor starts in emulation mode, `PC` is set to the reset vector,
    /// `S` is set to `$01FF`, `A`, `X` and `Y` are 8bits wide
    /// and interrupts are disabled. Other values are zeroed.
    pub fn new(abus: &mut ABus) -> W65C816S {
        W65C816S {
            a: 0x00,
            x: 0x00,
            y: 0x00,
            pc: abus.page_wrapping_cpu_read16(RESET8),
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

    pub fn reset(&mut self, abus: &mut ABus) {
        self.pc = abus.page_wrapping_cpu_read16(RESET8);
    }

    /// Returns the value of `C`
    pub fn a(&self) -> u16 {
        self.a
    }
    /// Returns the value of `X`
    pub fn x(&self) -> u16 {
        self.x
    }
    /// Returns the value of `Y`
    pub fn y(&self) -> u16 {
        self.y
    }
    /// Returns the value of `PB`
    pub fn pb(&self) -> u8 {
        self.pb
    }
    /// Returns the value of `DB`
    pub fn db(&self) -> u8 {
        self.db
    }
    /// Returns the value of `PC`
    pub fn pc(&self) -> u16 {
        self.pc
    }
    /// Returns the value of `S`
    pub fn s(&self) -> u16 {
        self.s
    }
    /// Returns the value of `D`
    pub fn d(&self) -> u16 {
        self.d
    }
    /// Returns the value of `E`
    pub fn e(&self) -> bool {
        self.e
    }
    /// Returns the value of `stopped`
    pub fn stopped(&self) -> bool {
        self.stopped
    }
    /// Returns the value of `waiting`
    pub fn waiting(&self) -> bool {
        self.waiting
    }
    /// Returns the value of the carry flag
    pub fn p_c(&self) -> bool {
        self.p.c
    }
    /// Returns the value of the zero flag
    pub fn p_z(&self) -> bool {
        self.p.z
    }
    /// Returns the value of the interrupt flag
    pub fn p_i(&self) -> bool {
        self.p.i
    }
    /// Returns the value of the decimal mode flag
    pub fn p_d(&self) -> bool {
        self.p.d
    }
    /// Returns the value of the X, Y register width flag
    pub fn p_x(&self) -> bool {
        self.p.x
    }
    /// Returns the value of the accumulator width flag
    pub fn p_m(&self) -> bool {
        self.p.m
    }
    /// Returns the value of the overflow flag
    pub fn p_v(&self) -> bool {
        self.p.v
    }
    /// Returns the value of the negative flag
    pub fn p_n(&self) -> bool {
        self.p.n
    }
    /// Returns the address of the next instruction
    pub fn current_address(&self) -> u32 {
        ((self.pb as u32) << 16) + self.pc as u32
    }

    /// Executes the instruction at `[$PBPCHPCL]` and returns the number of cycles it took
    ///
    /// `abus` is used for memory addressing as needed
    pub fn step(&mut self, abus: &mut ABus) -> u8 {
        let addr = self.current_address();
        let opcode = abus.cpu_read8(addr);

        // Executes op_func with data pointed by addressing and increments pc by op_length
        macro_rules! op {
            ($addressing:ident, $op_func:ident, $op_length:expr, $op_cycles:expr) => {{
                let data_addr = self.$addressing(addr, abus);
                self.$op_func(&data_addr, abus);
                self.pc = self.pc.wrapping_add($op_length);
                $op_cycles
            }};
        }

        // Executes op_func with data pointed by addressing and increments pc by op_length
        // Special version for instructions whose length vary on page boundary crossing
        macro_rules! op_p {
            ($addressing:ident, $op_func:ident, $op_length:expr, $op_cycles:expr) => {{
                let data_addr = self.$addressing(addr, abus);
                self.$op_func(&data_addr, abus);
                self.pc = self.pc.wrapping_add($op_length);
                let page_crossed = if data_addr.0 & 0xFF00 != addr & 0xFF00 {
                    1
                } else {
                    0
                };
                $op_cycles - self.p.x as u8 + self.p.x as u8 * page_crossed
            }};
        }

        // Used in place of INC and DEC that operate on cpu registers. Applies op (used with
        // wrapping_add or wrapping_sub) to given register at 8/16bits wide based on cond and
        // increments pc by one
        // N indicates the high bit of the result and Z if it's 0
        macro_rules! inc_dec {
            ($cond:expr, $reg_ref:expr, $op:ident) => {{
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
                2 // All incs and decs of registers are two cycles
            }};
        }

        // Branches to relative8 address if cond is true and increments pc by 2
        macro_rules! branch {
            ($condition:expr) => {{
                let cycles;
                if $condition {
                    let old_pc = self.pc;
                    self.pc = self.rel8(addr, abus).0 as u16;
                    // Add one cycle for branch and another if page boundary is crossed in
                    // emulation mode
                    cycles = if self.e && (self.pc & 0xFF00 != old_pc & 0xFF00) {
                        4
                    } else {
                        3
                    };
                } else {
                    self.pc = self.pc.wrapping_add(2);
                    cycles = 2;
                }
                cycles
            }};
        }

        // Sets flag to value and increments pc by 1
        macro_rules! cl_se {
            ($flag:expr, $value:expr) => {{
                $flag = $value;
                self.pc = self.pc.wrapping_add(1);
                2 // All CL*, SE* take two cycles
            }};
        }

        // Pushes the 16bit value pointed by addressing to stack and increments pc by op_length
        macro_rules! push_eff {
            ($addressing:ident, $op_length:expr, $op_cycles:expr) => {{
                let data_addr = self.$addressing(addr, abus);
                let data = abus.bank_wrapping_cpu_read16(data_addr.0);
                self.push16(data, abus);
                self.pc = self.pc.wrapping_add($op_length);
                $op_cycles
            }};
        }

        // Pushes register to stack at 8/16bits wide based on cond and increments pc by 1
        macro_rules! push_reg {
            ($cond:expr, $reg_val:expr, $op_cycles:expr) => {{
                if $cond {
                    let value = $reg_val as u8;
                    self.push8(value, abus);
                } else {
                    let value = $reg_val as u16;
                    self.push16(value, abus);
                }
                self.pc = self.pc.wrapping_add(1);
                $op_cycles
            }};
        }

        // Pulls 8/16bits from stack to given register based on cond and increments pc by 1
        // N indicates the high bit of the result and Z if it's 0
        macro_rules! pull_reg {
            ($cond:expr, $reg_ref:expr, $op_cycles:expr) => {{
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
                $op_cycles
            }};
        }

        // Sets 8/16bits in dst_reg to those of src_reg based on cond
        // N indicates the high bit of the result and Z if it's 0
        macro_rules! transfer {
            ($cond:expr, $src_reg:expr, $dst_reg:expr) => {{
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
                2 // All transfers take two cycles
            }};
        }

        // Some instructions take a cycle less if `DL` is `00`
        let dl_zero = if self.d & 0x00FF == 0 { 0 } else { 1 };
        // Cast flags to uint for use in cycle counts
        let e = if self.e { 1 } else { 0 };
        let m = if self.p.m { 1 } else { 0 };
        let x = if self.p.x { 1 } else { 0 };

        // Execute opcodes, utilize macros and op_funcs in common cases
        match opcode {
            op::ADC_61 => op!(dir_x_ptr16, op_adc, 2, 7 - m + dl_zero),
            op::ADC_63 => op!(stack, op_adc, 2, 5 - m),
            op::ADC_65 => op!(dir, op_adc, 2, 4 - m + dl_zero),
            op::ADC_67 => op!(dir_ptr24, op_adc, 2, 7 - m + dl_zero),
            op::ADC_69 => op!(imm, op_adc, 3 - self.p.m as u16, 6 - m),
            op::ADC_6D => op!(abs, op_adc, 3, 5 - m),
            op::ADC_6F => op!(long, op_adc, 4, 6 - m),
            op::ADC_71 => op_p!(dir_ptr16_y, op_adc, 2, 7 - m + dl_zero),
            op::ADC_72 => op!(dir_ptr16, op_adc, 2, 6 - m + dl_zero),
            op::ADC_73 => op!(stack_ptr16_y, op_adc, 2, 8 - m),
            op::ADC_75 => op!(dir_x, op_adc, 2, 5 - m + dl_zero),
            op::ADC_77 => op!(dir_ptr24_y, op_adc, 2, 7 - m + dl_zero),
            op::ADC_79 => op_p!(abs_y, op_adc, 3, 6 - m),
            op::ADC_7D => op_p!(abs_x, op_adc, 3, 6 - m),
            op::ADC_7F => op!(long_x, op_adc, 4, 6 - m),
            op::SBC_E1 => op!(dir_x_ptr16, op_sbc, 2, 7 - m + dl_zero),
            op::SBC_E3 => op!(stack, op_sbc, 2, 5 - m),
            op::SBC_E5 => op!(dir, op_sbc, 2, 4 - m + dl_zero),
            op::SBC_E7 => op!(dir_ptr24, op_sbc, 2, 7 - m + dl_zero),
            op::SBC_E9 => op!(imm, op_sbc, 3 - self.p.m as u16, 3 - m),
            op::SBC_ED => op!(abs, op_sbc, 3, 5 - m),
            op::SBC_EF => op!(long, op_sbc, 4, 6 - m),
            op::SBC_F1 => op_p!(dir_ptr16_y, op_sbc, 2, 7 - m + dl_zero),
            op::SBC_F2 => op!(dir_ptr16, op_sbc, 2, 6 - m + dl_zero),
            op::SBC_F3 => op!(stack_ptr16_y, op_sbc, 2, 8 - m),
            op::SBC_F5 => op!(dir_x, op_sbc, 2, 5 - m + dl_zero),
            op::SBC_F7 => op!(dir_ptr24_y, op_sbc, 2, 7 - m + dl_zero),
            op::SBC_F9 => op_p!(abs_y, op_sbc, 3, 6 - m),
            op::SBC_FD => op_p!(abs_x, op_sbc, 3, 6 - m),
            op::SBC_FF => op!(long_x, op_sbc, 4, 6 - m),
            op::CMP_C1 => op!(dir_x_ptr16, op_cmp, 2, 7 - m + dl_zero),
            op::CMP_C3 => op!(stack, op_cmp, 2, 5 - m),
            op::CMP_C5 => op!(dir, op_cmp, 2, 4 - m + dl_zero),
            op::CMP_C7 => op!(dir_ptr24, op_cmp, 2, 7 - m + dl_zero),
            op::CMP_C9 => op!(imm, op_cmp, 3 - self.p.m as u16, 3 - m),
            op::CMP_CD => op!(abs, op_cmp, 3, 5 - m),
            op::CMP_CF => op!(long, op_cmp, 4, 6 - m),
            op::CMP_D1 => op_p!(dir_ptr16_y, op_cmp, 2, 7 - m + dl_zero),
            op::CMP_D2 => op!(dir_ptr16, op_cmp, 2, 6 - m + dl_zero),
            op::CMP_D3 => op!(stack_ptr16_y, op_cmp, 2, 8 - m),
            op::CMP_D5 => op!(dir_x, op_cmp, 2, 5 - m + dl_zero),
            op::CMP_D7 => op!(dir_ptr24_y, op_cmp, 2, 7 - m + dl_zero),
            op::CMP_D9 => op_p!(abs_y, op_cmp, 3, 6 - m),
            op::CMP_DD => op_p!(abs_x, op_cmp, 3, 6 - m),
            op::CMP_DF => op!(long_x, op_cmp, 4, 6 - m),
            op::CPX_E0 => op!(imm, op_cpx, 3 - self.p.x as u16, 3 - x),
            op::CPX_E4 => op!(dir, op_cpx, 2, 4 - x + dl_zero),
            op::CPX_EC => op!(abs, op_cpx, 3, 5 - x),
            op::CPY_C0 => op!(imm, op_cpy, 3 - self.p.x as u16, 3 - x),
            op::CPY_C4 => op!(dir, op_cpy, 2, 4 + x + dl_zero),
            op::CPY_CC => op!(abs, op_cpy, 3, 5 - x),
            op::DEC_3A => inc_dec!(self.p.m, self.a, wrapping_sub),
            op::DEC_C6 => op!(dir, op_dec, 2, 7 - 2 * m + dl_zero),
            op::DEC_CE => op!(abs, op_dec, 3, 8 - 2 * m),
            op::DEC_D6 => op!(dir_x, op_dec, 2, 8 - 2 * m + dl_zero),
            op::DEC_DE => op!(abs_x, op_dec, 3, 9 - 2 * m),
            op::DEX => inc_dec!(self.p.x, self.x, wrapping_sub),
            op::DEY => inc_dec!(self.p.x, self.y, wrapping_sub),
            op::INC_1A => inc_dec!(self.p.m, self.a, wrapping_add),
            op::INC_E6 => op!(dir, op_inc, 2, 7 - 2 * m + dl_zero),
            op::INC_EE => op!(abs, op_inc, 3, 8 - 2 * m),
            op::INC_F6 => op!(dir_x, op_inc, 2, 8 - 2 * m + dl_zero),
            op::INC_FE => op!(abs_x, op_inc, 3, 9 - 2 * m),
            op::INX => inc_dec!(self.p.x, self.x, wrapping_add),
            op::INY => inc_dec!(self.p.x, self.y, wrapping_add),
            op::AND_21 => op!(dir_x_ptr16, op_and, 2, 7 - m + dl_zero),
            op::AND_23 => op!(stack, op_and, 2, 5 - m),
            op::AND_25 => op!(dir, op_and, 2, 4 - m + dl_zero),
            op::AND_27 => op!(dir_ptr24, op_and, 2, 7 - m + dl_zero),
            op::AND_29 => op!(imm, op_and, 3 - self.p.m as u16, 3 - m),
            op::AND_2D => op!(abs, op_and, 3, 5 - m),
            op::AND_2F => op!(long, op_and, 4, 6 - m),
            op::AND_31 => op_p!(dir_ptr16_y, op_and, 2, 7 - m + dl_zero),
            op::AND_32 => op!(dir_ptr16, op_and, 2, 6 - m + dl_zero),
            op::AND_33 => op!(stack_ptr16_y, op_and, 2, 8 - m),
            op::AND_35 => op!(dir_x, op_and, 2, 5 - m + dl_zero),
            op::AND_37 => op!(dir_ptr24_y, op_and, 2, 7 - m + dl_zero),
            op::AND_39 => op_p!(abs_y, op_and, 3, 6 - m),
            op::AND_3D => op_p!(abs_x, op_and, 3, 6 - m),
            op::AND_3F => op!(long_x, op_and, 4, 6 - m),
            op::EOR_41 => op!(dir_x_ptr16, op_eor, 2, 7 - m + dl_zero),
            op::EOR_43 => op!(stack, op_eor, 2, 5 - m),
            op::EOR_45 => op!(dir, op_eor, 2, 4 - m + dl_zero),
            op::EOR_47 => op!(dir_ptr24, op_eor, 2, 7 - m + dl_zero),
            op::EOR_49 => op!(imm, op_eor, 3 - self.p.m as u16, 3 - m),
            op::EOR_4D => op!(abs, op_eor, 3, 5 - m),
            op::EOR_4F => op!(long, op_eor, 4, 6 - m),
            op::EOR_51 => op_p!(dir_ptr16_y, op_eor, 2, 7 - m + dl_zero),
            op::EOR_52 => op!(dir_ptr16, op_eor, 2, 6 - m + dl_zero),
            op::EOR_53 => op!(stack_ptr16_y, op_eor, 2, 8 - m),
            op::EOR_55 => op!(dir_x, op_eor, 2, 5 - m + dl_zero),
            op::EOR_57 => op!(dir_ptr24_y, op_eor, 2, 7 - m + dl_zero),
            op::EOR_59 => op_p!(abs_y, op_eor, 3, 6 - m),
            op::EOR_5D => op_p!(abs_x, op_eor, 3, 6 - m),
            op::EOR_5F => op!(long_x, op_eor, 4, 6 - m),
            op::ORA_01 => op!(dir_x_ptr16, op_ora, 2, 7 - m + dl_zero),
            op::ORA_03 => op!(stack, op_ora, 2, 5 - m),
            op::ORA_05 => op!(dir, op_ora, 2, 4 - m + dl_zero),
            op::ORA_07 => op!(dir_ptr24, op_ora, 2, 7 - m + dl_zero),
            op::ORA_09 => op!(imm, op_ora, 3 - self.p.m as u16, 3 - m),
            op::ORA_0D => op!(abs, op_ora, 3, 5 - m),
            op::ORA_0F => op!(long, op_ora, 4, 6 - m),
            op::ORA_11 => op_p!(dir_ptr16_y, op_ora, 2, 7 - m + dl_zero),
            op::ORA_12 => op!(dir_ptr16, op_ora, 2, 6 - m + dl_zero),
            op::ORA_13 => op!(stack_ptr16_y, op_ora, 2, 8 - m),
            op::ORA_15 => op!(dir_x, op_ora, 2, 5 - m + dl_zero),
            op::ORA_17 => op!(dir_ptr24_y, op_ora, 2, 7 - m + dl_zero),
            op::ORA_19 => op_p!(abs_y, op_ora, 3, 6 - m),
            op::ORA_1D => op_p!(abs_x, op_ora, 3, 6 - m),
            op::ORA_1F => op!(long_x, op_ora, 4, 6 - m),
            op::BIT_24 => op!(dir, op_bit, 2, 4 - m + dl_zero),
            op::BIT_2C => op!(abs, op_bit, 3, 5 - m),
            op::BIT_34 => op!(dir_x, op_bit, 2, 5 - m + dl_zero),
            op::BIT_3C => op_p!(abs_x, op_bit, 3, 6 - m),
            op::BIT_89 => op!(imm, op_bit, 3 - self.p.m as u16, 3 - m), // TODO: Only affects Z
            op::TRB_14 => op!(dir, op_trb, 2, 7 - 2 * m + dl_zero),
            op::TRB_1C => op!(abs, op_trb, 3, 8 - 2 * m),
            op::TSB_04 => op!(dir, op_tsb, 2, 7 - 2 * m + dl_zero),
            op::TSB_0C => op!(abs, op_tsb, 3, 8 - 2 * m),
            op::ASL_06 => op!(dir, op_asl, 2, 7 - 2 * m + dl_zero),
            op::ASL_0A => {
                if self.p.m {
                    let old_a = self.a as u8;
                    self.a = (self.a & 0xFF00) | self.arithmetic_shift_left8(old_a) as u16;
                } else {
                    let old_a = self.a;
                    self.a = self.arithmetic_shift_left16(old_a);
                }
                self.pc = self.pc.wrapping_add(1);
                2
            }
            op::ASL_0E => op!(abs, op_asl, 3, 8 - 2 * m),
            op::ASL_16 => op!(dir_x, op_asl, 2, 8 - 2 * m + dl_zero),
            op::ASL_1E => op!(abs_x, op_asl, 3, 9 - 2 * m),
            op::LSR_46 => op!(dir, op_lsr, 2, 7 - 2 * m + dl_zero),
            op::LSR_4A => {
                if self.p.m {
                    let old_a = self.a as u8;
                    self.a = (self.a & 0xFF00) | self.logical_shift_right8(old_a) as u16;
                } else {
                    let old_a = self.a;
                    self.a = self.logical_shift_right16(old_a);
                }
                self.pc = self.pc.wrapping_add(1);
                2
            }
            op::LSR_4E => op!(abs, op_lsr, 3, 8 - 2 * m),
            op::LSR_56 => op!(dir_x, op_lsr, 2, 8 - 2 * m + dl_zero),
            op::LSR_5E => op!(abs_x, op_lsr, 3, 9 - 2 * m),
            op::ROL_26 => op!(dir, op_rol, 2, 7 - 2 * m + dl_zero),
            op::ROL_2A => {
                if self.p.m {
                    let old_a = self.a as u8;
                    self.a = (self.a & 0xFF00) | self.rotate_left8(old_a) as u16;
                } else {
                    let old_a = self.a;
                    self.a = self.rotate_left16(old_a);
                }
                self.pc = self.pc.wrapping_add(1);
                2
            }
            op::ROL_2E => op!(abs, op_rol, 3, 8 - 2 * m),
            op::ROL_36 => op!(dir_x, op_rol, 2, 8 - 2 * m + dl_zero),
            op::ROL_3E => op!(abs_x, op_rol, 3, 9 - 2 * m),
            op::ROR_66 => op!(dir, op_ror, 2, 7 - 2 * m + dl_zero),
            op::ROR_6A => {
                if self.p.m {
                    let old_a = self.a as u8;
                    self.a = (self.a & 0xFF00) | self.rotate_right8(old_a) as u16;
                } else {
                    let old_a = self.a;
                    self.a = self.rotate_right16(old_a);
                }
                self.pc = self.pc.wrapping_add(1);
                2
            }
            op::ROR_6E => op!(abs, op_ror, 3, 8 - 2 * m),
            op::ROR_76 => op!(dir_x, op_ror, 2, 8 - 2 * m + dl_zero),
            op::ROR_7E => op!(abs_x, op_ror, 3, 9 - 2 * m),
            op::BCC => branch!(!self.p.c),
            op::BCS => branch!(self.p.c),
            op::BEQ => branch!(self.p.z),
            op::BMI => branch!(self.p.n),
            op::BNE => branch!(!self.p.z),
            op::BPL => branch!(!self.p.n),
            op::BRA => {
                let old_pc = self.pc;
                self.pc = self.rel8(addr, abus).0 as u16;
                if self.e && (self.pc & 0xFF00 != old_pc & 0xFF00) {
                    4
                } else {
                    3
                }
            }
            op::BVC => branch!(!self.p.v),
            op::BVS => branch!(self.p.v),
            op::BRL => {
                self.pc = self.rel16(addr, abus).0 as u16;
                4
            }
            op::JMP_4C => {
                self.pc = self.abs(addr, abus).0 as u16; // TODO: See JSR_20
                3
            }
            op::JMP_5C => {
                let jump_addr = self.long(addr, abus).0;
                self.pb = (jump_addr >> 16) as u8;
                self.pc = jump_addr as u16;
                4
            }
            op::JMP_6C => {
                self.pc = self.abs_ptr16(addr, abus).0 as u16;
                5
            }
            op::JMP_7C => {
                self.pc = self.abs_x_ptr16(addr, abus).0 as u16;
                6
            }
            op::JMP_DC => {
                let jump_addr = self.abs_ptr24(addr, abus).0;
                self.pb = (jump_addr >> 16) as u8;
                self.pc = jump_addr as u16;
                6
            }
            op::JSL => {
                let jump_addr = self.long(addr, abus).0;
                let pb = self.pb;
                self.push8(pb, abus);
                let pc = self.pc;
                self.push16(pc.wrapping_add(3), abus);
                self.pb = (jump_addr >> 16) as u8;
                self.pc = jump_addr as u16;
                8
            }
            op::JSR_20 => {
                let jump_addr = self.abs(addr, abus).0; // TODO: This shoud use db instead of pb!!
                let return_addr = self.pc.wrapping_add(2);
                self.push16(return_addr, abus);
                self.pc = jump_addr as u16;
                6
            }
            op::JSR_FC => {
                let jump_addr = self.abs_x_ptr16(addr, abus).0;
                let return_addr = self.pc.wrapping_add(2);
                self.push16(return_addr, abus);
                self.pc = jump_addr as u16;
                8
            }
            op::RTL => {
                self.pc = self.pull16(abus).wrapping_add(1);
                self.pb = self.pull8(abus);
                6
            }
            op::RTS => {
                self.pc = self.pull16(abus).wrapping_add(1);
                6
            }
            op::BRK => {
                if self.e {
                    let pc = self.pc;
                    self.push16(pc.wrapping_add(2), abus);
                    let p = self.p.value();
                    self.push8(p | P_X, abus); // B(RK)-flag is set to distinguish BRK from IRQ
                    self.pb = 0x00;
                    self.pc = abus.page_wrapping_cpu_read16(IRQBRK8);
                } else {
                    let pb = self.pb;
                    self.push8(pb, abus);
                    let pc = self.pc;
                    self.push16(pc.wrapping_add(2), abus);
                    let p = self.p.value();
                    self.push8(p, abus);
                    self.pb = 0x00;
                    self.pc = abus.page_wrapping_cpu_read16(BRK16);
                }
                self.p.i = true;
                self.p.d = false;
                8 - e
            }
            op::COP => {
                if self.e {
                    let pc = self.pc;
                    self.push16(pc.wrapping_add(2), abus);
                    let p = self.p.value();
                    self.push8(p, abus);
                    self.pb = 0x00;
                    self.pc = abus.page_wrapping_cpu_read16(COP8);
                } else {
                    let pb = self.pb;
                    self.push8(pb, abus);
                    let pc = self.pc;
                    self.push16(pc.wrapping_add(2), abus);
                    let p = self.p.value();
                    self.push8(p, abus);
                    self.pb = 0x00;
                    self.pc = abus.page_wrapping_cpu_read16(COP16);
                }
                self.p.i = true;
                self.p.d = false;
                8 - e
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
                7 - e
            }
            op::CLC => cl_se!(self.p.c, false),
            op::CLD => cl_se!(self.p.d, false),
            op::CLI => cl_se!(self.p.i, false),
            op::CLV => cl_se!(self.p.v, false),
            op::SEC => cl_se!(self.p.c, true),
            op::SED => cl_se!(self.p.d, true),
            op::SEI => cl_se!(self.p.i, true),
            op::REP => {
                let data_addr = self.imm(addr, abus);
                let mask = abus.cpu_read8(data_addr.0);
                self.p.clear_flags(mask);
                // Emulation forces M and X to 1
                if self.e {
                    self.p.m = true;
                    self.p.x = true;
                }
                self.pc = self.pc.wrapping_add(2);
                3
            }
            op::SEP => {
                let data_addr = self.imm(addr, abus);
                let mask = abus.cpu_read8(data_addr.0);
                self.p.set_flags(mask);
                if self.p.x {
                    self.x &= 0x00FF;
                    self.y &= 0x00FF;
                }
                self.pc = self.pc.wrapping_add(2);
                3
            }
            op::LDA_A1 => op!(dir_x_ptr16, op_lda, 2, 7 - m + dl_zero),
            op::LDA_A3 => op!(stack, op_lda, 2, 5 - m),
            op::LDA_A5 => op!(dir, op_lda, 2, 4 - m + dl_zero),
            op::LDA_A7 => op!(dir_ptr24, op_lda, 2, 7 - m + dl_zero),
            op::LDA_A9 => op!(imm, op_lda, 3 - self.p.m as u16, 3 - m),
            op::LDA_AD => op!(abs, op_lda, 3, 5 - m),
            op::LDA_AF => op!(long, op_lda, 4, 6 - m),
            op::LDA_B1 => op_p!(dir_ptr16_y, op_lda, 2, 7 - m + dl_zero),
            op::LDA_B2 => op!(dir_ptr16, op_lda, 2, 6 - m + dl_zero),
            op::LDA_B3 => op!(stack_ptr16_y, op_lda, 2, 8 - m),
            op::LDA_B5 => op!(dir_x, op_lda, 2, 5 - m + dl_zero),
            op::LDA_B7 => op!(dir_ptr24_y, op_lda, 2, 7 - m + dl_zero),
            op::LDA_B9 => op_p!(abs_y, op_lda, 3, 6 - m),
            op::LDA_BD => op_p!(abs_x, op_lda, 3, 6 - m),
            op::LDA_BF => op!(long_x, op_lda, 4, 6 - m),
            op::LDX_A2 => op!(imm, op_ldx, 3 - self.p.x as u16, 3 - x),
            op::LDX_A6 => op!(dir, op_ldx, 2, 4 - x + dl_zero),
            op::LDX_AE => op!(abs, op_ldx, 3, 5 - x),
            op::LDX_B6 => op!(dir_y, op_ldx, 2, 5 - x + dl_zero),
            op::LDX_BE => op_p!(abs_y, op_ldx, 3, 6 - x),
            op::LDY_A0 => op!(imm, op_ldy, 3 - self.p.x as u16, 3 - x),
            op::LDY_A4 => op!(dir, op_ldy, 2, 4 - x + dl_zero),
            op::LDY_AC => op!(abs, op_ldy, 3, 5 - x),
            op::LDY_B4 => op!(dir_x, op_ldy, 2, 5 - x + dl_zero),
            op::LDY_BC => op_p!(abs_x, op_ldy, 3, 6 - x),
            op::STA_81 => op!(dir_x_ptr16, op_sta, 2, 7 - m + dl_zero),
            op::STA_83 => op!(stack, op_sta, 2, 5 - m),
            op::STA_85 => op!(dir, op_sta, 2, 4 - m + dl_zero),
            op::STA_87 => op!(dir_ptr24, op_sta, 2, 7 - m + dl_zero),
            op::STA_8D => op!(abs, op_sta, 3, 5 - m),
            op::STA_8F => op!(long, op_sta, 4, 6 - m),
            op::STA_91 => op!(dir_ptr16_y, op_sta, 2, 7 - m + dl_zero),
            op::STA_92 => op!(dir_ptr16, op_sta, 2, 6 - m + dl_zero),
            op::STA_93 => op!(stack_ptr16_y, op_sta, 2, 8 - m),
            op::STA_95 => op!(dir_x, op_sta, 2, 5 - m + dl_zero),
            op::STA_97 => op!(dir_ptr24_y, op_sta, 2, 7 - m + dl_zero),
            op::STA_99 => op!(abs_y, op_sta, 3, 6 - m),
            op::STA_9D => op!(abs_x, op_sta, 3, 6 - m),
            op::STA_9F => op!(long_x, op_sta, 4, 6 - m),
            op::STX_86 => op!(dir, op_stx, 2, 4 - x + dl_zero),
            op::STX_8E => op!(abs, op_stx, 3, 5 - x),
            op::STX_96 => op!(dir_y, op_stx, 2, 5 - x + dl_zero),
            op::STY_84 => op!(dir, op_sty, 2, 4 - x + dl_zero),
            op::STY_8C => op!(abs, op_sty, 3, 5 - x),
            op::STY_94 => op!(dir_x, op_sty, 2, 5 - x + dl_zero),
            op::STZ_64 => op!(dir, op_stz, 2, 4 - m + dl_zero),
            op::STZ_74 => op!(dir_x, op_stz, 2, 5 - m + dl_zero),
            op::STZ_9C => op!(abs, op_stz, 3, 5 - m),
            op::STZ_9E => op!(abs_x, op_stz, 3, 6 - m),
            op::MVN => op!(src_dest, op_mvn, 0, 7), // Op "loops" until move is complete
            op::MVP => op!(src_dest, op_mvp, 0, 7), // Op "loops" until move is complete
            op::NOP => {
                self.pc = self.pc.wrapping_add(1);
                2
            }
            op::WDM => {
                self.pc = self.pc.wrapping_add(2);
                2
            }
            op::PEA => push_eff!(imm, 3, 5),
            op::PEI => push_eff!(dir, 2, 6 + dl_zero),
            op::PER => push_eff!(imm, 3, 6),
            op::PHA => push_reg!(self.p.m, self.a, 4 - m),
            op::PHX => push_reg!(self.p.x, self.x, 4 - x),
            op::PHY => push_reg!(self.p.x, self.y, 4 - x),
            op::PLA => pull_reg!(self.p.m, self.a, 5 - m),
            op::PLX => pull_reg!(self.p.x, self.x, 5 - x),
            op::PLY => pull_reg!(self.p.x, self.y, 5 - x),
            op::PHB => push_reg!(true, self.db, 3),
            op::PHD => push_reg!(false, self.d, 4),
            op::PHK => push_reg!(true, self.pb, 3),
            op::PHP => push_reg!(true, self.p.value(), 3),
            op::PLB => {
                let value = self.pull8(abus);
                self.db = value;
                self.p.n = value > 0x7F;
                self.p.z = value == 0;
                self.pc = self.pc.wrapping_add(1);
                4
            }
            op::PLD => pull_reg!(false, self.d, 5),
            op::PLP => {
                let value = self.pull8(abus);
                self.p.set_value(value);
                self.pc = self.pc.wrapping_add(1);
                4
            }
            op::STP => {
                self.stopped = true;
                3
            }
            op::WAI => {
                self.waiting = true;
                3
            }
            op::TAX => transfer!(self.p.x, self.a, self.x),
            op::TAY => transfer!(self.p.x, self.a, self.y),
            op::TSX => transfer!(self.p.x, self.s, self.x),
            op::TXA => transfer!(self.p.m, self.x, self.a),
            op::TXS => {
                self.s = if self.e {
                    0x0100 | (self.x & 0x00FF)
                } else {
                    self.x
                };
                self.pc = self.pc.wrapping_add(1);
                2
            }
            op::TXY => transfer!(self.p.x, self.x, self.y),
            op::TYA => transfer!(self.p.m, self.y, self.a),
            op::TYX => transfer!(self.p.x, self.y, self.x),
            op::TCD => transfer!(false, self.a, self.d),
            op::TCS => {
                self.s = if self.e {
                    0x0100 | (self.a & 0x00FF)
                } else {
                    self.a
                };
                self.pc = self.pc.wrapping_add(1);
                2
            }
            op::TDC => transfer!(false, self.d, self.a),
            op::TSC => transfer!(false, self.s, self.a),
            op::XBA => {
                self.a = (self.a << 8) | (self.a >> 8);
                self.p.n = self.a as u8 > 0x7F;
                self.p.z = self.a as u8 > 0;
                self.pc = self.pc.wrapping_add(1);
                3
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
                2
            }
        }
    }

    // Addressing modes
    // TODO: DRY, mismatch funcs and macros?

    /// Returns the address and wrapping of data the instruction at `addr` points to using absolute
    /// mode
    ///
    /// Data is at lo`[$DBHHLL]` hi`[$DBHHLL+1]`. Note that JMP and JSR should use
    /// `PB` instead of `DB`!
    pub fn abs(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        self.abs_common(abus.fetch_operand16(addr))
    }
    pub fn peek_abs(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        self.abs_common(abus.peek_operand16(addr))
    }
    fn abs_common(&self, db_addr: u16) -> (u32, WrappingMode) {
        (addr_8_16(self.db, db_addr), WrappingMode::AddrSpace)
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// absolute,X mode
    ///
    /// Data is at lo`[$DBHHLL+X]` hi`[$DBHHLL+X+1]`
    pub fn abs_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        self.abs_n_common(abus.fetch_operand16(addr), self.x)
    }
    pub fn peek_abs_x(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        self.abs_n_common(abus.peek_operand16(addr), self.x)
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// absolute,Y mode
    ///
    /// Data is at lo`[$DBHHLL+Y]` hi`[$DBHHLL+Y+1]`
    pub fn abs_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        self.abs_n_common(abus.fetch_operand16(addr), self.y)
    }
    pub fn peek_abs_y(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        self.abs_n_common(abus.peek_operand16(addr), self.y)
    }

    fn abs_n_common(&self, db_addr: u16, n_reg: u16) -> (u32, WrappingMode) {
        (
            (addr_8_16(self.db, db_addr) + n_reg as u32) & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// (absolute) mode
    ///
    /// 16bit pointer at lo`[$00][$HHLL]` hi`[$00][$HHLL+1]` with actual data at `[$PBhilo]`
    pub fn abs_ptr16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = abus.fetch_operand16(addr) as u32;
        (
            addr_8_16(self.pb, abus.bank_wrapping_cpu_read16(pointer)),
            WrappingMode::Bank,
        )
    }
    pub fn peek_abs_ptr16(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        let pointer = abus.peek_operand16(addr) as u32;
        (
            addr_8_16(self.pb, abus.bank_wrapping_cpu_peek16(pointer)),
            WrappingMode::Bank,
        )
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// \[absolute\] mode
    ///
    /// 24bit pointer at lo`[$00][$HHLL]` mid`[$00][$HHLL+1]` hi`[$00][$HHLL+2]` with actual data at
    /// `[$himidlo]`
    pub fn abs_ptr24(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = abus.fetch_operand24(addr) as u32;
        (abus.bank_wrapping_cpu_read24(pointer), WrappingMode::Bank)
    }
    pub fn peek_abs_ptr24(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        let pointer = abus.peek_operand24(addr) as u32;
        (abus.bank_wrapping_cpu_peek24(pointer), WrappingMode::Bank)
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// (absolute,X) mode
    ///
    /// 16bit pointer at lo`[$00][$HHLL+X]` hi`[$00][$HHLL+X+1]` with actual data at `[$PBhilo]`
    pub fn abs_x_ptr16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = addr_8_16(self.pb, abus.fetch_operand16(addr).wrapping_add(self.x));
        (
            addr_8_16(self.pb, abus.bank_wrapping_cpu_read16(pointer)),
            WrappingMode::Bank,
        )
    }
    pub fn peek_abs_x_ptr16(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        let pointer = addr_8_16(self.pb, abus.peek_operand16(addr).wrapping_add(self.x));
        (
            addr_8_16(self.pb, abus.bank_wrapping_cpu_peek16(pointer)),
            WrappingMode::Bank,
        )
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// direct mode
    ///
    /// Data at `[$00][$DL][$LL]` for "old" instructions if in emulation mode and `DL` is `$00`,
    /// otherwise lo`[$00][$D+LL]` hi`[$00][$D+LL+1]`. Math turns out to be the same for both.
    pub fn dir(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        self.dir_common(abus.fetch_operand8(addr) as u16)
    }
    pub fn peek_dir(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        self.dir_common(abus.peek_operand8(addr) as u16)
    }
    pub fn dir_common(&self, offset: u16) -> (u32, WrappingMode) {
        (
            self.d.wrapping_add(offset as u16) as u32,
            WrappingMode::Bank,
        )
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// direct,X mode
    ///
    /// Data at `[$00][$DL][$LL+X]` if in emulation mode and `DL` is `$00`, lo`[$00][$D+LL+X]`
    /// otherwise hi`[$00][$D+LL+X+1]`.
    pub fn dir_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        // TODO: Combine with dir_y
        if self.e && (self.d & 0xFF) == 0 {
            (
                (self.d | (abus.fetch_operand8(addr).wrapping_add(self.x as u8) as u16)) as u32,
                WrappingMode::Page,
            )
        } else {
            (
                self.d
                    .wrapping_add(abus.fetch_operand8(addr) as u16)
                    .wrapping_add(self.x) as u32,
                WrappingMode::Bank,
            )
        }
    }

    pub fn peek_dir_x(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        // TODO: Combine with dir_y
        if self.e && (self.d & 0xFF) == 0 {
            (
                (self.d | (abus.peek_operand8(addr).wrapping_add(self.x as u8) as u16)) as u32,
                WrappingMode::Page,
            )
        } else {
            (
                self.d
                    .wrapping_add(abus.peek_operand8(addr) as u16)
                    .wrapping_add(self.x) as u32,
                WrappingMode::Bank,
            )
        }
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// direct,Y mode
    ///
    /// Data at `[$00][$DL][$LL+Y]` if in emulation mode and `DL` is `$00`, lo`[$00][$D+LL+Y]`
    /// otherwise hi`[$00][$D+LL+Y+1]`
    pub fn dir_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        // TODO: Combine with dir_x
        if self.e && (self.d & 0xFF) == 0 {
            (
                (self.d | (abus.fetch_operand8(addr).wrapping_add(self.y as u8) as u16)) as u32,
                WrappingMode::Page,
            )
        } else {
            (
                self.d
                    .wrapping_add(abus.fetch_operand8(addr) as u16)
                    .wrapping_add(self.y) as u32,
                WrappingMode::Bank,
            )
        }
    }
    pub fn peek_dir_y(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        // TODO: Combine with dir_x
        if self.e && (self.d & 0xFF) == 0 {
            (
                (self.d | (abus.peek_operand8(addr).wrapping_add(self.y as u8) as u16)) as u32,
                WrappingMode::Page,
            )
        } else {
            (
                self.d
                    .wrapping_add(abus.peek_operand8(addr) as u16)
                    .wrapping_add(self.y) as u32,
                WrappingMode::Bank,
            )
        }
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// (direct) mode
    ///
    /// 16bit pointer at lo`[$00][$DH][$LL]` hi`[$00][$DH][$LL+1]` if in emulation mode and `DL` is
    /// `$00`, otherwise lo`[$00][$D+LL]` hi`[$00][$D+LL+1]`. Data at lo`[$DBhilo]` hi`[$DBhilo+1]`.
    pub fn dir_ptr16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.dir(addr, abus).0;
        if self.e && (self.d & 0xFF) == 0 {
            let ll = abus.cpu_read8(pointer);
            let hh = abus.cpu_read8((pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32);
            (addr_8_8_8(self.db, hh, ll), WrappingMode::AddrSpace)
        } else {
            (
                addr_8_16(self.db, abus.bank_wrapping_cpu_read16(pointer)),
                WrappingMode::AddrSpace,
            )
        }
    }
    pub fn peek_dir_ptr16(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        let pointer = self.peek_dir(addr, abus).0;
        if self.e && (self.d & 0xFF) == 0 {
            let ll = abus.cpu_peek8(pointer);
            let hh = abus.cpu_peek8((pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32);
            (addr_8_8_8(self.db, hh, ll), WrappingMode::AddrSpace)
        } else {
            (
                addr_8_16(self.db, abus.bank_wrapping_cpu_peek16(pointer)),
                WrappingMode::AddrSpace,
            )
        }
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// \[direct\] mode
    ///
    /// 16bit pointer at lo`[$00][$DH][$LL]` mid`[$00][$DH][$LL+1]` hi`[$00][$DH][$LL+2]` if in
    /// emulation mode and `DL` is `$00`, otherwise lo`[$00][$D+LL]` mid`[$00][$D+LL+1]`
    /// hi`[$00][$D+LL+2]`. Data at lo`[$himidlo]` hi`[$himidlo+1]`.
    pub fn dir_ptr24(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.dir(addr, abus).0;
        (
            abus.bank_wrapping_cpu_read24(pointer),
            WrappingMode::AddrSpace,
        )
    }
    pub fn peek_dir_ptr24(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        let pointer = self.peek_dir(addr, abus).0;
        (
            abus.bank_wrapping_cpu_peek24(pointer),
            WrappingMode::AddrSpace,
        )
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// (direct,X) mode
    ///
    /// 16bit pointer at lo`[$00][$DH][$LL+X]` hi`[$00][$DH][$LL+X+1]` if in emulation mode and
    /// `DL` is `$00`, otherwise lo`[$00][$D+LL+X]` hi`[$00][$D+LL+X+1]`. Data at lo`[$DBhilo]`
    /// hi`[$DBhilo+1]`.
    pub fn dir_x_ptr16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.dir_x(addr, abus).0;
        if self.e && (self.d & 0xFF) == 0 {
            let ll = abus.cpu_read8(pointer);
            let hh = abus.cpu_read8((pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32);
            (addr_8_8_8(self.db, hh, ll), WrappingMode::AddrSpace)
        } else {
            (
                addr_8_16(self.db, abus.bank_wrapping_cpu_read16(pointer)),
                WrappingMode::AddrSpace,
            )
        }
    }
    pub fn peek_dir_x_ptr16(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        let pointer = self.peek_dir_x(addr, abus).0;
        if self.e && (self.d & 0xFF) == 0 {
            let ll = abus.cpu_peek8(pointer);
            let hh = abus.cpu_peek8((pointer & 0xFF00) | (pointer as u8).wrapping_add(1) as u32);
            (addr_8_8_8(self.db, hh, ll), WrappingMode::AddrSpace)
        } else {
            (
                addr_8_16(self.db, abus.bank_wrapping_cpu_peek16(pointer)),
                WrappingMode::AddrSpace,
            )
        }
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// (direct),Y mode
    ///
    /// 16bit pointer at lo`[$00][$DH][$LL]` hi`[$00][$DH][$LL+1]` if in emulation mode and
    /// `DL` is `$00`, otherwise lo`[$00][$D+LL]` hi`[$00][$D+LL+1]`. Data at lo`[$DBhilo+Y]`
    /// hi`[$DBhilo+Y+1]`.
    pub fn dir_ptr16_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        self.dir_ptr16_y_common(self.dir_ptr16(addr, abus).0)
    }
    pub fn peek_dir_ptr16_y(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        self.dir_ptr16_y_common(self.peek_dir_ptr16(addr, abus).0)
    }
    fn dir_ptr16_y_common(&self, pointer: u32) -> (u32, WrappingMode) {
        (
            (pointer + self.y as u32) & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// \[direct\],Y mode
    ///
    /// 16bit pointer at lo`[$00][$DH][$LL]` mid`[$00][$DH][$LL+1]` hi`[$00][$DH][$LL+2]`. Data at
    /// lo`[$himidlo+Y]` hi`[$himidlo+Y+1]`.
    pub fn dir_ptr24_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        self.dir_ptr24_y_common(self.dir_ptr24(addr, abus).0)
    }
    pub fn peek_dir_ptr24_y(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        self.dir_ptr24_y_common(self.peek_dir_ptr24(addr, abus).0)
    }
    fn dir_ptr24_y_common(&self, pointer: u32) -> (u32, WrappingMode) {
        (
            (pointer + self.y as u32) & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// immediate mode
    ///
    /// Data is lo`$LL` hi`$HH`
    #[allow(unused_variables)]
    pub fn imm(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (
            addr_8_16(self.pb, self.pc.wrapping_add(1)),
            WrappingMode::Bank,
        )
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// long mode
    ///
    /// Data at lo`[$HHMMLL]` hi`[$HHMMLL+1]`
    pub fn long(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        (abus.fetch_operand24(addr), WrappingMode::AddrSpace)
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// long,X mode
    ///
    /// Data at lo`[$HHMMLL+X]` hi`[$HHMMLL+X+1]`
    pub fn long_x(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        self.long_x_common(abus.fetch_operand24(addr))
    }
    pub fn peek_long_x(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        self.long_x_common(abus.peek_operand24(addr))
    }
    fn long_x_common(&self, pointer: u32) -> (u32, WrappingMode) {
        (
            (pointer + self.x as u32) & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// relative8 mode
    ///
    /// Data at `[$PB][$PC+2+LL]`, where `LL` is treated as a signed integer
    pub fn rel8(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        self.rel_8_common(abus.fetch_operand8(addr) as u16)
    }
    pub fn peek_rel8(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        self.rel_8_common(abus.peek_operand8(addr) as u16)
    }
    fn rel_8_common(&self, offset: u16) -> (u32, WrappingMode) {
        if offset < 0x80 {
            (
                addr_8_16(self.pb, self.pc.wrapping_add(2 + offset)),
                WrappingMode::Bank,
            )
        } else {
            (
                addr_8_16(self.pb, self.pc.wrapping_sub(254).wrapping_add(offset)),
                WrappingMode::Bank,
            )
        }
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// relative16 mode
    ///
    /// Data at `[$PB][$PC+3+HHLL]`
    pub fn rel16(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        self.rel16_common(abus.fetch_operand16(addr) as u16)
    }
    pub fn peek_rel16(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        self.rel16_common(abus.peek_operand16(addr) as u16)
    }
    fn rel16_common(&self, offset: u16) -> (u32, WrappingMode) {
        (
            addr_8_16(self.pb, self.pc.wrapping_add(3).wrapping_add(offset)),
            WrappingMode::Bank,
        )
    }

    /// Returns the addresses the move instruction at `addr` points to
    ///
    /// Source data is at `[$HHX]` and destination at `[$LLY]`
    pub fn src_dest(&self, addr: u32, abus: &mut ABus) -> (u32, u32) {
        let operand = abus.fetch_operand16(addr);
        (
            addr_8_16((operand >> 8) as u8, self.x),
            addr_8_16(operand as u8, self.y),
        )
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// stack,s mode
    ///
    /// Data at lo`[$00][$LL+S]` hi`[$00][$LL+S+1]`
    pub fn stack(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        self.stack_common(abus.fetch_operand8(addr) as u16)
    }
    pub fn peek_stack(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        self.stack_common(abus.peek_operand8(addr) as u16)
    }
    fn stack_common(&self, offset: u16) -> (u32, WrappingMode) {
        (self.s.wrapping_add(offset) as u32, WrappingMode::Bank)
    }

    /// Returns the address and wrapping of data the instruction at `addr` points to using
    /// (stack,s),Y mode
    ///
    /// 16bit pointer at lo`[$00][$LL+S]` hi`[$00][$LL+S+1]`. Data at lo`[$DBhilo+Y]`
    /// hi`[$DBhilo+Y+1]`.
    pub fn stack_ptr16_y(&self, addr: u32, abus: &mut ABus) -> (u32, WrappingMode) {
        let pointer = self.s.wrapping_add(abus.fetch_operand8(addr) as u16) as u32;
        (
            (addr_8_16(self.db, abus.bank_wrapping_cpu_read16(pointer)) + self.y as u32)
                & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }
    pub fn peek_stack_ptr16_y(&self, addr: u32, abus: &ABus) -> (u32, WrappingMode) {
        let pointer = self.s.wrapping_add(abus.peek_operand8(addr) as u16) as u32;
        (
            (addr_8_16(self.db, abus.bank_wrapping_cpu_peek16(pointer)) + self.y as u32)
                & 0x00FFFFFF,
            WrappingMode::AddrSpace,
        )
    }

    /// Pushes `value` to stack, incrementing the stack pointer accordingly
    fn push8(&mut self, value: u8, abus: &mut ABus) {
        abus.cpu_write8(self.s as u32, value);
        self.decrement_s(1);
    }

    /// Pushes `value` to stack, incrementing the stack pointer accordingly
    fn push16(&mut self, value: u16, abus: &mut ABus) {
        self.decrement_s(1);
        if self.e {
            abus.page_wrapping_cpu_write16(self.s as u32, value);
        } else {
            abus.bank_wrapping_cpu_write16(self.s as u32, value);
        }
        self.decrement_s(1);
    }

    /// Pushes lowest three bytes of `value` to stack, incrementing the stack pointer accordingly
    #[allow(dead_code)]
    fn push24(&mut self, value: u32, abus: &mut ABus) {
        self.decrement_s(2);
        if self.e {
            abus.page_wrapping_cpu_write24(self.s as u32, value);
        } else {
            abus.bank_wrapping_cpu_write24(self.s as u32, value);
        }
        self.decrement_s(1);
    }

    /// Pulls a byte from stack, decrementing the stack pointer accordingly
    fn pull8(&mut self, abus: &mut ABus) -> u8 {
        self.increment_s(1);
        abus.cpu_read8(self.s as u32)
    }

    /// Pulls two bytes from stack, decrementing the stack pointer accordingly
    fn pull16(&mut self, abus: &mut ABus) -> u16 {
        self.increment_s(1);
        let value = if self.e {
            abus.page_wrapping_cpu_read16(self.s as u32)
        } else {
            abus.bank_wrapping_cpu_read16(self.s as u32)
        };
        self.increment_s(1);
        value
    }

    /// Pulls three bytes from stack, decrementing the stack pointer accordingly
    #[allow(dead_code)]
    fn pull24(&mut self, abus: &mut ABus) -> u32 {
        self.increment_s(1);
        let value = if self.e {
            abus.page_wrapping_cpu_read24(self.s as u32)
        } else {
            abus.bank_wrapping_cpu_read24(self.s as u32)
        };
        self.increment_s(2);
        value
    }

    /// Decrements the stack pointer, wrapping controlled by emulation flag
    fn decrement_s(&mut self, offset: u8) {
        if self.e {
            self.s = 0x0100 | (self.s as u8).wrapping_sub(offset) as u16;
        } else {
            self.s = self.s.wrapping_sub(offset as u16);
        }
    }

    /// Increments the stack pointer, wrapping controlled by emulation flag
    fn increment_s(&mut self, offset: u8) {
        if self.e {
            self.s = 0x0100 | (self.s as u8).wrapping_add(offset) as u16;
        } else {
            self.s = self.s.wrapping_add(offset as u16);
        }
    }

    /// Sets emulation mode on
    ///
    /// Accumulator and index registers are forced 8bit wide and stack to page `$01`
    fn set_emumode(&mut self) {
        self.e = true;
        self.p.m = true;
        self.p.x = true;
        self.x &= 0x00FF;
        self.y &= 0x00FF;
        self.s = 0x0100 | (self.s & 0x00FF);
    }

    // Algorithms

    /// Returns the result of `lhs + rhs + P.C` or `lhs - rhs - 1 + P.C` depending on
    /// `subtraction`
    ///
    /// `P.D` determines if BCD arithmetic should be used. `P.C` indicates an unsigned carry,
    /// `P.V` indicates a signed (binary mode) overflow, `P.N` reflects the high bit of the result
    /// and `P.Z` is set if the result is zero.
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

    /// Returns the result of `lhs + rhs + P.C` or `lhs - rhs - 1 + P.C` depending on
    /// `subtraction`
    ///
    /// `P.D` determines if BCD arithmetic should be used. `P.C` indicates an unsigned carry,
    /// `P.V` indicates a signed (binary mode) overflow, `P.N` reflects the high bit of the result
    /// and `P.Z` is set if the result is zero.
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

    /// Compares the parameters with result reflected in status flags
    ///
    /// Performs binary subtraction without carry. `P.C` indicates carry, `P.N` reflects the high bit
    /// of the result and `P.C` wheter or not the result is zero.
    fn compare8(&mut self, lhs: u8, rhs: u8) {
        let result: u16 = lhs as u16 + !rhs as u16 + 1;
        self.p.n = result as u8 > 0x7F;
        self.p.z = result as u8 == 0;
        self.p.c = result > 0xFF;
    }

    /// Compares the parameters with result reflected in status flags
    ///
    /// Performs binary subtraction without carry. `P.C` indicates carry, `P.N` reflects the high bit
    /// of the result and `P.C` wheter or not the result is zero.
    fn compare16(&mut self, lhs: u16, rhs: u16) {
        let result: u32 = lhs as u32 + !rhs as u32 + 1;
        self.p.n = result as u16 > 0x7FFF;
        self.p.z = result as u16 == 0;
        self.p.c = result > 0xFFFF;
    }

    /// Returns the data shifted left by one
    ///
    /// High bit is shifted to `P.C`, `P.N` reflects the high bit of the result and `P.Z` whether or not
    /// the result is zero
    fn arithmetic_shift_left8(&mut self, data: u8) -> u8 {
        self.p.c = data & 0x80 > 0;
        let result = data << 1;
        self.p.n = result > 0x7F;
        self.p.z = result == 0;
        result
    }

    /// Returns the data shifted left by one
    ///
    /// High bit is shifted to `P.C`, `P.N` reflects the high bit of the result and `P.Z` whether or not
    /// the result is zero
    fn arithmetic_shift_left16(&mut self, data: u16) -> u16 {
        self.p.c = data & 0x8000 > 0;
        let result = data << 1;
        self.p.n = result > 0x7FFF;
        self.p.z = result == 0;
        result
    }

    /// Returns the data shifted right by one
    ///
    /// Low bit is shifted to `P.C`, `P.N` reflects the high bit of the result and `P.Z` whether or not
    /// the result is zero
    fn logical_shift_right8(&mut self, data: u8) -> u8 {
        self.p.c = data & 0x01 > 0;
        let result = data >> 1;
        self.p.n = result > 0x7F;
        self.p.z = result == 0;
        result
    }

    /// Returns the data shifted right by one
    ///
    /// Low bit is shifted to `P.C`, `P.N` reflects the high bit of the result and `P.Z` whether or not
    /// the result is zero
    fn logical_shift_right16(&mut self, data: u16) -> u16 {
        self.p.c = data & 0x0001 > 0;
        let result = data >> 1;
        self.p.n = result > 0x7FFF;
        self.p.z = result == 0;
        result
    }

    /// Returns the data rotated left by one with `P.C` shifted to the low bit
    ///
    /// High bit is shifted to `P.C`, `P.N` reflects the high bit of the result and `P.Z` whether or not
    /// the result is zero
    fn rotate_left8(&mut self, data: u8) -> u8 {
        let c = data & 0x80 > 0;
        let result = (data << 1) | self.p.c as u8;
        self.p.c = c;
        self.p.n = result > 0x7F;
        self.p.z = result == 0;
        result
    }

    /// Returns the data rotated left by one with `P.C` shifted to the low bit
    ///
    /// High bit is shifted to `P.C`, `P.N` reflects the high bit of the result and `P.Z` whether or not
    /// the result is zero
    fn rotate_left16(&mut self, data: u16) -> u16 {
        let c = data & 0x8000 > 0;
        let result = (data << 1) | self.p.c as u16;
        self.p.c = c;
        self.p.n = result > 0x7FFF;
        self.p.z = result == 0;
        result
    }

    /// Returns the data rotated right by one with `P.C` shifted to the high bit
    ///
    /// Low bit is shifted to `P.C`, `P.N` reflects the high bit of the result and `P.Z` whether or not
    /// the result is zero
    fn rotate_right8(&mut self, data: u8) -> u8 {
        let c = data & 0x01 > 0;
        let result = (data >> 1) | ((self.p.c as u8) << 7);
        self.p.c = c;
        self.p.n = result > 0x7F;
        self.p.z = result == 0;
        result
    }

    /// Returns the data rotated right by one with `P.C` shifted to the high bit
    ///
    /// Low bit is shifted to `P.C`, `P.N` reflects the high bit of the result and `P.Z` whether or not
    /// the result is zero
    fn rotate_right16(&mut self, data: u16) -> u16 {
        let c = data & 0x0001 > 0;
        let result = (data >> 1) | ((self.p.c as u16) << 15);
        self.p.c = c;
        self.p.n = result > 0x7FFF;
        self.p.z = result == 0;
        result
    }

    // Instructions

    /// Adds the given data to `A`
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    /// See [`add_sub8`] and [`add_sub16`] for further details.
    ///
    /// [`add_sub8`]: #method.add_sub8
    /// [`add_sub16`]: #method.add_sub16
    fn op_adc(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let acc8 = self.a as u8;
            let data = abus.cpu_read8(data_addr.0);
            self.a = (self.a & 0xFF00) | (self.add_sub8(acc8, data, false) as u16);
        } else {
            // 16-bit accumulator
            let acc16 = self.a;
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.a = self.add_sub16(acc16, data, false);
        }
    }

    /// Subtracts the given data from `A`
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    /// See [`add_sub8`] and [`add_sub16`] for further details.
    ///
    /// [`add_sub8`]: #method.add_sub8
    /// [`add_sub16`]: #method.add_sub16
    fn op_sbc(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let acc8 = self.a as u8;
            let data = abus.cpu_read8(data_addr.0);
            self.a = (self.a & 0xFF00) | (self.add_sub8(acc8, data, true) as u16);
        } else {
            // 16-bit accumulator
            let acc16 = self.a;
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.a = self.add_sub16(acc16, data, true);
        }
    }

    /// Compares `A` to the given data
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// See [`compare8`] and [`compare16`] for further details.
    ///
    /// [`compare8`]: #method.compare8
    /// [`compare16`]: #method.compare16
    fn op_cmp(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let acc8 = self.a as u8;
            let data = abus.cpu_read8(data_addr.0);
            self.compare8(acc8, data);
        } else {
            // 16-bit accumulator
            let acc16 = self.a;
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.compare16(acc16, data);
        }
    }

    /// Compares `X` to the given data
    ///
    /// 8bit if `P.X` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// See [`compare8`] and [`compare16`] for further details.
    ///
    /// [`compare8`]: #method.compare8
    /// [`compare16`]: #method.compare16
    fn op_cpx(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            // 8-bit X register
            let x8 = self.x as u8;
            let data = abus.cpu_read8(data_addr.0);
            self.compare8(x8, data);
        } else {
            // 16-bit X register
            let x16 = self.x;
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.compare16(x16, data);
        }
    }

    /// Compares `Y` to the given data
    ///
    /// 8bit if `P.X` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// See [`compare8`] and [`compare16`] for further details.
    ///
    /// [`compare8`]: #method.compare8
    /// [`compare16`]: #method.compare16
    fn op_cpy(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            // 8-bit Y register
            let y8 = self.y as u8;
            let data = abus.cpu_read8(data_addr.0);
            self.compare8(y8, data);
        } else {
            // 16-bit Y register
            let y16 = self.y;
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.compare16(y16, data);
        }
    }

    /// Decrements the data at given address
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// `P.N` reflects the high bit of the result and `P.Z` whether or not it is zero.
    fn op_dec(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0).wrapping_sub(1);
            self.p.n = data > 0x7F;
            self.p.z = data == 0;
            abus.cpu_write8(data_addr.0, data);
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => {
                    data = abus.bank_wrapping_cpu_read16(data_addr.0).wrapping_sub(1);
                    abus.bank_wrapping_cpu_write16(data_addr.0, data);
                }
                WrappingMode::AddrSpace => {
                    data = abus.addr_wrapping_cpu_read16(data_addr.0).wrapping_sub(1);
                    abus.addr_wrapping_cpu_write16(data_addr.0, data);
                }
                WrappingMode::Page => unreachable!(),
            }
            self.p.n = data > 0x7FFF;
            self.p.z = data == 0;
        }
    }

    /// Increments the data at given address
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// `P.N` reflects the high bit of the result and `P.Z` whether or not it is zero.
    fn op_inc(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0).wrapping_add(1);
            self.p.n = data > 0x7F;
            self.p.z = data == 0;
            abus.cpu_write8(data_addr.0, data);
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => {
                    data = abus.bank_wrapping_cpu_read16(data_addr.0).wrapping_add(1);
                    abus.bank_wrapping_cpu_write16(data_addr.0, data);
                }
                WrappingMode::AddrSpace => {
                    data = abus.addr_wrapping_cpu_read16(data_addr.0).wrapping_add(1);
                    abus.addr_wrapping_cpu_write16(data_addr.0, data);
                }
                WrappingMode::Page => unreachable!(),
            }
            self.p.n = data > 0x7FFF;
            self.p.z = data == 0;
        }
    }

    /// Performs bitwise AND of `A` and the data, storing the result in `A`
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// `P.N` reflects the high bit of the result and `P.Z` whether or not it is zero.
    fn op_and(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0);
            self.a = (self.a & 0xFF00) | (self.a as u8 & data) as u16;
            self.p.n = self.a as u8 > 0x7F;
            self.p.z = self.a as u8 == 0;
        } else {
            // 16-bit accumulator
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.a &= data;
            self.p.n = self.a > 0x7FFF;
            self.p.z = self.a == 0;
        }
    }

    /// Performs bitwise XOR of `A` and the data, storing the result in `A`
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// `P.N` reflects the high bit of the result and `P.Z` whether or not it is zero.
    fn op_eor(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0);
            self.a = (self.a & 0xFF00) | (self.a as u8 ^ data) as u16;
            self.p.n = self.a as u8 > 0x7F;
            self.p.z = self.a as u8 == 0;
        } else {
            // 16-bit accumulator
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.a ^= data;
            self.p.n = self.a > 0x7FFF;
            self.p.z = self.a == 0;
        }
    }

    /// Performs bitwise OR of `A` and the data, storing the result in `A`
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// `P.N` reflects the high bit of the result and `P.Z` whether or not it is zero.
    fn op_ora(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0);
            self.a = (self.a & 0xFF00) | (self.a as u8 | data) as u16;
            self.p.n = self.a as u8 > 0x7F;
            self.p.z = self.a as u8 == 0;
        } else {
            // 16-bit accumulator
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.a |= data;
            self.p.n = self.a > 0x7FFF;
            self.p.z = self.a == 0;
        }
    }

    /// Performs bitwise AND of `A` and the data only affecting flags
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// `P.N` reflects the high bit of the result, `P.V` the second highest bit and `P.Z`
    /// whether or not the result is zero.
    fn op_bit(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0);
            let result = self.a as u8 & data;
            self.p.n = data > 0x7F;
            self.p.v = data & 0x40 > 0;
            self.p.z = result == 0;
        } else {
            // 16-bit accumulator
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            let result = self.a & data;
            self.p.n = data > 0x7FFF;
            self.p.v = data & 0x4000 > 0;
            self.p.z = result == 0;
        }
    }

    /// Clears the bits in data that are ones in `A`
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// `P.Z` indicates whether or not the result is zero.
    fn op_trb(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0);
            abus.cpu_write8(data_addr.0, data & !(self.a as u8));
            self.p.z = self.a as u8 & data == 0;
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => {
                    data = abus.bank_wrapping_cpu_read16(data_addr.0);
                    abus.bank_wrapping_cpu_write16(data_addr.0, data & !self.a)
                }
                WrappingMode::AddrSpace => {
                    data = abus.addr_wrapping_cpu_read16(data_addr.0);
                    abus.addr_wrapping_cpu_write16(data_addr.0, data & !self.a)
                }
                WrappingMode::Page => unreachable!(),
            }
            self.p.z = self.a & data == 0;
        }
    }

    /// Sets the bits in data that are ones in `A`
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`, data is accessed according to WrappingMode.
    /// `P.Z` indicates whether or not the result is zero.
    fn op_tsb(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0);
            abus.cpu_write8(data_addr.0, data | (self.a as u8));
            self.p.z = self.a as u8 & data == 0;
        } else {
            // 16-bit accumulator
            let data: u16;
            match data_addr.1 {
                WrappingMode::Bank => {
                    data = abus.bank_wrapping_cpu_read16(data_addr.0);
                    abus.bank_wrapping_cpu_write16(data_addr.0, data | self.a)
                }
                WrappingMode::AddrSpace => {
                    data = abus.addr_wrapping_cpu_read16(data_addr.0);
                    abus.addr_wrapping_cpu_write16(data_addr.0, data | self.a)
                }
                WrappingMode::Page => unreachable!(),
            }
            self.p.z = self.a & data == 0;
        }
    }

    /// Shifts the data left by 1
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    /// See [`arithmetic_shift_left8`] and [`arithmetic_shift_left16`] for further details.
    ///
    /// [`arithmetic_shift_left8`]: #method.arithmetic_shift_left8
    /// [`arithmetic_shift_left16`]: #method.arithmetic_shift_left16
    fn op_asl(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0);
            abus.cpu_write8(data_addr.0, self.arithmetic_shift_left8(data));
        } else {
            // 16-bit accumulator
            match data_addr.1 {
                WrappingMode::Bank => {
                    let data = abus.bank_wrapping_cpu_read16(data_addr.0);
                    abus.bank_wrapping_cpu_write16(data_addr.0, self.arithmetic_shift_left16(data))
                }
                WrappingMode::AddrSpace => {
                    let data = abus.addr_wrapping_cpu_read16(data_addr.0);
                    abus.addr_wrapping_cpu_write16(data_addr.0, self.arithmetic_shift_left16(data))
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    /// Shifts the data right by 1
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    /// See [`logical_shift_right8`] and [`logical_shift_right16`] for further details.
    ///
    /// [`logical_shift_right8`]: #method.logical_shift_right8
    /// [`logical_shift_right16`]: #method.logical_shift_right16
    fn op_lsr(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0);
            abus.cpu_write8(data_addr.0, self.logical_shift_right8(data));
        } else {
            // 16-bit accumulator
            match data_addr.1 {
                WrappingMode::Bank => {
                    let data = abus.bank_wrapping_cpu_read16(data_addr.0);
                    abus.bank_wrapping_cpu_write16(data_addr.0, self.logical_shift_right16(data))
                }
                WrappingMode::AddrSpace => {
                    let data = abus.addr_wrapping_cpu_read16(data_addr.0);
                    abus.addr_wrapping_cpu_write16(data_addr.0, self.logical_shift_right16(data))
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    /// Rotates the data left by 1
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    /// See [`rotate_left8`] and [`rotate_left16`] for further details.
    ///
    /// [`rotate_left8`]: #method.rotate_left8
    /// [`rotate_left16`]: #method.rotate_left16
    fn op_rol(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0);
            abus.cpu_write8(data_addr.0, self.rotate_left8(data));
        } else {
            // 16-bit accumulator
            match data_addr.1 {
                WrappingMode::Bank => {
                    let data = abus.bank_wrapping_cpu_read16(data_addr.0);
                    abus.bank_wrapping_cpu_write16(data_addr.0, self.rotate_left16(data))
                }
                WrappingMode::AddrSpace => {
                    let data = abus.addr_wrapping_cpu_read16(data_addr.0);
                    abus.addr_wrapping_cpu_write16(data_addr.0, self.rotate_left16(data))
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    /// Rotates the data right by 1
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    /// See [`rotate_right8`] and [`rotate_right16`] for further details.
    ///
    /// [`rotate_right8`]: #method.rotate_right8
    /// [`rotate_right16`]: #method.rotate_right16
    fn op_ror(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            // 8-bit accumulator
            let data = abus.cpu_read8(data_addr.0);
            abus.cpu_write8(data_addr.0, self.rotate_right8(data));
        } else {
            // 16-bit accumulator
            match data_addr.1 {
                WrappingMode::Bank => {
                    let data = abus.bank_wrapping_cpu_read16(data_addr.0);
                    abus.bank_wrapping_cpu_write16(data_addr.0, self.rotate_right16(data))
                }
                WrappingMode::AddrSpace => {
                    let data = abus.addr_wrapping_cpu_read16(data_addr.0);
                    abus.addr_wrapping_cpu_write16(data_addr.0, self.rotate_right16(data))
                }
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    /// Loads the data to `A`
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    /// `P.N` reflects the high bit of the result and `P.Z` whether or not it is zero.
    fn op_lda(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            let data = abus.cpu_read8(data_addr.0) as u16;
            self.a = (self.a & 0xFF00) | data;
            self.p.n = data > 0x7F;
            self.p.z = data == 0;
        } else {
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.a = data;
            self.p.n = data > 0x7FFF;
            self.p.z = data == 0;
        }
    }

    /// Loads the data to `X`
    ///
    /// 8bit if `P.X` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    /// `P.N` reflects the high bit of the result and `P.Z` whether or not it is zero.
    fn op_ldx(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            let data = abus.cpu_read8(data_addr.0) as u16;
            self.x = (self.x & 0xFF00) | data;
            self.p.n = data > 0x7F;
            self.p.z = data == 0;
        } else {
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.x = data;
            self.p.n = data > 0x7FFF;
            self.p.z = data == 0;
        }
    }

    /// Loads the data to `Y`
    ///
    /// 8bit if `P.X` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    /// `P.N` reflects the high bit of the result and `P.Z` whether or not it is zero.
    fn op_ldy(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            let data = abus.cpu_read8(data_addr.0) as u16;
            self.y = (self.y & 0xFF00) | data;
            self.p.n = data > 0x7F;
            self.p.z = data == 0;
        } else {
            let data = match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_read16(data_addr.0),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_read16(data_addr.0),
                WrappingMode::Page => unreachable!(),
            };
            self.y = data;
            self.p.n = data > 0x7FFF;
            self.p.z = data == 0;
        }
    }

    /// Stores `A` to memory `data_addr` points to
    ///
    /// 8bit if `P.M` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    fn op_sta(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            abus.cpu_write8(data_addr.0, self.a as u8);
        } else {
            match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_write16(data_addr.0, self.a),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_write16(data_addr.0, self.a),
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    /// Stores `X` to memory `data_addr` points to
    ///
    /// 8bit if `P.X` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    fn op_stx(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            abus.cpu_write8(data_addr.0, self.x as u8);
        } else {
            match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_write16(data_addr.0, self.x),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_write16(data_addr.0, self.x),
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    /// Stores `Y` to memory `data_addr` points to
    ///
    /// 8bit if `P.X` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    fn op_sty(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.x {
            abus.cpu_write8(data_addr.0, self.y as u8);
        } else {
            match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_write16(data_addr.0, self.y),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_write16(data_addr.0, self.y),
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    /// Zeroes the memory `data_addr` points to
    ///
    /// 8bit if `P.X` is `1` and 16bit if `0`. Data is accessed according to WrappingMode.
    fn op_stz(&mut self, data_addr: &(u32, WrappingMode), abus: &mut ABus) {
        if self.p.m {
            abus.cpu_write8(data_addr.0, 0x00);
        } else {
            match data_addr.1 {
                WrappingMode::Bank => abus.bank_wrapping_cpu_write16(data_addr.0, 0x0000),
                WrappingMode::AddrSpace => abus.addr_wrapping_cpu_write16(data_addr.0, 0x0000),
                WrappingMode::Page => unreachable!(),
            }
        }
    }

    /// Moves data from `data_addrs.0` to `data_addrs.1`, decrements `A` and increments both
    /// `X` and `Y`
    ///
    /// DB is set to the destination bank and instruction "loops" until `A` holds `$FFFF`.
    /// PC` is incremented by 3 afterward.
    fn op_mvn(&mut self, data_addrs: &(u32, u32), abus: &mut ABus) {
        let data = abus.cpu_read8(data_addrs.0);
        abus.cpu_write8(data_addrs.1, data);
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

    /// Moves data from `data_addrs.0` to `data_addrs.1`, decrements `A`, `X` and `Y`
    ///
    /// DB is set to the destination bank and instruction "loops" until `A` holds `$FFFF`.
    /// PC` is incremented by 3 afterward.
    fn op_mvp(&mut self, data_addrs: &(u32, u32), abus: &mut ABus) {
        let data = abus.cpu_read8(data_addrs.0);
        abus.cpu_write8(data_addrs.1, data);
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

/// Concats bank and 16bit address to 24bit address
#[inline(always)]
fn addr_8_16(bank: u8, byte: u16) -> u32 {
    ((bank as u32) << 16) | byte as u32
}

/// Concats bank, page and byte to 24bit address
#[inline(always)]
fn addr_8_8_8(bank: u8, page: u8, byte: u8) -> u32 {
    ((bank as u32) << 16) | ((page as u32) << 8) | byte as u32
}

/// Converts normal u8 to 8bit binary-coded decimal
fn dec_to_bcd8(value: u8) -> u8 {
    if value > 99 {
        error!("{} too large for 8bit BCD", value);
    }
    ((value / 10) << 4) | (value % 10)
}

/// Converts normal u16 to 16bit binary-coded decimal
fn dec_to_bcd16(value: u16) -> u16 {
    if value > 9999 {
        error!("{} too large for 16bit BCD", value);
    }
    ((value / 1000) << 12)
        | (((value % 1000) / 100) << 8)
        | (((value % 100) / 10) << 4)
        | (value % 10)
}

/// Convert 8bit binary-coded decimal to normal u8
fn bcd_to_dec8(value: u8) -> u8 {
    let ll = value & 0x000F;
    let hh = (value >> 4) & 0x000F;
    if hh > 0x9 || ll > 0x9 {
        error!("{:02X} is not a valid 8bit BCD", value);
    }
    hh * 10 + ll
}

/// Converts 16bit binary-coded decimal to normal u16
fn bcd_to_dec16(value: u16) -> u16 {
    let ll = value & 0x000F;
    let ml = (value >> 4) & 0x000F;
    let mh = (value >> 8) & 0x000F;
    let hh = (value >> 12) & 0x000F;
    if hh > 0x9 || mh > 0x9 || ml > 0x9 || ll > 0x9 {
        error!("{:04X} is not a valid 16bit BCD", value);
    }
    hh * 1000 + mh * 100 + ml * 10 + ll
}

// TODO: Move inside W65C816S
/// Native mode co-processor vector (unused in SNES?)
const COP16: u32 = 0x00FFE4;
/// Native mode BRK vector
const BRK16: u32 = 0x00FFE6;
/// Native mode ABORT vector
#[allow(dead_code)]
const ABORT16: u32 = 0x00FFE8;
/// Native mode non-maskable interrupt vector. Called on vblank
#[allow(dead_code)]
const NMI16: u32 = 0x00FFEA;
/// Native mode interrupt request
#[allow(dead_code)]
const IRQ16: u32 = 0x00FFEE;
/// Emulation mode co-processor vector (unused in SNES?)
const COP8: u32 = 0x00FFF4;
/// Emulation mode ABORT vector
#[allow(dead_code)]
const ABORT8: u32 = 0x00FFF8;
/// Emulation mode non-maskable interrupt vector. Called on vblank
#[allow(dead_code)]
const NMI8: u32 = 0x00FFFA;
/// Reset vector, execution begins from this
const RESET8: u32 = 0x00FFFC;
/// Emulation mode interrupt request / BRK vector
const IRQBRK8: u32 = 0x00FFFE;

// TODO: Move inside StatusReg?
/// Carry mask
const P_C: u8 = 0b0000_0001;
/// Zero mask
const P_Z: u8 = 0b0000_0010;
/// Interrupt mask
const P_I: u8 = 0b0000_0100;
/// Decimal mode mask
const P_D: u8 = 0b0000_1000;
/// Index register width mask
const P_X: u8 = 0b0001_0000;
/// Accumulator width mask
const P_M: u8 = 0b0010_0000;
/// Overflow mask
const P_V: u8 = 0b0100_0000;
/// Negative mask
const P_N: u8 = 0b1000_0000;

/// Indicates the wrapping mode of the data an address is pointing to
#[derive(PartialEq, Debug)]
pub enum WrappingMode {
    /// Data wraps at page boundary
    Page,
    /// Data wraps at bank boundary
    Bank,
    /// Data wraps at address space boundary
    AddrSpace,
}

/// The status register in 65C816
#[derive(Clone)]
struct StatusReg {
    /// Negative (0 = positive, 1 = negative)
    n: bool,
    /// Overflow (0 = no overflow, 1 = overflow)
    v: bool,
    /// Memory and accumulator width (0 = 16bit, 1 = 8bit)
    m: bool,
    /// Index register width (0 = 16bit, 1 = 8bit)
    x: bool,
    /// Decimal mode for ADC/SBC (0 = binary, 1 = binary-coded decimal)
    d: bool,
    /// Interrupt flag (0 = IRQ enable, 1 = IRQ disable)
    i: bool,
    /// Zero flag (0 = non-zero, 1 = zero)
    z: bool,
    /// Carry flag (0 = no carry, 1 = carry)
    c: bool,
}

impl StatusReg {
    /// Initializes a new instance with default values (`m`, `x` and `i` are set)
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

    /// Sets the flags that are set in `mask`
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

    /// Clears the flags that are set in `mask`
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

    /// Returns the u8 value of the register as it would be in hardware
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

    /// Sets the register to `value`
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

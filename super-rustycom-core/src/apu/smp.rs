use super::bus::Bus;

pub struct SPC700 {
    /// 8bit accumulator
    a: u8,
    /// 8bit index
    x: u8,
    /// 8bit index
    y: u8,
    /// 8bit stack pointer
    sp: u8,
    /// Status flags
    /// `N`egative
    ///
    /// O`V`erflow
    ///
    /// `P` Zero page location (0=$00xx, 1=$01xx)
    ///
    /// `B`reak
    ///
    /// `H`alf carry
    ///
    /// `I`nterrupt
    ///
    /// `Z`ero
    ///
    /// `C`arry
    psw: StatusReg,
    /// Combination of `Y` and `A` ($YYAA)
    ya: u16,
    /// Program counter
    pc: u16,
}

impl SPC700 {
    pub fn new() -> SPC700 {
        SPC700 {
            a: 0x00,
            x: 0x00,
            y: 0x00,
            sp: 0x00,
            psw: StatusReg::new(),
            ya: 0x0000,
            pc: 0x0000,
        }
    }

    pub fn a(&self) -> u8 {
        self.a
    }
    pub fn x(&self) -> u8 {
        self.x
    }
    pub fn y(&self) -> u8 {
        self.y
    }
    pub fn sp(&self) -> u8 {
        self.sp
    }
    pub fn psw(&self) -> &StatusReg {
        &self.psw
    }
    pub fn ya(&self) -> u16 {
        self.ya
    }
    pub fn pc(&self) -> u16 {
        self.pc
    }

    /// Executes the instruction pointed by `PC` and returns the cycles it took
    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        // Addressing macros return a reference to the byte at the addressed location
        // mut_-versions return mutable reference
        macro_rules! mut_dp {
            // !ad
            ($addr:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.mut_byte(page | ($addr as u16))
            }};

            // !ad + reg
            ($addr:expr, $reg:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.mut_byte(((page | $addr as u16) + $reg as u16) & (page | 0x00FF))
            }};
        }
        macro_rules! dp {
            // !ad
            ($addr:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.byte(page | ($addr as u16))
            }};

            // !ad + reg
            ($addr:expr, $reg:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.byte(((page | $addr as u16) + $reg as u16) & (page | 0x00FF))
            }};
        }
        macro_rules! mut_dp_word {
            // !ad
            ($addr:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.write_word(page | ($addr as u16))
            }};
        }
        macro_rules! dp_word {
            // !ad
            ($addr:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.word(page | ($addr as u16))
            }};
        }
        macro_rules! mut_abs {
            // !addr
            ($addr:expr) => {
                bus.mut_byte($addr as u16)
            };

            // !addr + reg
            ($addr:expr, $reg:expr) => {
                bus.mut_byte($addr.wrapping_add($reg as u16))
            };
        }
        macro_rules! abs {
            // !addr
            ($addr:expr) => {
                bus.byte($addr as u16)
            };

            // !addr + reg
            ($addr:expr, $reg:expr) => {
                bus.byte($addr.wrapping_add($reg as u16))
            };
        }
        macro_rules! mut_ind_y {
            // [addr]+Y
            ($addr:expr ) => {
                bus.mut_byte(bus.word($addr as u16).wrapping_add(self.y as u16))
            };
        }
        macro_rules! ind_y {
            // [ad]+Y
            ($addr:expr ) => {
                bus.byte(bus.word($addr as u16).wrapping_add(self.y as u16))
            };
        }
        macro_rules! mut_x_ind {
            // [addr+X]
            ($addr:expr ) => {
                // TODO: Does this wrap with page?
                // Need to do cast after wrapping add if so
                bus.mut_byte(bus.word(($addr as u16) + (self.x as u16)))
            };
        }
        macro_rules! x_ind {
            // [addr+X]
            ($addr:expr ) => {
                bus.byte(bus.word($addr.wrapping_add(self.x) as u16))
            };
        }

        // Ops
        macro_rules! mov_reg_byte {
            // reg = byte, affects N, Z
            ($reg:expr, $byte:expr, $op_length:expr, $op_cycles:expr) => {{
                *$reg = $byte;
                self.psw.set_n_z($byte);
                self.pc = self.pc.wrapping_add($op_length);
                $op_cycles
            }};
        }
        macro_rules! mov_addr_byte {
            // [addr] = byte, doesn't affect flags
            ($mem_byte:expr, $byte:expr, $op_length:expr, $op_cycles:expr) => {{
                *$mem_byte = $byte;
                self.pc = self.pc.wrapping_add($op_length);
                $op_cycles
            }};
        }
        macro_rules! push {
            // Push byte to stack
            ($byte:expr) => {{
                *bus.mut_byte(0x0010 & (self.sp as u16)) = $byte;
                self.sp = self.sp.wrapping_sub(1);
                self.pc = self.pc.wrapping_add(1);
                4
            }};
        }
        macro_rules! pull {
            // Pull byte from stack
            ($reg:expr) => {{
                self.sp = self.sp.wrapping_add(1);
                *$reg = bus.byte(0x0010 & (self.sp as u16));
                4
            }};
        }
        macro_rules! op {
            // Used to wrap the alu ops
            ($op:ident, $lhs:expr, $rhs:expr, $op_length:expr, $op_cycles:expr) => {{
                $op!($lhs, $rhs);
                self.pc = self.pc.wrapping_add($op_length);
                $op_cycles
            }};
            // Used to wrap increment, decrement, shift and rotate ops
            ($op:ident, $tgt:expr, $op_length:expr, $op_cycles:expr) => {{
                $op!($tgt);
                self.pc = self.pc.wrapping_add($op_length);
                $op_cycles
            }};
        }
        macro_rules! or {
            // lhs = lhs | rhs, afects N,Z
            // sfc dev wiki doesn't have this affecting lhs
            // nocash and ferris do
            ($lhs:expr, $rhs:expr) => {{
                let result = *$lhs | $rhs;
                self.psw.set_n_z(result);
                *$lhs = result;
            }};
        }
        macro_rules! and {
            // lhs = lhs & rhs, afects N,Z
            ($lhs:expr, $rhs:expr) => {{
                let result = *$lhs & $rhs;
                self.psw.set_n_z(result);
                *$lhs = result;
            }};
        }
        macro_rules! eor {
            // lhs = lhs ^ rhs, afects N,Z
            ($lhs:expr, $rhs:expr) => {{
                let result = *$lhs ^ $rhs;
                self.psw.set_n_z(result);
                *$lhs = result;
            }};
        }
        macro_rules! cmp {
            // lhs - rhs, afects N,Z,C
            ($lhs:expr, $rhs:expr) => {{
                let result = ($lhs as u16) + (!$rhs as u16);
                self.psw.set_n_z_c(result);
            }};
        }
        macro_rules! set_adc_sbc_flags8 {
            ($lhs:expr, $rhs:expr, $result:expr) => {{
                self.psw.set_n_z_c($result);
                let result8 = $result as u8;
                self.psw.set_v(
                    ($lhs < 0x80 && $rhs < 0x80 && result8 > 0x7F)
                        || ($lhs > 0x7F && $rhs > 0x7F && result8 < 0x80),
                );
                // TODO: H
            }};
        }
        macro_rules! addw_subw_end {
            ($rhs:expr, $result:expr) => {{
                self.psw.set_n($result & 0x8000 > 0);
                self.psw.set_z($result == 0x0000);
                self.psw.set_c($result > 0xFFFF);
                let result16 = $result as u16;
                self.psw.set_v(
                    (self.ya < 0x8000 && $rhs < 0x8000 && result16 > 0x7FFF)
                        || (self.ya > 0x7FFF && $rhs > 0x7FFF && result16 < 0x8000),
                );
                // TODO: H
                self.y = ($result >> 8) as u8;
                self.a = $result as u8;
                self.pc = self.pc.wrapping_add(2);
                5
            }};
        }
        macro_rules! adc {
            // lhs = lhs + rhs + C, affects N,V,H,Z,C
            ($lhs:expr, $rhs:expr) => {{
                let result = (*$lhs as u16) + ($rhs as u16) + (self.psw.c() as u16);
                set_adc_sbc_flags8!(*$lhs, $rhs, result);
                *$lhs = result as u8;
            }};
        }
        macro_rules! sbc {
            // lhs = lhs - rhs - !C, afects N,Z,C
            ($lhs:expr, $rhs:expr) => {{
                let result = (*$lhs as u16) + (!$rhs as u16) + (self.psw.c() as u16);
                set_adc_sbc_flags8!(*$lhs, $rhs, result);
                *$lhs = result as u8;
            }};
        }
        macro_rules! asl {
            // tgt << 1, bit0 is 0, affects N,Z,C
            ($tgt:expr) => {{
                let result = (*$tgt as u16) << 1;
                self.psw.set_n_z_c(result);
                *$tgt = result as u8;
            }};
        }
        macro_rules! rol {
            // tgt << 1, bit0 is C, affects N,Z,C
            ($tgt:expr) => {{
                let result = ((*$tgt as u16) << 1) | (self.psw.c() as u16);
                self.psw.set_n_z_c(result);
                *$tgt = result as u8;
            }};
        }
        macro_rules! lsr {
            // tgt >> 1, bit0 is 0, affects N,Z,C
            ($tgt:expr) => {{
                let result = *$tgt >> 1;
                self.psw.set_n_z(result);
                self.psw.set_c(*$tgt & 0x01 > 0);
                *$tgt = result;
            }};
        }
        macro_rules! ror {
            // tgt >> 1, bit0 is C, affects N,Z,C
            ($tgt:expr) => {{
                let result = ((self.psw.c() as u8) << 7) | (*$tgt >> 1);
                self.psw.set_n_z(result);
                self.psw.set_c(*$tgt & 0x01 > 0);
                *$tgt = result as u8;
            }};
        }
        macro_rules! inc {
            // tgt += 1, affects N,Z
            ($tgt:expr) => {{
                let result = $tgt.wrapping_add(1);
                self.psw.set_n_z(result);
                *$tgt = result as u8;
            }};
        }
        macro_rules! dec {
            // tgt -= 1, affects N,Z
            ($tgt:expr) => {{
                let result = $tgt.wrapping_sub(1);
                self.psw.set_n_z(result);
                *$tgt = result as u8;
            }};
        }

        let op_code = bus.byte(self.pc);
        // Pre-fetch operands for brevity
        let b0 = bus.byte(self.pc.wrapping_add(1));
        let b1 = bus.byte(self.pc.wrapping_add(2));
        let op8 = b0;
        let op16 = ((b1 as u16) << 8) | b0 as u16;

        let op_length = match op_code {
            // MOV A,#nn
            0xE8 => mov_reg_byte!(&mut self.a, op8, 2, 2),
            // MOV X,#nn
            0xCD => mov_reg_byte!(&mut self.x, op8, 2, 2),
            // MOV Y,#nn
            0x8D => mov_reg_byte!(&mut self.x, op8, 2, 2),
            // MOV A,X
            0x7D => mov_reg_byte!(&mut self.a, self.x, 1, 2),
            // MOV X,A
            0x5D => mov_reg_byte!(&mut self.x, self.a, 1, 2),
            // MOV A,Y
            0xDD => mov_reg_byte!(&mut self.a, self.y, 1, 2),
            // MOV Y,A
            0xFD => mov_reg_byte!(&mut self.y, self.a, 1, 2),
            // MOV X,SP
            0x9D => mov_reg_byte!(&mut self.x, self.sp, 1, 2),
            // MOV SP,X
            0xBD => mov_reg_byte!(&mut self.sp, self.x, 1, 2),
            // MOV A,!dp
            0xE4 => mov_reg_byte!(&mut self.a, dp!(op8), 2, 3),
            // MOV A,!dp+X
            0xF4 => mov_reg_byte!(&mut self.a, dp!(op8, self.x), 2, 4),
            // MOV A,!abs
            0xE5 => mov_reg_byte!(&mut self.a, abs!(op16), 3, 4),
            // MOV A,!abs+X
            0xF5 => mov_reg_byte!(&mut self.a, abs!(op16, self.x), 3, 5),
            // MOV A,!abs+Y
            0xF6 => mov_reg_byte!(&mut self.a, abs!(op16, self.y), 3, 5),
            // MOV A,(X)
            0xE6 => mov_reg_byte!(&mut self.a, abs!(self.x), 1, 3),
            // MOV A,(X)+
            0xBF => {
                mov_reg_byte!(&mut self.a, abs!(self.x), 1, 4);
                self.x = self.x.wrapping_add(1);
                4
            }
            // MOV A,[!dp+Y]
            0xF7 => mov_reg_byte!(&mut self.a, ind_y!(op8), 2, 6),
            // MOV A,[!dp+X]
            0xE7 => mov_reg_byte!(&mut self.a, x_ind!(op8), 2, 6),
            // MOV X,!dp
            0xF8 => mov_reg_byte!(&mut self.x, dp!(op8), 2, 3),
            // MOV X,!dp+Y
            0xF9 => mov_reg_byte!(&mut self.x, dp!(op8, self.y), 2, 4),
            // MOV X,!abs
            0xE9 => mov_reg_byte!(&mut self.x, abs!(op16), 3, 4),
            // MOV Y,!dp
            0xEB => mov_reg_byte!(&mut self.y, dp!(op8), 2, 3),
            // MOV Y,!dp+X
            0xFB => mov_reg_byte!(&mut self.y, dp!(op8, self.x), 2, 4),
            // MOV Y,!abs
            0xEC => mov_reg_byte!(&mut self.y, abs!(op16), 3, 4),
            // MOVW YA,!dp
            0xBA => unimplemented!(), // len 2, cycles 5
            // MOV !dp,#nn
            0x8F => mov_addr_byte!(mut_dp!(b1), b0, 3, 5),
            // MOV !dp,!dp
            0xFA => mov_addr_byte!(mut_dp!(b1), dp!(b0), 3, 5),
            // MOV !dp,A
            0xC4 => mov_addr_byte!(mut_dp!(op8), self.a, 2, 4),
            // MOV !dp,X
            0xD8 => mov_addr_byte!(mut_dp!(op8), self.x, 2, 4),
            // MOV !dp,Y
            0xCB => mov_addr_byte!(mut_dp!(op8), self.y, 2, 4),
            // MOV !dp+X,A
            0xD4 => mov_addr_byte!(mut_dp!(op8, self.x), self.a, 2, 5),
            // MOV !dp+X,Y
            0xDB => mov_addr_byte!(mut_dp!(op8, self.x), self.y, 2, 5),
            // MOV !dp+Y,X
            0xD9 => mov_addr_byte!(mut_dp!(op8, self.y), self.x, 2, 5),
            // MOV !abs,A
            0xC5 => mov_addr_byte!(mut_abs!(op16), self.a, 3, 5),
            // MOV !abs,X
            0xC9 => mov_addr_byte!(mut_abs!(op16), self.x, 3, 5),
            // MOV !abs,Y
            0xCC => mov_addr_byte!(mut_abs!(op16), self.y, 3, 5),
            // MOV !abs+X,A
            0xD5 => mov_addr_byte!(mut_abs!(op16, self.x), self.a, 3, 6),
            // MOV !abs+Y,A
            0xD6 => mov_addr_byte!(mut_abs!(op16, self.y), self.a, 3, 6),
            // MOV (X)+,A
            0xAF => {
                mov_addr_byte!(mut_abs!(self.x), self.a, 1, 4);
                self.x = self.x.wrapping_add(1);
                4
            }
            // MOV (X),A
            0xC6 => mov_addr_byte!(mut_dp!(self.x), self.a, 1, 4),
            // MOV [dp]+Y,A
            0xD7 => mov_addr_byte!(mut_ind_y!(op8), self.a, 2, 7),
            // MOV [dp+X],A
            0xC7 => mov_addr_byte!(mut_x_ind!(op8), self.a, 2, 7),
            // MOVW aa,YA
            0xDA => unimplemented!(), // len 5
            // PUSH A
            0x2D => push!(self.a),
            // PUSH X
            0x4D => push!(self.x),
            // PUSH Y
            0x6D => push!(self.y),
            // PUSH Y
            0x0D => push!(self.psw.value),
            // PULL A
            0xAE => pull!(&mut self.a),
            // PULL X
            0xCE => pull!(&mut self.x),
            // PULL Y
            0xEE => pull!(&mut self.y),
            // PULL Y
            0x8E => pull!(&mut self.psw.value),
            // AND A,#nn
            0x28 => op!(and, &mut self.a, op8, 2, 2),
            // AND A,(X)
            0x26 => op!(and, &mut self.a, dp!(self.x), 1, 3),
            // AND A,dp
            0x24 => op!(and, &mut self.a, dp!(op8), 2, 3),
            // AND A,dp+X
            0x34 => op!(and, &mut self.a, dp!(op8, self.x), 2, 4),
            // AND A,!abs
            0x25 => op!(and, &mut self.a, abs!(op16), 3, 4),
            // AND A,!abs+X
            0x35 => op!(and, &mut self.a, abs!(op16, self.x), 3, 5),
            // AND A,!abs+Y
            0x36 => op!(and, &mut self.a, abs!(op16, self.y), 3, 5),
            // AND A,[dp+X]
            0x27 => op!(and, &mut self.a, x_ind!(op8), 2, 6),
            // AND A,[dp]+Y
            0x37 => op!(and, &mut self.a, ind_y!(op8), 2, 6),
            // AND dp,dp
            0x29 => op!(and, mut_dp!(b0), dp!(b1), 3, 6),
            // AND dp,#nn
            0x38 => op!(and, mut_dp!(b0), b1, 3, 5),
            // AND (X),(Y)
            0x39 => op!(and, mut_dp!(self.x), dp!(self.y), 1, 5),
            // OR A,#nn
            0x08 => op!(or, &mut self.a, op8, 2, 2),
            // OR A,(X)
            0x06 => op!(or, &mut self.a, dp!(self.x), 1, 3),
            // OR A,dp
            0x04 => op!(or, &mut self.a, dp!(op8), 2, 3),
            // OR A,dp+X
            0x14 => op!(or, &mut self.a, dp!(op8, self.x), 2, 4),
            // OR A,!abs
            0x05 => op!(or, &mut self.a, abs!(op16), 3, 4),
            // OR A,!abs+X
            0x15 => op!(or, &mut self.a, abs!(op16, self.x), 3, 5),
            // OR A,!abs+Y
            0x16 => op!(or, &mut self.a, abs!(op16, self.y), 3, 5),
            // OR A,[dp+X]
            0x07 => op!(or, &mut self.a, x_ind!(op8), 2, 6),
            // OR A,[dp]+Y
            0x17 => op!(or, &mut self.a, ind_y!(op8), 2, 6),
            // OR dp,dp
            0x09 => op!(or, mut_dp!(b0), dp!(b1), 3, 6),
            // OR dp,#nn
            0x18 => op!(or, mut_dp!(b0), b1, 3, 5),
            // OR (X),(Y)
            0x19 => op!(or, mut_dp!(self.x), dp!(self.y), 1, 5),
            // EOR A,#nn
            0x48 => op!(eor, &mut self.a, op8, 2, 2),
            // EOR A,(X)
            0x46 => op!(eor, &mut self.a, dp!(self.x), 1, 3),
            // EOR A,dp
            0x44 => op!(eor, &mut self.a, dp!(op8), 2, 3),
            // EOR A,dp+X
            0x54 => op!(eor, &mut self.a, dp!(op8, self.x), 2, 4),
            // EOR A,!abs
            0x45 => op!(eor, &mut self.a, abs!(op16), 3, 4),
            // EOR A,!abs+X
            0x55 => op!(eor, &mut self.a, abs!(op16, self.x), 3, 5),
            // EOR A,!abs+Y
            0x56 => op!(eor, &mut self.a, abs!(op16, self.y), 3, 5),
            // EOR A,[dp+X]
            0x47 => op!(eor, &mut self.a, x_ind!(op8), 2, 6),
            // EOR A,[dp]+Y
            0x57 => op!(eor, &mut self.a, ind_y!(op8), 2, 6),
            // EOR dp,dp
            0x49 => op!(eor, mut_dp!(b0), dp!(b1), 3, 6),
            // EOR dp,#nn
            0x58 => op!(eor, mut_dp!(b0), b1, 3, 5),
            // EOR (X),(Y)
            0x59 => op!(eor, mut_dp!(self.x), dp!(self.y), 1, 5),
            // ADC A,#nn
            0x88 => op!(adc, &mut self.a, op8, 2, 2),
            // ADC A,(X)
            0x86 => op!(adc, &mut self.a, dp!(self.x), 1, 3),
            // ADC A,dp
            0x84 => op!(adc, &mut self.a, dp!(op8), 2, 3),
            // ADC A,dp+X
            0x94 => op!(adc, &mut self.a, dp!(op8, self.x), 2, 4),
            // ADC A,!abs
            0x85 => op!(adc, &mut self.a, abs!(op16), 3, 4),
            // ADC A,!abs+X
            0x95 => op!(adc, &mut self.a, abs!(op16, self.x), 3, 5),
            // ADC A,!abs+Y
            0x96 => op!(adc, &mut self.a, abs!(op16, self.y), 3, 5),
            // ADC A,[dp+X]
            0x87 => op!(adc, &mut self.a, x_ind!(op8), 2, 6),
            // ADC A,[dp]+Y
            0x97 => op!(adc, &mut self.a, ind_y!(op8), 2, 6),
            // ADC dp,dp
            0x89 => op!(adc, mut_dp!(b0), dp!(b1), 3, 6),
            // ADC dp,#nn
            0x98 => op!(adc, mut_dp!(b0), b1, 3, 5),
            // ADC (X),(Y)
            0x99 => op!(adc, mut_dp!(self.x), dp!(self.y), 1, 5),
            // SBC A,#nn
            0xA8 => op!(sbc, &mut self.a, op8, 2, 2),
            // SBC A,(X)
            0xA6 => op!(sbc, &mut self.a, dp!(self.x), 1, 3),
            // SBC A,dp
            0xA4 => op!(sbc, &mut self.a, dp!(op8), 2, 3),
            // SBC A,dp+X
            0xB4 => op!(sbc, &mut self.a, dp!(op8, self.x), 2, 4),
            // SBC A,!abs
            0xA5 => op!(sbc, &mut self.a, abs!(op16), 3, 4),
            // SBC A,!abs+X
            0xB5 => op!(sbc, &mut self.a, abs!(op16, self.x), 3, 5),
            // SBC A,!abs+Y
            0xB6 => op!(sbc, &mut self.a, abs!(op16, self.y), 3, 5),
            // SBC A,[dp+X]
            0xA7 => op!(sbc, &mut self.a, x_ind!(op8), 2, 6),
            // SBC A,[dp]+Y
            0xB7 => op!(sbc, &mut self.a, ind_y!(op8), 2, 6),
            // SBC dp,dp
            0xA9 => op!(sbc, mut_dp!(b0), dp!(b1), 3, 6),
            // SBC dp,#nn
            0xB8 => op!(sbc, mut_dp!(b0), b1, 3, 5),
            // SBC (X),(Y)
            0xB9 => op!(sbc, mut_dp!(self.x), dp!(self.y), 1, 5),
            // CMP A,#nn
            0x68 => op!(cmp, self.a, op8, 2, 2),
            // CMP A,(X)
            0x66 => op!(cmp, self.a, dp!(self.x), 1, 3),
            // CMP A,dp
            0x64 => op!(cmp, self.a, dp!(op8), 2, 3),
            // CMP A,dp+X
            0x74 => op!(cmp, self.a, dp!(op8, self.x), 2, 4),
            // CMP A,!abs
            0x65 => op!(cmp, self.a, abs!(op16), 3, 4),
            // CMP A,!abs+X
            0x75 => op!(cmp, self.a, abs!(op16, self.x), 3, 5),
            // CMP A,!abs+Y
            0x76 => op!(cmp, self.a, abs!(op16, self.y), 3, 5),
            // CMP A,[dp+X]
            0x67 => op!(cmp, self.a, x_ind!(op8), 2, 6),
            // CMP A,[dp]+Y
            0x77 => op!(cmp, self.a, ind_y!(op8), 2, 6),
            // CMP dp,dp
            0x69 => op!(cmp, dp!(b0), dp!(b1), 3, 6),
            // CMP dp,#nn
            0x78 => op!(cmp, dp!(b0), b1, 3, 5),
            // CMP (X),(Y)
            0x79 => op!(cmp, dp!(self.x), dp!(self.y), 1, 5),
            // CMP X,#nn
            0xC8 => op!(cmp, self.x, op8, 2, 2),
            // CMP X,dp
            0x3E => op!(cmp, self.x, dp!(op8), 2, 3),
            // CMP X,!abs
            0x1E => op!(cmp, self.x, abs!(op16), 3, 4),
            // CMP Y,#nn
            0xAD => op!(cmp, self.y, op8, 2, 2),
            // CMP Y,dp
            0x7E => op!(cmp, self.y, dp!(op8), 2, 3),
            // CMP Y,!abs
            0x5E => op!(cmp, self.y, abs!(op16), 3, 4),
            // INC A
            0xBC => op!(inc, &mut self.a, 1, 2),
            // INC X
            0x3D => op!(inc, &mut self.x, 1, 2),
            // INC Y
            0xFC => op!(inc, &mut self.y, 1, 2),
            // INC dp
            0xAB => op!(inc, mut_dp!(op8), 2, 4),
            // INC dp+X
            0xBB => op!(inc, mut_dp!(op8, self.x), 2, 5),
            // INC !abs
            0xAC => op!(inc, mut_abs!(op16), 3, 5),
            // DEC A
            0x9C => op!(dec, &mut self.a, 1, 2),
            // DEC X
            0x1D => op!(dec, &mut self.x, 1, 2),
            // DEC Y
            0xDC => op!(dec, &mut self.y, 1, 2),
            // DEC dp
            0x8B => op!(dec, mut_dp!(op8), 2, 4),
            // DEC dp+X
            0x9B => op!(dec, mut_dp!(op8, self.x), 2, 5),
            // DEC !abs
            0x8C => op!(dec, mut_abs!(op16), 3, 5),
            // ASL A
            0x1C => op!(asl, &mut self.a, 1, 2),
            // ASL dp
            0x0B => op!(asl, mut_dp!(op8), 2, 4),
            // ASL dp
            0x1B => op!(asl, mut_dp!(op8, self.x), 2, 5),
            // ASL !abs
            0x0C => op!(asl, mut_abs!(op16), 3, 5),
            // ROL A
            0x3C => op!(rol, &mut self.a, 1, 2),
            // ROL dp
            0x2B => op!(rol, mut_dp!(op8), 2, 4),
            // ROL dp+X
            0x3B => op!(rol, mut_dp!(op8, self.x), 2, 5),
            // ROL !abs
            0x2C => op!(rol, mut_abs!(op16), 3, 5),
            // LSR A
            0x5C => op!(lsr, &mut self.a, 1, 2),
            // LSR dp
            0x4B => op!(lsr, mut_dp!(op8), 2, 4),
            // LSR dp+X
            0x5B => op!(lsr, mut_dp!(op8, self.x), 2, 5),
            // LSR !abs
            0x4C => op!(lsr, mut_abs!(op16), 3, 5),
            // ROR A
            0x7C => op!(ror, &mut self.a, 1, 2),
            // ROR dp
            0x6B => op!(ror, mut_dp!(op8), 2, 4),
            // ROR dp+X
            0x7B => op!(ror, mut_dp!(op8, self.x), 2, 5),
            // ROR !abs
            0x6C => op!(ror, mut_abs!(op16), 3, 5),
            // XCN A  affects N,Z
            0x9F => {
                let result = (self.a << 4) | (self.a >> 4);
                self.psw.set_n_z(result);
                self.a = result;
                self.pc = self.pc.wrapping_add(1);
                5
            }
            // ADDW YA,dp  affects N,V,H,Z,C
            0x7A => {
                let rhs = dp_word!(op8);
                let result = (self.ya as u32) + (rhs as u32) + (self.psw.c() as u32);
                addw_subw_end!(rhs, result)
            }
            // SUBW YA,dp  affects N,V,H,Z,C
            0x9A => {
                let rhs = dp_word!(op8);
                let result = (self.ya as u32) + (!rhs as u32) + (self.psw.c() as u32);
                addw_subw_end!(rhs, result)
            }
            // CMPW YA,dp  affects N,Z,C
            0x5A => {
                let rhs = dp_word!(op8);
                let result = (self.ya as u32) + (!rhs as u32);

                self.psw.set_n(result & 0x8000 > 0);
                self.psw.set_z(result == 0x0000);
                self.psw.set_c(result > 0xFFFF);

                self.y = (result >> 8) as u8;
                self.a = result as u8;

                self.pc = self.pc.wrapping_add(2);
                5
            }
            // MOVW YA,dp  affects N,Z
            0xBA => {
                let page = (self.psw.p() as u16) << 8;
                self.ya = bus.word(page | (op8 as u16));
                self.pc = self.pc.wrapping_add(2);
                5
            }
            // MOVW dp,YA
            0xBA => {
                let page = (self.psw.p() as u16) << 8;
                bus.write_word(page | (op8 as u16), self.ya);
                self.pc = self.pc.wrapping_add(2);
                5
            }
            _ => unimplemented!(),
        };
        self.ya = ((self.y as u16) << 8) | (self.a as u16);
        op_length
    }
}

/// The status register in SPC700
#[derive(Clone)]
pub struct StatusReg {
    /// Flags in order from MSb to SLb
    /// Negative (0 = positive, 1 = negative)
    /// Overflow (0 = no overflow, 1 = overflow)
    /// Zero page location (0 = $00xx, 1 = $01xx)
    /// Break flag (0 = reset, 1 = BRK)
    /// Half carry (0 = borrow or no-carry, 1 = carry or no-borrow)
    /// Interrupt flag (0 = enable, 1 = disable) No function in SNES APU
    /// Zero flag (0 = non-zero, 1 = zero)
    /// Carry flag (0 = no carry, 1 = carry)
    value: u8,
}

// Status reg masks
const P_N: u8 = 0b1000_0000;
const P_V: u8 = 0b0100_0000;
const P_P: u8 = 0b0010_0000;
const P_B: u8 = 0b0001_0000;
const P_H: u8 = 0b0000_1000;
const P_I: u8 = 0b0000_0100;
const P_Z: u8 = 0b0000_0010;
const P_C: u8 = 0b0000_0001;

macro_rules! set_bit {
    ($reg:expr, $bit_mask:expr, $cond:expr) => {
        // From seander's bit twiddling hacks
        *$reg = if $cond {
            *$reg | $bit_mask
        } else {
            *$reg & !$bit_mask
        }
    };
}

impl StatusReg {
    /// Initializes a new instance with default values (`m`, `x` and `i` are set)
    pub fn new() -> StatusReg {
        StatusReg { value: 0x00 }
    }

    pub fn n(&self) -> bool {
        self.value & P_N > 0
    }

    pub fn v(&self) -> bool {
        self.value & P_V > 0
    }

    pub fn p(&self) -> bool {
        self.value & P_P > 0
    }

    pub fn b(&self) -> bool {
        self.value & P_B > 0
    }

    pub fn h(&self) -> bool {
        self.value & P_H > 0
    }

    pub fn i(&self) -> bool {
        self.value & P_I > 0
    }

    pub fn z(&self) -> bool {
        self.value & P_Z > 0
    }

    pub fn c(&self) -> bool {
        self.value & P_C > 0
    }

    pub fn set_n_z(&mut self, value: u8) {
        self.set_n((value & 0x80) > 0);
        self.set_z(value == 0x00);
    }

    pub fn set_n_z_c(&mut self, value: u16) {
        self.set_n_z(value as u8);
        self.set_c(value > 0xFF);
    }

    pub fn set_n(&mut self, value: bool) {
        set_bit!(&mut self.value, P_N, value);
    }

    pub fn set_v(&mut self, value: bool) {
        set_bit!(&mut self.value, P_V, value);
    }

    pub fn set_p(&mut self, value: bool) {
        set_bit!(&mut self.value, P_P, value);
    }

    pub fn set_b(&mut self, value: bool) {
        set_bit!(&mut self.value, P_B, value);
    }

    pub fn set_h(&mut self, value: bool) {
        set_bit!(&mut self.value, P_H, value);
    }

    pub fn set_i(&mut self, value: bool) {
        set_bit!(&mut self.value, P_I, value);
    }

    pub fn set_z(&mut self, value: bool) {
        set_bit!(&mut self.value, P_Z, value);
    }

    pub fn set_c(&mut self, value: bool) {
        set_bit!(&mut self.value, P_C, value);
    }
}

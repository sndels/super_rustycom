use super::bus::Bus;

pub struct Spc700 {
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
    /// Indicates a carry from the lower BCD nibble to the upper nibble
    ///
    /// `I`nterrupt
    ///
    /// `Z`ero
    ///
    /// `C`arry
    psw: StatusReg,
    /// Program counter
    pc: u16,
    /// Mode, i.e. running, sleeping or stopped
    mode: Mode,
}

impl Spc700 {
    pub fn default() -> Spc700 {
        Spc700 {
            a: 0x00,
            x: 0x00,
            y: 0x00,
            sp: 0x00,
            psw: StatusReg::new(),
            pc: 0xFFC0,
            mode: Mode::Running,
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
    /// Combination "register" `Y``A`
    pub fn ya(&self) -> u16 {
        ((self.y as u16) << 8) | (self.a as u16)
    }
    pub fn pc(&self) -> u16 {
        self.pc
    }

    /// Executes the instruction pointed by `PC` and returns the cycles it took
    pub fn step(&mut self, bus: &mut Bus) -> Option<u8> {
        if self.mode != Mode::Running {
            // TODO: Handle mode changes back to running
            return None;
        }

        let op_code = bus.byte(self.pc);
        // Pre-fetch operands for brevity
        let op0 = bus.byte(self.pc.wrapping_add(2));
        let op1 = bus.byte(self.pc.wrapping_add(1));
        let op8 = op1;
        let op16 = ((op0 as u16) << 8) | op1 as u16;

        // Addressing macros return a reference to the byte at the addressed location
        // mut_-versions return mutable reference
        // TODO: Go through wrapping rules for dp, absolute and writes, see anomie
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
        macro_rules! dp_word {
            // !ad
            ($addr:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.word(page | ($addr as u16))
            }};
        }
        macro_rules! mut_abs13 {
            // aaa.b
            // Not really mutable bit, returns ref to the full byte
            ($addr:expr) => {{
                bus.mut_byte($addr & 0x1FFF)
            }};
        }
        macro_rules! abs13 {
            // aaa.b
            ($addr:expr) => {{
                bus.byte($addr & 0x1FFF)
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
            // [ad]+Y
            ($addr:expr ) => {
                // TODO: Does this honor current direct page?
                bus.mut_byte(bus.word($addr as u16).wrapping_add(self.y as u16))
            };
        }
        macro_rules! ind_y {
            // [ad]+Y
            ($addr:expr ) => {
                // TODO: Does this honor current direct page?
                bus.byte(bus.word($addr as u16).wrapping_add(self.y as u16))
            };
        }
        macro_rules! mut_x_ind {
            // [ad+X]
            ($addr:expr ) => {
                // TODO: Does this honor current direct page?
                // TODO: Does this really with page?
                bus.mut_byte(bus.word($addr.wrapping_add(self.x) as u16))
            };
        }
        macro_rules! x_ind {
            // [ad+X]
            ($addr:expr ) => {
                // TODO: Does this honor current direct page?
                // TODO: Does this really with page?
                bus.byte(bus.word($addr.wrapping_add(self.x) as u16))
            };
        }

        // Ops
        macro_rules! mov_reg_byte {
            // reg = byte, affects N, Z
            ($reg:expr, $byte:expr, $op_length:expr, $op_cycles:expr) => {{
                *$reg = $byte;
                self.psw.set_n_z_byte($byte);
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
        macro_rules! push_byte {
            ($byte:expr) => {{
                *bus.mut_byte(0x0100 & (self.sp as u16)) = $byte;
                self.sp = self.sp.wrapping_sub(1);
            }};
        }
        macro_rules! push_word {
            ($word:expr) => {{
                bus.write_word(0x0100 & (self.sp.wrapping_sub(1) as u16), $word);
                self.sp = self.sp.wrapping_sub(2);
            }};
        }
        macro_rules! pop_byte {
            () => {{
                self.sp = self.sp.wrapping_add(1);
                bus.byte(0x0100 & (self.sp as u16))
            }};
        }
        macro_rules! pop_word {
            () => {{
                let word = bus.word(0x0100 & (self.sp.wrapping_add(1) as u16));
                self.sp = self.sp.wrapping_add(2);
                word
            }};
        }
        macro_rules! push {
            // Push `reg` to stack
            ($reg:expr) => {{
                push_byte!($reg);
                self.pc = self.pc.wrapping_add(1);
                4
            }};
        }
        macro_rules! pop {
            // Pop byte from stack to `reg`
            ($reg:expr) => {{
                *$reg = pop_byte!();
                self.pc = self.pc.wrapping_add(1);
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
                self.psw.set_n_z_byte(result);
                *$lhs = result;
            }};
        }
        macro_rules! and {
            // lhs = lhs & rhs, afects N,Z
            ($lhs:expr, $rhs:expr) => {{
                let result = *$lhs & $rhs;
                self.psw.set_n_z_byte(result);
                *$lhs = result;
            }};
        }
        macro_rules! eor {
            // lhs = lhs ^ rhs, afects N,Z
            ($lhs:expr, $rhs:expr) => {{
                let result = *$lhs ^ $rhs;
                self.psw.set_n_z_byte(result);
                *$lhs = result;
            }};
        }
        macro_rules! cmp {
            // lhs - rhs, afects N,Z,C
            ($lhs:expr, $rhs:expr) => {{
                let result = ($lhs as u16) + (!$rhs as u16) + 1;
                self.psw.set_n_z_c_byte(result);
            }};
        }
        macro_rules! set_adc_sbc_flags8 {
            ($lhs:expr, $rhs:expr, $result:expr) => {{
                self.psw.set_n_z_c_byte($result);
                let result8 = $result as u8;
                self.psw.set_v(
                    ($lhs < 0x80 && $rhs < 0x80 && result8 > 0x7F)
                        || ($lhs > 0x7F && $rhs > 0x7F && result8 < 0x80),
                );
                // BCD carry from lower nibble to upper
                self.psw.set_h(($lhs & 0x0F) + ($rhs & 0x0F) > 0x0F);
            }};
        }
        macro_rules! addw_subw_end {
            ($rhs:expr, $result:expr) => {{
                self.psw.set_n_z_c_word($result);
                let result16 = $result as u16;

                self.psw.set_v(
                    (self.ya() < 0x8000 && $rhs < 0x8000 && result16 > 0x7FFF)
                        || (self.ya() > 0x7FFF && $rhs > 0x7FFF && result16 < 0x8000),
                );
                // BCD carry from lower nibble to upper
                self.psw
                    .set_h((self.ya() & 0x00FF) + ($rhs & 0x00FF) > 0x00FF);

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
                let result = (*$lhs as u16)
                    .wrapping_add(!$rhs as u16)
                    .wrapping_add(self.psw.c() as u16);
                set_adc_sbc_flags8!(*$lhs, $rhs, result);
                *$lhs = result as u8;
            }};
        }
        macro_rules! asl {
            // tgt << 1, bit0 is 0, affects N,Z,C
            ($tgt:expr) => {{
                let result = (*$tgt as u16) << 1;
                self.psw.set_n_z_c_byte(result);
                *$tgt = result as u8;
            }};
        }
        macro_rules! rol {
            // tgt << 1, bit0 is C, affects N,Z,C
            ($tgt:expr) => {{
                let result = ((*$tgt as u16) << 1) | (self.psw.c() as u16);
                self.psw.set_n_z_c_byte(result);
                *$tgt = result as u8;
            }};
        }
        macro_rules! lsr {
            // tgt >> 1, bit0 is 0, affects N,Z,C
            ($tgt:expr) => {{
                let result = *$tgt >> 1;
                self.psw.set_n_z_byte(result);
                self.psw.set_c(*$tgt & 0x01 > 0);
                *$tgt = result;
            }};
        }
        macro_rules! ror {
            // tgt >> 1, bit0 is C, affects N,Z,C
            ($tgt:expr) => {{
                let result = ((self.psw.c() as u8) << 7) | (*$tgt >> 1);
                self.psw.set_n_z_byte(result);
                self.psw.set_c(*$tgt & 0x01 > 0);
                *$tgt = result as u8;
            }};
        }
        macro_rules! inc {
            // tgt += 1, affects N,Z
            ($tgt:expr) => {{
                let result = $tgt.wrapping_add(1);
                self.psw.set_n_z_byte(result);
                *$tgt = result as u8;
            }};
        }
        macro_rules! dec {
            // tgt -= 1, affects N,Z
            ($tgt:expr) => {{
                let result = $tgt.wrapping_sub(1);
                self.psw.set_n_z_byte(result);
                *$tgt = result as u8;
            }};
        }
        macro_rules! incw_decw {
            // Expects wrapping_add/wrapping_sub as `op`, affects N,Z
            ($dp:expr, $op:ident) => {{
                let page = (self.psw.p() as u16) << 8;
                let addr = page | ($dp as u16);
                let result = bus.word(addr).$op(1);
                self.psw.set_n_z_word(result);
                bus.write_word(addr, result);
                self.pc = self.pc.wrapping_add(2);
                6
            }};
        }
        macro_rules! br {
            // Branches with relative address based on cond
            // Not taking the branch takes 2 less cycles
            ($cond:expr, $rel:expr, $op_length:expr, $base_cycles:expr) => {{
                // PC gets incremented when reading op code and operands
                self.pc = self.pc.wrapping_add($op_length);
                if $cond {
                    // Do sign extend as rel is signed
                    self.pc = self.pc.wrapping_add(($rel as i8) as u16);
                    $base_cycles
                } else {
                    $base_cycles - 2
                }
            }};
        }
        macro_rules! call {
            // Pushes PC to stack and sets it to addr
            // Takes 8 cycles
            ($addr:expr) => {{
                push_word!(self.pc);
                self.pc = $addr;
                8
            }};
        }
        macro_rules! write_ya {
            // We need to update YA as well as Y, A
            ($value:expr) => {{
                self.y = ($value >> 8) as u8;
                self.a = $value as u8;
            }};
        }
        macro_rules! bit_set {
            ($bit:expr, $byte:expr) => {
                ($byte >> $bit) & 0x1 == 0x1
            };
        }
        macro_rules! bit_op {
            ($op:tt, $complement:expr, $op_length:expr) => {{
                let bit = (op16 >> 13) as u8;
                let byte = abs13!(op16);
                let bit_mask = if $complement { !bit_set!(byte, bit) } else { bit_set!(byte, bit) };
                self.psw.set_c(self.psw.c() $op bit_mask);
                self.pc = self.pc.wrapping_add(3);
                $op_length
            }};
        }

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
            // MOV A,dp
            0xE4 => mov_reg_byte!(&mut self.a, dp!(op8), 2, 3),
            // MOV A,dp+X
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
            // MOV A,[dp+Y]
            0xF7 => mov_reg_byte!(&mut self.a, ind_y!(op8), 2, 6),
            // MOV A,[dp+X]
            0xE7 => mov_reg_byte!(&mut self.a, x_ind!(op8), 2, 6),
            // MOV X,dp
            0xF8 => mov_reg_byte!(&mut self.x, dp!(op8), 2, 3),
            // MOV X,dp+Y
            0xF9 => mov_reg_byte!(&mut self.x, dp!(op8, self.y), 2, 4),
            // MOV X,!abs
            0xE9 => mov_reg_byte!(&mut self.x, abs!(op16), 3, 4),
            // MOV Y,dp
            0xEB => mov_reg_byte!(&mut self.y, dp!(op8), 2, 3),
            // MOV Y,dp+X
            0xFB => mov_reg_byte!(&mut self.y, dp!(op8, self.x), 2, 4),
            // MOV Y,!abs
            0xEC => mov_reg_byte!(&mut self.y, abs!(op16), 3, 4),
            // MOVW YA,dp  affects N,Z
            0xBA => {
                let page = (self.psw.p() as u16) << 8;
                let word = bus.word(page | (op8 as u16));
                write_ya!(word);
                self.psw.set_n_z_word(word);
                self.pc = self.pc.wrapping_add(2);
                5
            }
            // MOV dp,#nn
            0x8F => mov_addr_byte!(mut_dp!(op0), op1, 3, 5),
            // MOV dp,dp
            0xFA => mov_addr_byte!(mut_dp!(op0), dp!(op1), 3, 5),
            // MOV dp,A
            0xC4 => mov_addr_byte!(mut_dp!(op8), self.a, 2, 4),
            // MOV dp,X
            0xD8 => mov_addr_byte!(mut_dp!(op8), self.x, 2, 4),
            // MOV dp,Y
            0xCB => mov_addr_byte!(mut_dp!(op8), self.y, 2, 4),
            // MOV dp+X,A
            0xD4 => mov_addr_byte!(mut_dp!(op8, self.x), self.a, 2, 5),
            // MOV dp+X,Y
            0xDB => mov_addr_byte!(mut_dp!(op8, self.x), self.y, 2, 5),
            // MOV dp+Y,X
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
            // MOVW dp,YA
            0xDA => {
                let page = (self.psw.p() as u16) << 8;
                bus.write_word(page | (op8 as u16), self.ya());
                self.pc = self.pc.wrapping_add(2);
                5
            }
            // PUSH A
            0x2D => push!(self.a),
            // PUSH X
            0x4D => push!(self.x),
            // PUSH Y
            0x6D => push!(self.y),
            // PUSH Y
            0x0D => push!(self.psw.value),
            // PULL A
            0xAE => pop!(&mut self.a),
            // PULL X
            0xCE => pop!(&mut self.x),
            // PULL Y
            0xEE => pop!(&mut self.y),
            // PULL Y
            0x8E => pop!(&mut self.psw.value),
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
            0x29 => op!(and, mut_dp!(op0), dp!(op1), 3, 6),
            // AND dp,#nn
            0x38 => op!(and, mut_dp!(op0), op1, 3, 5),
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
            0x09 => op!(or, mut_dp!(op0), dp!(op1), 3, 6),
            // OR dp,#nn
            0x18 => op!(or, mut_dp!(op0), op1, 3, 5),
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
            0x49 => op!(eor, mut_dp!(op0), dp!(op1), 3, 6),
            // EOR dp,#nn
            0x58 => op!(eor, mut_dp!(op0), op1, 3, 5),
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
            0x89 => op!(adc, mut_dp!(op0), dp!(op1), 3, 6),
            // ADC dp,#nn
            0x98 => op!(adc, mut_dp!(op0), op1, 3, 5),
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
            0xA9 => op!(sbc, mut_dp!(op0), dp!(op1), 3, 6),
            // SBC dp,#nn
            0xB8 => op!(sbc, mut_dp!(op0), op1, 3, 5),
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
            0x69 => op!(cmp, dp!(op0), dp!(op1), 3, 6),
            // CMP dp,#nn
            0x78 => op!(cmp, dp!(op0), op1, 3, 5),
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
                self.psw.set_n_z_byte(result);
                self.a = result;
                self.pc = self.pc.wrapping_add(1);
                5
            }
            // ADDW YA,dp  affects N,V,H,Z,C
            0x7A => {
                let rhs = dp_word!(op8);
                let result = (self.ya() as u32) + (rhs as u32) + (self.psw.c() as u32);
                addw_subw_end!(rhs, result)
            }
            // SUBW YA,dp  affects N,V,H,Z,C
            0x9A => {
                let rhs = dp_word!(op8);
                let result = (self.ya() as u32)
                    .wrapping_add(!rhs as u32)
                    .wrapping_add(self.psw.c() as u32);
                addw_subw_end!(rhs, result)
            }
            // CMPW YA,dp  affects N,Z,C
            0x5A => {
                let rhs = dp_word!(op8);
                let result = (self.ya() as u32).wrapping_add(!rhs as u32);

                self.psw.set_n_z_c_word(result);

                self.y = (result >> 8) as u8;
                self.a = result as u8;

                self.pc = self.pc.wrapping_add(2);
                5
            }
            // INCW dp
            0x3A => incw_decw!(op8, wrapping_add),
            // DECW dp
            0x1A => incw_decw!(op8, wrapping_sub),
            // MUL  affects N,Z
            0xCF => {
                let result = (self.y as u16) * (self.a as u16);
                self.psw.set_n_z_word(result);
                write_ya!(result);
                self.pc = self.pc.wrapping_add(1);
                9
            }
            // DIV  affects N,V,H,Z
            0x9E => {
                let result_div = self.ya() / (self.x as u16);
                let result_mod = (self.ya() % (self.x as u16)) as u8;

                // From anomie
                self.psw.set_n_z_byte(result_div as u8);
                self.psw.set_v(result_div > 0xFF);
                self.psw.set_h((self.x & 0x0F) <= (self.y & 0x0f));

                self.a = result_div as u8;
                self.y = result_mod;

                self.pc = self.pc.wrapping_add(1);
                12
            }
            // DAA
            0xDF => {
                // From anomie
                if (self.a > 0x99) || self.psw.c() {
                    let result = (self.a as u16) + 0x0060;
                    self.psw.set_c_byte(result);
                    self.a = result as u8;
                }
                if ((self.a & 0x0F) > 0x09) || self.psw.h() {
                    self.a = self.a.wrapping_add(0x06);
                }
                self.psw.set_n_byte(self.a);
                self.psw.set_z_byte(self.a);

                self.pc = self.pc.wrapping_add(1);
                2
            }
            // DAS
            0xBE => {
                // From anomie
                if (self.a > 0x99) || self.psw.c() {
                    let result = (self.a as u16).wrapping_add(!0x0060);
                    self.psw.set_c_byte(result);
                    self.a = result as u8;
                }
                if ((self.a & 0x0F) > 0x09) || self.psw.h() {
                    self.a = self.a.wrapping_sub(0x06);
                }
                self.psw.set_n_byte(self.a);
                self.psw.set_z_byte(self.a);

                self.pc = self.pc.wrapping_add(1);
                2
            }
            // BRA rel
            0x2F => br!(true, op8, 2, 4),
            // BEQ rel
            0xF0 => br!(self.psw.z(), op8, 2, 4),
            // BNE rel
            0xD0 => br!(!self.psw.z(), op8, 2, 4),
            // BCS rel
            0xB0 => br!(self.psw.c(), op8, 2, 4),
            // BCC rel
            0x90 => br!(!self.psw.c(), op8, 2, 4),
            // BVS rel
            0x70 => br!(self.psw.v(), op8, 2, 4),
            // BVS rel
            0x50 => br!(!self.psw.v(), op8, 2, 4),
            // BMI rel
            0x30 => br!(self.psw.n(), op8, 2, 4),
            // BPL rel
            0x10 => br!(!self.psw.n(), op8, 2, 4),
            // BBC d.bit,rel, branches if bit ? in d is cleared
            // bit is defined by op_code
            0x13 | 0x33 | 0x53 | 0x73 | 0x93 | 0xB3 | 0xD3 | 0xF3 => {
                let bit = ((op_code >> 4) - 1) / 2;
                br!(((dp!(op0) >> bit) & 0x01) == 0, op1, 3, 7)
            }
            // BBS d.bit,rel, branches if bit ? in d is set
            // bit is defined by op_code
            0x03 | 0x23 | 0x43 | 0x63 | 0x83 | 0xA3 | 0xC3 | 0xE3 => {
                let bit = (op_code >> 4) / 2;
                br!(((dp!(op0) >> bit) & 0x01) == 1, op1, 3, 7)
            }
            // CBNE dp,rel
            // Note different operator order
            0x2E => br!(self.a != dp!(op1), op0, 3, 7),
            // CBNE dp+X,rel
            // Note different operator order
            0xDE => br!(self.a != dp!(op1, self.x), op0, 3, 8),
            // DBNZ Y,rel
            0xFE => {
                self.y = self.y.wrapping_sub(1);
                br!(self.y != 0, op8, 2, 6)
            }
            // DBNZ dp,rel
            // Note different operator order
            0x6E => {
                let tgt = mut_dp!(op1);
                *tgt = tgt.wrapping_sub(1);
                br!(*tgt != 0, op0, 3, 7)
            }
            // JMP !abs
            0x5F => {
                self.pc = bus.word(op16);
                3
            }
            // JMP [!abs+X]
            0x1F => {
                self.pc = bus.word(op16.wrapping_add(self.x as u16));
                6
            }
            // CALL !abs
            0x3F => call!(op16),
            // PCALL uu
            0x4F => {
                self.pc = 0xFF00 | (op8 as u16);
                6
            }
            // TCALL ?
            // Call to $FFDE-((OPCODE >> 4) * 2)
            0x01 | 0x11 | 0x21 | 0x31 | 0x41 | 0x51 | 0x61 | 0x71 | 0x81 | 0x91 | 0xA1 | 0xB1
            | 0xC1 | 0xD1 | 0xE1 | 0xF1 => call!(0xFFDE - (((op_code >> 4) * 2) as u16)),
            // RET
            0x6F => {
                self.pc = pop_word!();
                5
            }
            // RET1
            0x7F => {
                self.psw.value = pop_byte!();
                self.pc = pop_word!();
                6
            }
            // BRK
            0x0F => {
                push_word!(self.pc.wrapping_add(1));
                push_byte!(self.psw.value);
                self.psw.set_b(true);
                self.psw.set_i(false);
                self.pc = 0xFFDE;
                8
            }
            // CLRC
            0x60 => {
                self.psw.set_c(false);
                self.pc = self.pc.wrapping_add(1);
                2
            }
            // SETC
            0x80 => {
                self.psw.set_c(true);
                self.pc = self.pc.wrapping_add(1);
                2
            }
            // NOTC
            0xED => {
                self.psw.set_c(!self.psw.c());
                self.pc = self.pc.wrapping_add(1);
                3
            }
            // CLRV, also clears H
            0xE0 => {
                self.psw.set_v(false);
                self.psw.set_h(false);
                self.pc = self.pc.wrapping_add(1);
                2
            }
            // CLRP
            0x20 => {
                self.psw.set_p(false);
                self.pc = self.pc.wrapping_add(1);
                2
            }
            // SETP
            0x40 => {
                self.psw.set_p(true);
                self.pc = self.pc.wrapping_add(1);
                2
            }
            // EI
            0xA0 => {
                self.psw.set_i(true);
                self.pc = self.pc.wrapping_add(1);
                3
            }
            // DI
            0xC0 => {
                self.psw.set_i(false);
                self.pc = self.pc.wrapping_add(1);
                3
            }
            // SET1 dp,bit
            // bit is defined by op_code
            0x02 | 0x22 | 0x42 | 0x62 | 0x82 | 0xA2 | 0xC2 | 0xE2 => {
                let bit = (op_code >> 4) / 2;
                let byte = mut_dp!(op8);
                *byte |= 0x1 << bit;
                self.pc = self.pc.wrapping_add(2);
                4
            }
            // CLR1 dp,bit
            // bit is defined by op_code
            0x12 | 0x32 | 0x52 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
                let bit = ((op_code >> 4) - 1) / 2;
                let byte = mut_dp!(op8);
                *byte &= !(0x1 << bit);
                self.pc = self.pc.wrapping_add(2);
                4
            }
            // TCLR1 !abs
            0x4E => {
                let byte = mut_abs!(op16);
                self.psw.set_n_z_byte(self.a.wrapping_sub(*byte));
                *byte &= !self.a;
                self.pc = self.pc.wrapping_add(3);
                6
            }
            // TSET1 !abs
            0x0E => {
                let byte = mut_abs!(op16);
                self.psw.set_n_z_byte(self.a.wrapping_sub(*byte));
                *byte |= self.a;
                self.pc = self.pc.wrapping_add(3);
                6
            }
            // AND1 C,/m.b
            0x6A => bit_op!(&, true,4),
            // AND1 C,m.b
            0x4A => bit_op!(&, false,4),
            // OR1 C,/m.b
            0x2A => bit_op!(|, true,5),
            // OR1 C,m.b
            0x0A => bit_op!(|, false,5),
            // EOR1 C,m.b
            0x8A => bit_op!(^, false,5),
            // NOT1 m.b
            0xEA => {
                let bit = (op16 >> 13) as u8;
                let byte = mut_abs13!(op16);
                if bit_set!(*byte, bit) {
                    *byte &= !(0x1 << bit);
                } else {
                    *byte |= 0x1 << bit;
                }
                self.pc = self.pc.wrapping_add(3);
                5
            }
            // MOV1 C,m.b
            0xAA => {
                let bit = (op16 >> 13) as u8;
                let byte = mut_abs13!(op16);
                self.psw.set_c((*byte >> bit) & 0x1 == 0x1);
                self.pc = self.pc.wrapping_add(3);
                4
            }
            // MOV1 m.b,C
            0xCA => {
                let bit = (op16 >> 13) as u8;
                let byte = mut_abs13!(op16);
                if self.psw.c() {
                    *byte &= !(0x1 << bit);
                } else {
                    *byte |= 0x1 << bit;
                }
                self.pc = self.pc.wrapping_add(3);
                6
            }
            // NOP
            0x00 => {
                self.pc = self.pc.wrapping_add(1);
                2
            }
            // SLEEP
            0xEF => {
                self.pc = self.pc.wrapping_add(1);
                self.mode = Mode::Sleeping;
                0
            }
            // STOP
            0xFF => {
                self.pc = self.pc.wrapping_add(1);
                self.mode = Mode::Stopped;
                0
            }
        };
        Some(op_length)
    }
}

#[derive(PartialEq)]
enum Mode {
    Running,
    Sleeping,
    Stopped,
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
    // TODO: Is this broken or is 0x00 correct?
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

    pub fn set_n_z_byte(&mut self, value: u8) {
        self.set_n_byte(value);
        self.set_z_byte(value);
    }

    pub fn set_n_z_word(&mut self, value: u16) {
        self.set_n_word(value);
        self.set_z_word(value);
    }

    pub fn set_n_z_c_byte(&mut self, value: u16) {
        self.set_n_z_byte(value as u8);
        self.set_c_byte(value);
    }

    pub fn set_n_z_c_word(&mut self, value: u32) {
        self.set_n_z_word(value as u16);
        self.set_c_word(value);
    }

    pub fn set_n_byte(&mut self, value: u8) {
        set_bit!(&mut self.value, P_N, (value & 0x80) > 0);
    }

    pub fn set_n_word(&mut self, value: u16) {
        set_bit!(&mut self.value, P_N, (value & 0x8000) > 0);
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

    pub fn set_z_byte(&mut self, value: u8) {
        set_bit!(&mut self.value, P_Z, value == 0x00);
    }

    pub fn set_z_word(&mut self, value: u16) {
        set_bit!(&mut self.value, P_Z, value == 0x0000);
    }

    pub fn set_c(&mut self, value: bool) {
        set_bit!(&mut self.value, P_C, value);
    }

    pub fn set_c_byte(&mut self, value: u16) {
        set_bit!(&mut self.value, P_C, value > 0xFF);
    }

    pub fn set_c_word(&mut self, value: u32) {
        set_bit!(&mut self.value, P_C, value > 0xFFFF);
    }
}

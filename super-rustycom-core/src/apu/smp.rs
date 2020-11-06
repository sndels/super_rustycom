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
            pc: 0xFFC0,
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
        macro_rules! mut_abs {
            // [aa], [aaaa]
            ($addr:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.access_write(page | $addr as u16)
            }};

            // [aa+X], [aa+Y], [aaaa+X], [aaaa+Y]
            ($addr:expr, $reg:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.access_write(((page | $addr as u16) + $reg as u16) & (page | 0x00FF))
            }};
        }
        macro_rules! abs {
            // [aa], [aaaa]
            ($addr:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.read8(page | $addr as u16)
            }};

            // [aa+X], [aa+Y], [aaaa+X], [aaaa+Y]
            ($addr:expr, $reg:expr) => {{
                let page = (self.psw.p() as u16) << 8;
                bus.read8(((page | $addr as u16) + $reg as u16) & (page | 0x00FF))
            }};
        }
        macro_rules! mut_abs_ptr_y {
            // [[aa]+Y]
            ($addr:expr ) => {
                bus.access_write(bus.read16($addr as u16).wrapping_add(self.y as u16))
            };
        }
        macro_rules! abs_ptr_y {
            ($addr:expr ) => {
                bus.read8(bus.read16($addr as u16).wrapping_add(self.y as u16))
            };
        }
        macro_rules! mut_abs_x_ptr {
            // [[aa+X]]
            ($addr:expr ) => {
                // TODO: Does this wrap with page?
                // Need to do cast after wrapping add if so
                bus.access_write(bus.read16(($addr as u16) + (self.x as u16)))
            };
        }
        macro_rules! abs_x_ptr {
            ($addr:expr ) => {
                bus.read8(bus.read16($addr.wrapping_add(self.x) as u16))
            };
        }

        // Ops
        macro_rules! mov_reg_byte {
            // reg = byte, affects negative, zero flags
            ($reg:expr, $byte:expr, $op_length:expr, $op_cycles:expr) => {{
                *$reg = $byte;
                self.psw.set_n(($byte & 0x80) > 0);
                self.psw.set_z($byte == 0x00);
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
            ($byte:expr) => {{
                *bus.access_write(0x0010 & (self.sp as u16)) = $byte;
                self.sp = self.sp.wrapping_sub(1);
                self.pc = self.pc.wrapping_add(1);
                4
            }};
        }
        macro_rules! pull {
            ($reg:expr) => {{
                self.sp = self.sp.wrapping_add(1);
                *$reg = bus.read8(0x0010 & (self.sp as u16));
                4
            }};
        }

        let op_code = bus.read8(self.pc);
        // Pre-fetch operands for brevity
        let b0 = bus.read8(self.pc.wrapping_add(1));
        let b1 = bus.read8(self.pc.wrapping_add(2));
        let op8 = b0;
        let op16 = ((b1 as u16) << 8) | b0 as u16;

        match op_code {
            // MOV A,nn
            0xE8 => mov_reg_byte!(&mut self.a, op8, 2, 2),
            // MOV X,nn
            0xCD => mov_reg_byte!(&mut self.x, op8, 2, 2),
            // MOV Y,nn
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
            // MOV A,[aa]
            0xE4 => mov_reg_byte!(&mut self.a, abs!(op8), 2, 3),
            // MOV A,[aa+X]
            0xF4 => mov_reg_byte!(&mut self.a, abs!(op8, self.x), 2, 4),
            // MOV A,[aaaa]
            0xE5 => mov_reg_byte!(&mut self.a, abs!(op16), 3, 4),
            // MOV A,[aaaa+X]
            0xF5 => mov_reg_byte!(&mut self.a, abs!(op16, self.x), 3, 5),
            // MOV A,[aaaa+Y]
            0xF6 => mov_reg_byte!(&mut self.a, abs!(op16, self.y), 3, 5),
            // MOV A,[X]
            0xE6 => mov_reg_byte!(&mut self.a, abs!(self.x), 1, 3),
            // MOV A,[X]+
            0xBF => {
                mov_reg_byte!(&mut self.a, abs!(self.x), 1, 4);
                self.x = self.x.wrapping_add(1);
                4
            }
            // MOV A,[[aa]+Y]
            0xF7 => mov_reg_byte!(&mut self.a, abs_ptr_y!(op8), 2, 6),
            // MOV A,[[aa+X]]
            0xE7 => mov_reg_byte!(&mut self.a, abs_x_ptr!(op8), 2, 6),
            // MOV X,[aa]
            0xF8 => mov_reg_byte!(&mut self.x, abs!(op8), 2, 3),
            // MOV X,[aa+Y]
            0xF9 => mov_reg_byte!(&mut self.x, abs!(op8, self.y), 2, 4),
            // MOV X,[aaaa]
            0xE9 => mov_reg_byte!(&mut self.x, abs!(op16), 3, 4),
            // MOV Y,[aa]
            0xEB => mov_reg_byte!(&mut self.y, abs!(op8), 2, 3),
            // MOV Y,[aa+X]
            0xFB => mov_reg_byte!(&mut self.y, abs!(op8, self.x), 2, 4),
            // MOV Y,[aaaa]
            0xEC => mov_reg_byte!(&mut self.y, abs!(op16), 3, 4),
            // MOVW YA,aa
            0xBA => unimplemented!(), // len 2, cycles 5
            // MOV [aa],nn
            0x8F => mov_addr_byte!(mut_abs!(b1), b0, 3, 5),
            // MOV [aa],[bb]
            0xFA => mov_addr_byte!(mut_abs!(b1), abs!(b0), 3, 5),
            // MOV [aa],A
            0xC4 => mov_addr_byte!(mut_abs!(op8), self.a, 2, 4),
            // MOV [aa],X
            0xD8 => mov_addr_byte!(mut_abs!(op8), self.x, 2, 4),
            // MOV [aa],Y
            0xCB => mov_addr_byte!(mut_abs!(op8), self.y, 2, 4),
            // MOV [aa+X],A
            0xD4 => mov_addr_byte!(mut_abs!(op8, self.x), self.a, 2, 5),
            // MOV [aa+X],Y
            0xDB => mov_addr_byte!(mut_abs!(op8, self.x), self.y, 2, 5),
            // MOV [aa+Y],X
            0xD9 => mov_addr_byte!(mut_abs!(op8, self.y), self.x, 2, 5),
            // MOV [aaaa],A
            0xC5 => mov_addr_byte!(mut_abs!(op16), self.a, 3, 5),
            // MOV [aaaa],X
            0xC9 => mov_addr_byte!(mut_abs!(op16), self.x, 3, 5),
            // MOV [aaaa],Y
            0xCC => mov_addr_byte!(mut_abs!(op16), self.y, 3, 5),
            // MOV [aaaa+X],A
            0xD5 => mov_addr_byte!(mut_abs!(op16, self.x), self.a, 3, 6),
            // MOV [aaaa+Y],A
            0xD6 => mov_addr_byte!(mut_abs!(op16, self.y), self.a, 3, 6),
            // MOV [X]+,A
            0xAF => {
                mov_addr_byte!(mut_abs!(self.x), self.a, 1, 4);
                self.x = self.x.wrapping_add(1);
                4
            }
            // MOV [X],A
            0xC6 => mov_addr_byte!(mut_abs!(self.x), self.a, 1, 4),
            // MOV [[aa]+Y],A
            0xD7 => mov_addr_byte!(mut_abs_ptr_y!(op8), self.a, 2, 7),
            // MOV [[aa+X]],A
            0xC7 => mov_addr_byte!(mut_abs_x_ptr!(op8), self.a, 2, 7),
            // MOVW [aa],YA
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
            _ => unimplemented!(),
        }
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

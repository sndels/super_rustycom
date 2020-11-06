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
    pub fn ya(&self) -> u16 {
        self.ya
    }
    pub fn pc(&self) -> u16 {
        self.pc
    }

    /// Returns the value of the negative flag
    pub fn psw_n(&self) -> bool {
        self.psw.n
    }
    /// Returns the value of the overflow flag
    pub fn psw_v(&self) -> bool {
        self.psw.v
    }
    /// Returns the value of the zero page location flag
    pub fn psw_p(&self) -> bool {
        self.psw.p
    }
    /// Returns the value of the break flag
    pub fn psw_b(&self) -> bool {
        self.psw.b
    }
    /// Returns the value of the half carry flag
    pub fn psw_h(&self) -> bool {
        self.psw.h
    }
    /// Returns the value of the interrupt flag
    pub fn psw_i(&self) -> bool {
        self.psw.i
    }
    /// Returns the value of the zero flag
    pub fn psw_z(&self) -> bool {
        self.psw.z
    }
    /// Returns the value of the carry flag
    pub fn psw_c(&self) -> bool {
        self.psw.c
    }

    /// Executes the instruction pointed by `PC` and returns the cycles it took
    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        // Addressing macros return a reference to the byte at the addressed location
        // mut_-versions return mutable reference
        macro_rules! mut_abs {
            // [aa], [aaaa]
            ($addr:expr) => {{
                let page = (self.psw_p() as u16) << 8;
                bus.access_write(page | $addr as u16)
            }};

            // [aa+X], [aa+Y], [aaaa+X], [aaaa+Y]
            ($addr:expr, $reg:expr) => {{
                let page = (self.psw_p() as u16) << 8;
                bus.access_write(((page | $addr as u16) + $reg as u16) & (page | 0x00FF))
            }};
        }
        macro_rules! abs {
            // [aa], [aaaa]
            ($addr:expr) => {{
                let page = (self.psw_p() as u16) << 8;
                bus.read8(page | $addr as u16)
            }};

            // [aa+X], [aa+Y], [aaaa+X], [aaaa+Y]
            ($addr:expr, $reg:expr) => {{
                let page = (self.psw_p() as u16) << 8;
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
                self.psw.n = ($byte & 0x80) > 0;
                self.psw.z = $byte == 0x00;
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
            _ => unimplemented!(),
        }
    }
}

/// The status register in SPC700
#[derive(Clone)]
struct StatusReg {
    /// Negative (0 = positive, 1 = negative)
    n: bool,
    /// Overflow (0 = no overflow, 1 = overflow)
    v: bool,
    /// Zero page location (0 = $00xx, 1 = $01xx)
    p: bool,
    /// Break flag (0 = reset, 1 = BRK)
    b: bool,
    /// Half carry (0 = borrow or no-carry, 1 = carry or no-borrow)
    h: bool,
    /// Interrupt flag (0 = enable, 1 = disable)
    /// No function in SNES APU
    i: bool,
    /// Zero flag (0 = non-zero, 1 = zero)
    z: bool,
    /// Carry flag (0 = no carry, 1 = carry)
    c: bool,
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

impl StatusReg {
    /// Initializes a new instance with default values (`m`, `x` and `i` are set)
    pub fn new() -> StatusReg {
        StatusReg {
            n: false,
            v: false,
            p: false,
            b: false,
            h: false,
            i: false,
            z: false,
            c: false,
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
        if self.p {
            value |= P_P;
        }
        if self.b {
            value |= P_B;
        }
        if self.h {
            value |= P_H;
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
        self.p = value & P_P > 0;
        self.b = value & P_B > 0;
        self.h = value & P_H > 0;
        self.i = value & P_I > 0;
        self.z = value & P_Z > 0;
        self.c = value & P_C > 0;
    }
}

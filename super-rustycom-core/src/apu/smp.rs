use crate::abus::ABus;

struct SPC700 {
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
    pub fn new() -> APU {
        APU {
            a:0x00,
            x:0x00,
            r:0x00,
            y:0x00,
            sp:0x00,
            psw

         }
    }

    pub fn a(&self) -> u8 {
        self.a
    }
    pub fn x(&self) -> u8 {
        self.x
    }
    pub fn sp(&self) -> u8 {
        self.x
    }
    pub fn ya(&self) -> u16 {
        self.ya
    }
    pub fn pc(&self) -> u16 {
        self.pc
    }

    /// Returns the value of the negative flag
    pub fn p_n(&self) -> bool {
        self.p.n
    }
    /// Returns the value of the overflow flag
    pub fn p_v(&self) -> bool {
        self.p.v
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
    pub fn step(&mut self, bus: &mut ApuBus) -> u8 {

    // Addressing macros return a reference to the byte at the addressed location
    // mut_-versions return mutable reference
    macro_rules! mut_abs {
        // [aa], [aaaa]
    ($addr:expr) => {{
        let page = (self.psw_p() as u16) << 8;
        bus.access_mut(page | $addr as u16)
    }};
    // [aa+X], [aa+Y], [aaaa+X], [aaaa+Y]
    ($addr:expr, $reg:expr) => {{
        let page = (self.psw_p() as u16) << 8;
        bus.access_mut(((page | $addr as u16) + $reg as u16) & (page | 0x00FF))
    }};
}
    macro_rules! abs {
    ($addr:expr) => {{
        &*mut_abs!($addr)
    }};
    ($addr:expr, $reg:expr) => {{
        &*mut_abs!($addr, $reg)
    }};
}
    macro_rules! mut_abs_ptr_y {
        // [[aa]+Y]
        ($addr:expr )=> {{
        bus.access_mut(bus.read16($addr).wrapping_add(self.y as u16)))
        }};
    }
    macro_rules! abs_ptr_y {
        ($addr:expr )=> {{
        abs_ptr_y!($addr)
        }};
    }
    macro_rules! mut_abs_x_ptr {
        // [[aa+X]]
        ($addr:expr )=> {{
        bus.access_mut(bus.read16($addr.wrapping_add(self.x) as u16))
        }};
    }
    macro_rules! abs_x_ptr {
        ($addr:expr )=> {{
        abs_x_ptr!($addr)
        }};
    }

    // Ops
    macro_rules! mov_reg_byte {
        // reg = byte, affects negative, zero flags
        ($reg:expr, $byte:expr, $op_length:expr) => {
            *lhs = byte;
            self.psw.n = byte& 0x80;
            self.psw.n = byte== 0x00;
            $op_length
        }
    }
    macro_rules! mov_addr_byte {
        // [addr] = byte, doesn't affect flags
        ($addr:expr, $byte:expr, $op_length:expr) => {
            *bus.access_mut(addr) = byte;
            $op_length
        }
    }


    let op_code = bus.access(self.pc);
    // Pre-fetch operands for brevity
    let b0 =
        bus.access(self.pc.wrapping_add(1));
    let b1=
        bus.access(self.pc.wrapping_add(2));
    let op8 = b0;
    let op16 =
        ((b1 as u16) << 8)| b0as u16;

    match op_code {
        // MOV A,nn
        0xE8 => mov_reg_byte!(self.a, op8,2),
        // MOV X,nn
        0xCD => mov_reg_byte!(self.x, op8,2),
        // MOV Y,nn
        0x8D => mov_reg_byte!(self.x, op8,2),
        // MOV A,X
        0x7D => mov_reg_byte!(self.a, self.x,2),
        // MOV X,A
        0x5D => mov_reg_byte!(self.x, self.a,2),
        // MOV A,Y
        0xDD => mov_reg_byte!(self.a, self.y,2),
        // MOV Y,A
        0xFD => mov_reg_byte!(self.y, self.a,2),
        // MOV X,SP
        0x9D => mov_reg_byte!(self.x, self.sp,2),
        // MOV SP,X
        0xBD => mov_reg_byte!(self.sp, self.x,2),
        // MOV A,[aa]
        0xE4 => mov_reg_byte!(self.a, abs!(op8),3),
        // MOV A,[aa+X]
        0xE5 => mov_reg_byte!(self.a, abs!(op8, self.x), 4),
        // MOV A,[aaaa]
        0xE5 => mov_reg_byte!(self.a, abs!(op16),4),
        // MOV A,[aaaa+X]
        0xF5 => mov_reg_byte!(self.a, abs!(op16, self.x),5),
        // MOV A,[aaaa+Y]
        0xF6 => mov_reg_byte!(self.a, abs!(op16, self.y),5),
        // MOV A,[X]
        0xE6 => mov_reg_byte!(self.a, abs!(self.x),3),
        // MOV A,[X]+
        0xBF => {mov_reg_byte!(self.a, abs!(self.x), 4);self.x.wrapping_add(1); 4},
        // MOV A,[[aa]+Y]
        0xF7 => mov_reg_byte!(self.a, abs_ptr_y!(op8),6),
        // MOV A,[[aa+X]]
        0xE7 => mov_reg_byte!(self.a, abs_x_ptr!(op8),6),
        // MOV X,[aa]
        0xF8 => mov_reg_byte!(self.x, abs!(op8),3),
        // MOV X,[aa+Y]
        0xF9 => mov_reg_byte!(self.x, abs!(op8, self.y),4),
        // MOV X,[aaaa]
        0xE9 => mov_reg_byte!(self.x, abs!(op16),4),
        // MOV Y,[aa]
        0xEB => mov_reg_byte!(self.y, abs!(op8),3),
        // MOV Y,[aa+X]
        0xFB => mov_reg_byte!(self.y, abs!(op8, self.x),4),
        // MOV Y,[aaaa]
        0xEC => mov_reg_byte!(self.y, abs!(op16),4),
        // MOVW YA,aa
        0xBA => unimplemented!()// len 5,
        // MOV [aa],nn
        0x8F => mov_addr_byte!(mut_abs!(b1), b0,5),
        // MOV [aa],[bb]
        0xFA => mov_addr_byte!(mut_abs!(b1), mut_abs!(b0),5),
        // MOV [aa],A
        0xC4 => mov_addr_byte!(mut_abs!(op8), self.a,4),
        // MOV [aa],X
        0xD8 => mov_addr_byte!(mut_abs!(op8), self.x,4),
        // MOV [aa],Y
        0xCB => mov_addr_byte!(mut_abs!(op8), self.y,4),
        // MOV [aa+X],A
        0xD4 => mov_addr_byte!(mut_abs!(op8, self.x), self.a,5),
        // MOV [aa+X],Y
        0xDB => mov_addr_byte!(mut_abs!(op8, self.x), self.y,5),
        // MOV [aa+Y],X
        0xD9 => mov_addr_byte!(mut_abs!(op8, self.y), self.x,5),
        // MOV [aaaa],A
        0xC5 => mov_addr_byte!(mut_abs!(op16), self.a,5),
        // MOV [aaaa],X
        0xC9 => mov_addr_byte!(mut_abs!(op16), self.x,5),
        // MOV [aaaa],Y
        0xCC => mov_addr_byte!(mut_abs!(op16), self.y,5),
        // MOV [aaaa+X],A
        0xD5 => mov_addr_byte!(mut_abs!(op16, self.x), self.a,6),
        // MOV [aaaa+Y],A
        0xD6 => mov_addr_byte!(mut_abs!(op16, self.y), self.a,6),
        // MOV [X]+,A
        0xAF => {mov_addr_byte!(mut_abs!(self.x), self.a,4);self.x.wrapping_add(1);4},
        // MOV [X],A
        0xC6 => mov_addr_byte!(mut_abs!(self.x), self.a,4),
        // MOV [[aa]+Y],A
        0xD7 => mov_addr_byte!(mut_abs_ptr_y!(op8), self.a,7),
        // MOV [[aa+X]],A
        0xC7 => mov_addr_byte!(mut_abs_x_ptr!(op8), self.a,7),
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
        if self.m {
            value |= P_P;
        }
        if self.x {
            value |= P_B;
        }
        if self.d {
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


// At $FFC0-$FFFF
// Takes care of init and (initial) data transfer, though ROMs leverage transfer again by jumping to $FFC0
const IPL_ROM = [
    0xCD, 0xEF,
    0xBD,
    0xE8, 0x00,
    0xC6,
    0x1D,
    0xD0, 0xFC,
    0x8F, 0xAA,0xF4
    0x8F, 0xBB,0xF5
    0x78, 0xCC,0xF4
    0xD0, 0xFB,
    0x2F, 0x19,
    0xEB, 0xF4,
    0xD0, 0xFC,
    0x7E, 0xF4,
    0xD0, 0x0B,
    0xE4, 0xF5,
    0xCB, 0xF4,
    0xD7, 0x00,
    0xFC,
    0xD0, 0xF3,
    0xAB, 0x01,
    0x10, 0xEF,
    0x7E, 0xF4,
    0x10, 0xEB,
    0xBA, 0xF6,
    0xDA, 0x00,
    0xBA, 0xF4,
    0xC4, 0xF4,
    0xDD,
    0x5D,
    0xD0, 0xDB,
    0x1F, 0x00,0x00
    0xC0, 0xFF,
];

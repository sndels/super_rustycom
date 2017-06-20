pub struct Cpu {
    a:  u16,  // Accumulator
    x:  u16,  // Index register
    y:  u16,  // Index register
    pc: u16,  // Program counter
    s:  u16,  // Stack pointer
    p:  u8,   // Processor status register
    d:  u16,  // Zeropage offset
    pb: u8,   // Data bank
    db: u8,   // Program cunter bank
    e:  bool, // Emulation mode
}

struct ABus {
    wram:   [u8; 128000],
    vram:   [u8; 64000],
    oam:    [u8; 544],
    cgram:  [u8; 512],
    // TODO: ROM
    // TODO: PPU1,2
    // TODO: PPU control regs
    // TODO: APU com regs
    // TODO: Joypad
    // TODO: Math regs
    // TODO: H-/V-blank regs and timers
    // TODO: DMA + control regs
}

// Memory map
pub const WS1_SYSLR_FIRST_BANK: usize = 0x00;
pub const WS1_SYSLR_LAST_BANK: usize  = 0x3F;

pub const WS1_HIROM_FIRST_BANK: usize = 0x40;
pub const WS1_HIROM_LAST_BANK: usize  = 0x7D;

pub const WRAM_FIRST_BANK: usize      = 0x7E;
pub const WRAM_LAST_BANK: usize       = 0x7F;

pub const WS2_SYSLR_FIRST_BANK: usize = 0x80;
pub const WS2_SYSLR_LAST_BANK: usize  = 0xBF;

pub const WS2_HIROM_FIRST_BANK: usize = 0xC0;
pub const WS2_HIROM_LAST_BANK: usize  = 0xFF;

// Map of the shared system and LoROM -banks
pub const SYS_FIRST: usize   = 0x0000;
pub const SYS_LAST: usize    = 0x7FFF;
pub const LOROM_FIRST: usize = 0x8000;
pub const LOROM_LAST: usize  = 0xFFFF;

// System area map
pub const WRAM_MIRR_FIRST: usize = 0x0000;
pub const WRAM_MIRR_LAST: usize  = 0x1FFF;
pub const PPU_IO_FIRST: usize    = 0x2100;
pub const PPU_IO_LAST: usize     = 0x213F;
pub const APU_IO_FIRST: usize    = 0x2140;
pub const APU_IO_LAST: usize     = 0x2147;
pub const WMDATA: usize          = 0x2180; // R/W
pub const WMADDL: usize          = 0x2181; // W
pub const WMADDM: usize          = 0x2182; // W
pub const WMADDH: usize          = 0x2183; // W
pub const JOYWR: usize           = 0x4016; // CPU W
pub const JOYA: usize            = 0x4016; // CPU R
pub const JOYB: usize            = 0x4017; // CPU R
// CPU W
pub const NMITIMEN: usize        = 0x4200;
pub const WRIO: usize            = 0x4201;
pub const WRMPYA: usize          = 0x4202;
pub const WRMPYB: usize          = 0x4203;
pub const WRDIVL: usize          = 0x4204;
pub const WRDIVH: usize          = 0x4205;
pub const WRDIVB: usize          = 0x4206;
pub const HTIMEL: usize          = 0x4207;
pub const HTIMEH: usize          = 0x4208;
pub const VTIMEL: usize          = 0x4209;
pub const VTIMEH: usize          = 0x420A;
pub const MDMAEN: usize          = 0x420B;
pub const HDMAEN: usize          = 0x420C;
pub const MEMSEL: usize          = 0x420D;
// CPU R
pub const RDNMI: usize           = 0x4210;
pub const TIMEUP: usize          = 0x4211;
pub const HVBJOY: usize          = 0x4212;
pub const RDIO: usize            = 0x4213;
pub const RDDIVL: usize          = 0x4214;
pub const RDDIVH: usize          = 0x4215;
pub const RDMPYL: usize          = 0x4216;
pub const RDMPYH: usize          = 0x4217;
pub const JOY1L: usize           = 0x4218;
pub const JOY1H: usize           = 0x4219;
pub const JOY2L: usize           = 0x421A;
pub const JOY2H: usize           = 0x421B;
pub const JOY3L: usize           = 0x421C;
pub const JOY3H: usize           = 0x421D;
pub const JOY4L: usize           = 0x421E;
pub const JOY4H: usize           = 0x421F;
// CPU DMA R/W
pub const DMA_FIRST: usize       = 0x4300;
pub const DMA_LAST: usize        = 0x43FF;
// Expansion
pub const EXP_FIRST: usize       = 0x6000;
pub const EXP_LAST: usize        = 0x7FFF;

// PPU IO, relative addresses
// CPU W
pub const INIDISP: usize = 0x00;
pub const OBSEL: usize   = 0x01;
pub const OAMADDL: usize = 0x02;
pub const OAMADDH: usize = 0x03;
pub const OAMDATA: usize = 0x04;
pub const BGMODE: usize  = 0x05;
pub const MOSAIC: usize  = 0x06;
pub const BG1SC: usize   = 0x07;
pub const BG2SC: usize   = 0x08;
pub const BG3SC: usize   = 0x09;
pub const BG4SC: usize   = 0x0A;
pub const BG12NBA: usize = 0x0B;
pub const BG34NBA: usize = 0x0C;
pub const BG1HOFS: usize = 0x0D;
pub const BG1VOFS: usize = 0x0E;
pub const BG2HOFS: usize = 0x0F;
pub const BG2VOFS: usize = 0x10;
pub const BG3HOFS: usize = 0x11;
pub const BG3VOFS: usize = 0x12;
pub const BG4HOFS: usize = 0x13;
pub const BG4VOFS: usize = 0x14;
pub const VMAIN: usize   = 0x15;
pub const VMADDL: usize  = 0x16;
pub const VMADDH: usize  = 0x17;
pub const VMDATAL: usize = 0x18;
pub const VMDATAH: usize = 0x19;
pub const M7SEL: usize   = 0x1A;
pub const M7A: usize     = 0x1B;
pub const M7B: usize     = 0x1C;
pub const M7C: usize     = 0x1D;
pub const M7D: usize     = 0x1E;
pub const M7X: usize     = 0x1F;
pub const M7Y: usize     = 0x20;
pub const CGADD: usize   = 0x21;
pub const CGDATA: usize  = 0x22;
pub const W12SEL: usize  = 0x23;
pub const W34SEL: usize  = 0x24;
pub const WOBJSEL: usize = 0x25;
pub const WH0: usize     = 0x26;
pub const WH1: usize     = 0x27;
pub const WH2: usize     = 0x28;
pub const WH3: usize     = 0x29;
pub const WBGLOG: usize  = 0x2A;
pub const WOBJLOG: usize = 0x2B;
pub const TM: usize      = 0x2C;
pub const TS: usize      = 0x2D;
pub const TMW: usize     = 0x2E;
pub const TSW: usize     = 0x2F;
pub const CGWSEL: usize  = 0x30;
pub const CGADSUB: usize = 0x31;
pub const COLDATA: usize = 0x32;
pub const SETINI: usize  = 0x33;
// CPU R
pub const MPYL: usize    = 0x34;
pub const MPYM: usize    = 0x35;
pub const MPYH: usize    = 0x36;
pub const SLHV: usize    = 0x37;
pub const RDOAM: usize   = 0x38;
pub const RDVRAML: usize = 0x39;
pub const RDVRAMH: usize = 0x3A;
pub const RDCGRAM: usize = 0x3B;
pub const OPHCT: usize   = 0x3C;
pub const OPVCT: usize   = 0x3D;
pub const STAT77: usize  = 0x3E;
pub const STAT78: usize  = 0x3F;

use abus::ABus;

// Memory map
pub const WS1_SYSLR_FIRST_BANK: usize = 0x00;
pub const WS1_SYSLR_LAST_BANK: usize = 0x3F;

pub const WS1_HIROM_FIRST_BANK: usize = 0x40;
pub const WS1_HIROM_LAST_BANK: usize = 0x7D;

pub const WRAM_FIRST_BANK: usize = 0x7E;
pub const WRAM_LAST_BANK: usize = 0x7F;

pub const WS2_SYSLR_FIRST_BANK: usize = 0x80;
pub const WS2_SYSLR_LAST_BANK: usize = 0xBF;

pub const WS2_HIROM_FIRST_BANK: usize = 0xC0;
pub const WS2_HIROM_LAST_BANK: usize = 0xFF;

// Map of the shared system and LoROM -banks
pub const SYS_FIRST: usize = 0x0000;
pub const SYS_LAST: usize = 0x7FFF;
pub const LOROM_FIRST: usize = 0x8000;
pub const LOROM_LAST: usize = 0xFFFF;

// System area map
pub const WRAM_MIRR_FIRST: usize = 0x0000;
pub const WRAM_MIRR_LAST: usize = 0x1FFF;
pub const PPU_IO_FIRST: usize = 0x2100;
pub const PPU_IO_LAST: usize = 0x213F;
pub const APU_IO_FIRST: usize = 0x2140;
pub const APU_IO_LAST: usize = 0x2147;
pub const WMDATA: usize = 0x2180; // R/W
pub const WMADDL: usize = 0x2181; // W
pub const WMADDM: usize = 0x2182; // W
pub const WMADDH: usize = 0x2183; // W
pub const JOYWR: usize = 0x4016; // CPU W
pub const JOYA: usize = 0x4016; // CPU R
pub const JOYB: usize = 0x4017; // CPU R
                                // CPU W
pub const NMITIMEN: usize = 0x4200;
pub const WRIO: usize = 0x4201;
pub const WRMPYA: usize = 0x4202;
pub const WRMPYB: usize = 0x4203;
pub const WRDIVL: usize = 0x4204;
pub const WRDIVH: usize = 0x4205;
pub const WRDIVB: usize = 0x4206;
pub const HTIMEL: usize = 0x4207;
pub const HTIMEH: usize = 0x4208;
pub const VTIMEL: usize = 0x4209;
pub const VTIMEH: usize = 0x420A;
pub const MDMAEN: usize = 0x420B;
pub const HDMAEN: usize = 0x420C;
pub const MEMSEL: usize = 0x420D;
// CPU R
pub const RDNMI: usize = 0x4210;
pub const TIMEUP: usize = 0x4211;
pub const HVBJOY: usize = 0x4212;
pub const RDIO: usize = 0x4213;
pub const RDDIVL: usize = 0x4214;
pub const RDDIVH: usize = 0x4215;
pub const RDMPYL: usize = 0x4216;
pub const RDMPYH: usize = 0x4217;
pub const JOY1L: usize = 0x4218;
pub const JOY1H: usize = 0x4219;
pub const JOY2L: usize = 0x421A;
pub const JOY2H: usize = 0x421B;
pub const JOY3L: usize = 0x421C;
pub const JOY3H: usize = 0x421D;
pub const JOY4L: usize = 0x421E;
pub const JOY4H: usize = 0x421F;
// CPU DMA R/W
pub const DMA_FIRST: usize = 0x4300;
pub const DMA_LAST: usize = 0x43FF;
// Expansion
pub const EXP_FIRST: usize = 0x6000;
pub const EXP_LAST: usize = 0x7FFF;

// PPU IO
// CPU W
pub const INIDISP: usize = 0x2100;
pub const OBSEL: usize = 0x2101;
pub const OAMADDL: usize = 0x2102;
pub const OAMADDH: usize = 0x2103;
pub const OAMDATA: usize = 0x2104;
pub const BGMODE: usize = 0x2105;
pub const MOSAIC: usize = 0x2106;
pub const BG1SC: usize = 0x2107;
pub const BG2SC: usize = 0x2108;
pub const BG3SC: usize = 0x2109;
pub const BG4SC: usize = 0x210A;
pub const BG12NBA: usize = 0x210B;
pub const BG34NBA: usize = 0x210C;
pub const BG1HOFS: usize = 0x210D;
pub const BG1VOFS: usize = 0x210E;
pub const BG2HOFS: usize = 0x210F;
pub const BG2VOFS: usize = 0x2110;
pub const BG3HOFS: usize = 0x2111;
pub const BG3VOFS: usize = 0x2112;
pub const BG4HOFS: usize = 0x2113;
pub const BG4VOFS: usize = 0x2114;
pub const VMAIN: usize = 0x2115;
pub const VMADDL: usize = 0x2116;
pub const VMADDH: usize = 0x2117;
pub const VMDATAL: usize = 0x2118;
pub const VMDATAH: usize = 0x2119;
pub const M7SEL: usize = 0x211A;
pub const M7A: usize = 0x211B;
pub const M7B: usize = 0x211C;
pub const M7C: usize = 0x211D;
pub const M7D: usize = 0x211E;
pub const M7X: usize = 0x211F;
pub const M7Y: usize = 0x2120;
pub const CGADD: usize = 0x2121;
pub const CGDATA: usize = 0x2122;
pub const W12SEL: usize = 0x2123;
pub const W34SEL: usize = 0x2124;
pub const WOBJSEL: usize = 0x2125;
pub const WH0: usize = 0x2126;
pub const WH1: usize = 0x2127;
pub const WH2: usize = 0x2128;
pub const WH3: usize = 0x2129;
pub const WBGLOG: usize = 0x212A;
pub const WOBJLOG: usize = 0x212B;
pub const TM: usize = 0x212C;
pub const TS: usize = 0x212D;
pub const TMW: usize = 0x212E;
pub const TSW: usize = 0x212F;
pub const CGWSEL: usize = 0x2130;
pub const CGADSUB: usize = 0x2131;
pub const COLDATA: usize = 0x2132;
pub const SETINI: usize = 0x2133;
// CPU R
pub const MPYL: usize = 0x2134;
pub const MPYM: usize = 0x2135;
pub const MPYH: usize = 0x2136;
pub const SLHV: usize = 0x2137;
pub const RDOAM: usize = 0x2138;
pub const RDVRAML: usize = 0x2139;
pub const RDVRAMH: usize = 0x213A;
pub const RDCGRAM: usize = 0x213B;
pub const OPHCT: usize = 0x213C;
pub const OPVCT: usize = 0x213D;
pub const STAT77: usize = 0x213E;
pub const STAT78: usize = 0x213F;

// APU IO
pub const APUI00: usize = 0x2140;
pub const APUI01: usize = 0x2141;
pub const APUI02: usize = 0x2142;
pub const APUI03: usize = 0x2143;

pub fn cpu_read_sys(abus: &mut ABus, addr: usize) -> u8 {
    match addr {
        WRAM_MIRR_FIRST...WRAM_MIRR_LAST => abus.wram[addr],
        PPU_IO_FIRST...PPU_IO_LAST => {
            // PPU IO
            if addr < MPYL {
                panic!("Read ${:06X}: PPU IO write-only for cpu", addr);
            }
            abus.ppu_io.read(addr)
        }
        APU_IO_FIRST...APU_IO_LAST => {
            // APU IO
            let apu_port = if addr < 0x2144 { addr } else { addr - 4 };
            match apu_port {
                APUI00 => (abus.apu_io0 >> 8) as u8,
                APUI01 => (abus.apu_io1 >> 8) as u8,
                APUI02 => (abus.apu_io2 >> 8) as u8,
                APUI03 => (abus.apu_io3 >> 8) as u8,
                _ => unreachable!(),
            }
        }
        WMDATA => {
            // Get current address
            let wram_addr = (((abus.wm_add_h & 0x1) as usize) << 16)
                | ((abus.wm_add_m as usize) << 8)
                | (abus.wm_add_l as usize);
            // Increment address
            abus.wm_add_l = abus.wm_add_l.wrapping_add(1);
            if abus.wm_add_l == 0x00 {
                abus.wm_add_m = abus.wm_add_m.wrapping_add(1);
                if abus.wm_add_m == 0x00 {
                    abus.wm_add_h = abus.wm_add_h.wrapping_add(1);
                    if abus.wm_add_h & 0x01 == 0x00 {
                        abus.wm_add_l = 0x01;
                    }
                }
            }
            // Return data at the original address
            abus.wram[wram_addr]
        }
        JOYA => abus.joy_io.joy_a(),
        JOYB => abus.joy_io.joy_b(),
        RDNMI => {
            let val = abus.rd_nmi;
            abus.rd_nmi &= 0b0111_1111;
            val
        }
        TIMEUP => {
            let val = abus.time_up;
            abus.time_up &= 0b0111_1111;
            val
        }
        HVBJOY => abus.hvb_joy,
        RDIO => abus.joy_io.rd_io(),
        RDDIVL => abus.mpy_div.get_div_res_low(),
        RDDIVH => abus.mpy_div.get_div_res_high(),
        RDMPYL => abus.mpy_div.get_mpy_res_low(),
        RDMPYH => abus.mpy_div.get_mpy_res_high(),
        JOY1L => abus.joy_io.joy_1l(),
        JOY1H => abus.joy_io.joy_1h(),
        JOY2L => abus.joy_io.joy_2l(),
        JOY2H => abus.joy_io.joy_2h(),
        JOY3L => abus.joy_io.joy_3l(),
        JOY3H => abus.joy_io.joy_3h(),
        JOY4L => abus.joy_io.joy_4l(),
        JOY4H => abus.joy_io.joy_4h(),
        DMA_FIRST...DMA_LAST => {
            // DMA
            abus.dma.read(addr)
        }
        EXP_FIRST...EXP_LAST => {
            // Expansion
            panic!("Read ${:06X}: Expansion not implemented", addr)
        }
        _ => panic!(
            "System area read ${:04X}: Address unused or write-only",
            addr
        ),
    }
}

pub fn cpu_read8(abus: &mut ABus, addr: u32) -> u8 {
    let bank = (addr >> 16) as usize;
    let bank_addr = (addr & 0x00FFFF) as usize;
    match bank {
        WS1_SYSLR_FIRST_BANK...WS1_SYSLR_LAST_BANK => match bank_addr {
            SYS_FIRST...SYS_LAST => cpu_read_sys(abus, bank_addr),
            LOROM_FIRST...LOROM_LAST => abus.rom.read_ws1_lo_rom8(bank, bank_addr),
            _ => unreachable!(),
        },
        WS1_HIROM_FIRST_BANK...WS1_HIROM_LAST_BANK => abus.rom.read_ws1_hi_rom8(bank, bank_addr),
        WRAM_FIRST_BANK...WRAM_LAST_BANK => {
            abus.wram[(bank - WRAM_FIRST_BANK) * 0x10000 + bank_addr]
        }
        WS2_SYSLR_FIRST_BANK...WS2_SYSLR_LAST_BANK => match bank_addr {
            SYS_FIRST...SYS_LAST => cpu_read_sys(abus, bank_addr),
            LOROM_FIRST...LOROM_LAST => abus.rom.read_ws2_lo_rom8(bank, bank_addr),
            _ => unreachable!(),
        },
        WS2_HIROM_FIRST_BANK...WS2_HIROM_LAST_BANK => abus.rom.read_ws2_hi_rom8(bank, bank_addr),
        _ => unreachable!(),
    }
}

#[allow(dead_code)]
pub fn apu_read8(abus: &mut ABus, reg: u8) -> u8 {
    match reg {
        0x00 => abus.apu_io0 as u8,
        0x01 => abus.apu_io1 as u8,
        0x02 => abus.apu_io2 as u8,
        0x03 => abus.apu_io3 as u8,
        _ => panic!("APU read from invalid IO register!"),
    }
}

pub fn addr_wrapping_cpu_read16(abus: &mut ABus, addr: u32) -> u16 {
    cpu_read8(abus, addr) as u16 | ((cpu_read8(abus, addr_wrapping_add(addr, 1)) as u16) << 8)
}

#[allow(dead_code)]
pub fn addr_wrapping_cpu_read24(abus: &mut ABus, addr: u32) -> u32 {
    cpu_read8(abus, addr) as u32 | ((cpu_read8(abus, addr_wrapping_add(addr, 1)) as u32) << 8)
        | ((cpu_read8(abus, addr_wrapping_add(addr, 2)) as u32) << 16)
}

pub fn bank_wrapping_cpu_read16(abus: &mut ABus, addr: u32) -> u16 {
    cpu_read8(abus, addr) as u16 | ((cpu_read8(abus, bank_wrapping_add(addr, 1)) as u16) << 8)
}

pub fn bank_wrapping_cpu_read24(abus: &mut ABus, addr: u32) -> u32 {
    cpu_read8(abus, addr) as u32 | ((cpu_read8(abus, bank_wrapping_add(addr, 1)) as u32) << 8)
        | ((cpu_read8(abus, bank_wrapping_add(addr, 2)) as u32) << 16)
}

pub fn page_wrapping_cpu_read16(abus: &mut ABus, addr: u32) -> u16 {
    cpu_read8(abus, addr) as u16 | ((cpu_read8(abus, page_wrapping_add(addr, 1)) as u16) << 8)
}

#[allow(dead_code)]
pub fn page_wrapping_cpu_read24(abus: &mut ABus, addr: u32) -> u32 {
    cpu_read8(abus, addr) as u32 | ((cpu_read8(abus, page_wrapping_add(addr, 1)) as u32) << 8)
        | ((cpu_read8(abus, page_wrapping_add(addr, 2)) as u32) << 16)
}

pub fn fetch_operand8(abus: &mut ABus, addr: u32) -> u8 {
    cpu_read8(abus, bank_wrapping_add(addr, 1))
}

pub fn fetch_operand16(abus: &mut ABus, addr: u32) -> u16 {
    bank_wrapping_cpu_read16(abus, bank_wrapping_add(addr, 1))
}

pub fn fetch_operand24(abus: &mut ABus, addr: u32) -> u32 {
    bank_wrapping_cpu_read24(abus, bank_wrapping_add(addr, 1))
}

fn cpu_write_sys(abus: &mut ABus, addr: usize, value: u8) {
    match addr {
        WRAM_MIRR_FIRST...WRAM_MIRR_LAST => {
            // WRAM
            abus.wram[addr] = value
        }
        PPU_IO_FIRST...PPU_IO_LAST => {
            // PPU IO
            if addr > SETINI {
                panic!("Write ${:06X}: PPU IO read-only for cpu", addr);
            }
            abus.ppu_io.write(addr, value);
        }
        APU_IO_FIRST...APU_IO_LAST => {
            // APU IO
            let apu_port = if addr < 0x2144 { addr } else { addr - 4 };
            match apu_port {
                APUI00 => abus.apu_io0 = (abus.apu_io0 & 0xFF00) | value as u16,
                APUI01 => abus.apu_io1 = (abus.apu_io1 & 0xFF00) | value as u16,
                APUI02 => abus.apu_io2 = (abus.apu_io2 & 0xFF00) | value as u16,
                APUI03 => abus.apu_io3 = (abus.apu_io3 & 0xFF00) | value as u16,
                _ => unreachable!(),
            }
        }
        WMDATA => {
            let wram_addr = ((abus.wm_add_h as usize) << 16) | ((abus.wm_add_m as usize) << 8)
                | (abus.wm_add_l as usize);
            abus.wram[wram_addr] = value;
        }
        WMADDL => abus.wm_add_l = value,
        WMADDM => abus.wm_add_m = value,
        WMADDH => abus.wm_add_h = value,
        JOYWR => abus.joy_io.set_joy_wr(value),
        NMITIMEN => abus.nmitimen = value,
        WRIO => abus.joy_io.set_wr_io(value),
        WRMPYA => abus.mpy_div.set_multiplicand(value),
        WRMPYB => abus.mpy_div.set_multiplier_and_start_multiply(value),
        WRDIVL => abus.mpy_div.set_dividend_low(value),
        WRDIVH => abus.mpy_div.set_dividend_high(value),
        WRDIVB => abus.mpy_div.set_divisor_and_start_division(value),
        HTIMEL => abus.htime = (abus.htime & 0xFF00) | value as u16,
        HTIMEH => abus.htime = ((value as u16) << 8) | (abus.htime & 0x00FF),
        VTIMEL => abus.vtime = (abus.vtime & 0xFF00) | value as u16,
        VTIMEH => abus.vtime = ((value as u16) << 8) | (abus.vtime & 0x00FF),
        MDMAEN => abus.dma.write_mdma_en(value),
        HDMAEN => abus.dma.write_hdma_en(value),
        MEMSEL => abus.memsel = value,
        DMA_FIRST...DMA_LAST => {
            // DMA
            abus.dma.write(addr, value);
        }
        EXP_FIRST...EXP_LAST => {
            // Expansion
            panic!("Write ${:06X}: Expansion not implemented", addr)
        }
        _ => panic!(
            "System area write ${:04X}: Address unused or read-only",
            addr
        ),
    }
}

pub fn cpu_write8(abus: &mut ABus, addr: u32, value: u8) {
    let bank = (addr >> 16) as usize;
    let bank_addr = (addr & 0x00FFFF) as usize;
    match bank {
        WS1_SYSLR_FIRST_BANK...WS1_SYSLR_LAST_BANK => match bank_addr {
            SYS_FIRST...SYS_LAST => cpu_write_sys(abus, bank_addr, value),
            LOROM_FIRST...LOROM_LAST => {
                abus.rom.write_ws1_lo_rom8(bank, bank_addr, value);
            }
            _ => unreachable!(),
        },
        WS1_HIROM_FIRST_BANK...WS1_HIROM_LAST_BANK => {
            abus.rom.write_ws1_hi_rom8(bank, bank_addr, value);
        }
        WRAM_FIRST_BANK...WRAM_LAST_BANK => {
            abus.wram[(bank - WRAM_FIRST_BANK) * 0x10000 + bank_addr] = value;
        }
        WS2_SYSLR_FIRST_BANK...WS2_SYSLR_LAST_BANK => match bank_addr {
            SYS_FIRST...SYS_LAST => cpu_write_sys(abus, bank_addr, value),
            LOROM_FIRST...LOROM_LAST => {
                abus.rom.write_ws2_lo_rom8(bank, bank_addr, value);
            }
            _ => unreachable!(),
        },
        WS2_HIROM_FIRST_BANK...WS2_HIROM_LAST_BANK => {
            abus.rom.write_ws2_hi_rom8(bank, bank_addr, value);
        }
        _ => unreachable!(),
    }
}

pub fn apu_write8(abus: &mut ABus, reg: u8, value: u8) {
    match reg {
        0x00 => abus.apu_io0 = ((value as u16) << 8) | (abus.apu_io0 & 0xFF),
        0x01 => abus.apu_io1 = ((value as u16) << 8) | (abus.apu_io1 & 0xFF),
        0x02 => abus.apu_io2 = ((value as u16) << 8) | (abus.apu_io2 & 0xFF),
        0x03 => abus.apu_io3 = ((value as u16) << 8) | (abus.apu_io3 & 0xFF),
        _ => panic!("APU write to invalid IO register!"),
    }
}

pub fn addr_wrapping_cpu_write16(abus: &mut ABus, addr: u32, value: u16) {
    cpu_write8(abus, addr, value as u8);
    cpu_write8(abus, addr_wrapping_add(addr, 1), (value >> 8) as u8);
}

#[allow(dead_code)]
pub fn addr_wrapping_cpu_write24(abus: &mut ABus, addr: u32, value: u32) {
    cpu_write8(abus, addr, value as u8);
    cpu_write8(abus, addr_wrapping_add(addr, 1), (value >> 8) as u8);
    cpu_write8(abus, addr_wrapping_add(addr, 2), (value >> 16) as u8);
}

pub fn bank_wrapping_cpu_write16(abus: &mut ABus, addr: u32, value: u16) {
    cpu_write8(abus, addr, value as u8);
    cpu_write8(abus, bank_wrapping_add(addr, 1), (value >> 8) as u8);
}

#[allow(dead_code)]
pub fn bank_wrapping_cpu_write24(abus: &mut ABus, addr: u32, value: u32) {
    cpu_write8(abus, addr, value as u8);
    cpu_write8(abus, bank_wrapping_add(addr, 1), (value >> 8) as u8);
    cpu_write8(abus, bank_wrapping_add(addr, 2), (value >> 16) as u8);
}

pub fn page_wrapping_cpu_write16(abus: &mut ABus, addr: u32, value: u16) {
    cpu_write8(abus, addr, value as u8);
    cpu_write8(abus, page_wrapping_add(addr, 1), (value >> 8) as u8);
}

#[allow(dead_code)]
pub fn page_wrapping_cpu_write24(abus: &mut ABus, addr: u32, value: u32) {
    cpu_write8(abus, addr, value as u8);
    cpu_write8(abus, page_wrapping_add(addr, 1), (value >> 8) as u8);
    cpu_write8(abus, page_wrapping_add(addr, 2), (value >> 16) as u8);
}

pub fn addr_wrapping_add(addr: u32, offset: u32) -> u32 { (addr + offset) & 0x00FFFFFF }

pub fn bank_wrapping_add(addr: u32, offset: u16) -> u32 {
    (addr & 0xFF0000) | ((addr as u16).wrapping_add(offset) as u32)
}

pub fn page_wrapping_add(addr: u32, offset: u8) -> u32 {
    (addr & 0xFFFF00) | ((addr as u8).wrapping_add(offset) as u32)
}

#[allow(dead_code)]
pub fn bank_wrapping_sub(addr: u32, offset: u16) -> u32 {
    (addr & 0xFF0000) | ((addr as u16).wrapping_sub(offset) as u32)
}

#[allow(dead_code)]
pub fn page_wrapping_sub(addr: u32, offset: u8) -> u32 {
    (addr & 0xFFFF00) | ((addr as u8).wrapping_sub(offset) as u32)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_adds() {
        assert_eq!(0xAAABA9, bank_wrapping_add(0xAAAAAA, 0xFF));
        assert_eq!(0xAA2AA9, bank_wrapping_add(0xAAAAAA, 0x7FFF));
        assert_eq!(0xAAAAB9, page_wrapping_add(0xAAAAAA, 0xF));
        assert_eq!(0xAAAA29, page_wrapping_add(0xAAAAAA, 0x7F));
    }

    #[test]
    fn wrapping_subs() {
        assert_eq!(0xAAA9AB, bank_wrapping_sub(0xAAAAAA, 0xFF));
        assert_eq!(0xAA2AAB, bank_wrapping_sub(0xAAAAAA, 0x7FFF));
        assert_eq!(0xAAAA9B, page_wrapping_sub(0xAAAAAA, 0xF));
        assert_eq!(0xAAAA2B, page_wrapping_sub(0xAAAAAA, 0x7F));
    }

    #[test]
    fn cpu_wrapping_reads() {
        let mut abus = ABus::new_empty_rom();
        abus.wram[0x100FF] = 0xEF;
        abus.wram[0x10000] = 0xCD;
        abus.wram[0x10001] = 0xAB;
        assert_eq!(0xCDEF, page_wrapping_cpu_read16(&mut abus, 0x7F00FF));
        assert_eq!(0xABCDEF, page_wrapping_cpu_read24(&mut abus, 0x7F00FF));
        abus.wram[0x100FF] = 0x0;
        abus.wram[0x1FFFF] = 0xEF;
        assert_eq!(0xCDEF, bank_wrapping_cpu_read16(&mut abus, 0x7FFFFF));
        assert_eq!(0xABCDEF, bank_wrapping_cpu_read24(&mut abus, 0x7FFFFF));
    }

    #[test]
    fn cpu_wrapping_writes() {
        let mut abus = ABus::new_empty_rom();
        bank_wrapping_cpu_write16(&mut abus, 0x7FFFFF, 0xABCD);
        assert_eq!(0xCD, abus.wram[0x1FFFF]);
        assert_eq!(0xAB, abus.wram[0x10000]);
        abus.wram[0x1FFFF] = 0x0;
        abus.wram[0x10000] = 0x0;
        bank_wrapping_cpu_write24(&mut abus, 0x7FFFFF, 0xABCDEF);
        assert_eq!(0xEF, abus.wram[0x1FFFF]);
        assert_eq!(0xCD, abus.wram[0x10000]);
        assert_eq!(0xAB, abus.wram[0x10001]);
        abus.wram[0x1FFFF] = 0x0;
        abus.wram[0x10000] = 0x0;
        abus.wram[0x10001] = 0x0;
        page_wrapping_cpu_write16(&mut abus, 0x7F00FF, 0xABCD);
        assert_eq!(0xCD, abus.wram[0x100FF]);
        assert_eq!(0xAB, abus.wram[0x10000]);
        abus.wram[0x100FF] = 0x0;
        abus.wram[0x10000] = 0x0;
        page_wrapping_cpu_write24(&mut abus, 0x7F00FF, 0xABCDEF);
        assert_eq!(0xEF, abus.wram[0x100FF]);
        assert_eq!(0xCD, abus.wram[0x10000]);
        assert_eq!(0xAB, abus.wram[0x10001]);
    }

    #[test]
    fn fetch_operand() {
        let mut abus = ABus::new_empty_rom();
        abus.wram[0x1FFFD] = 0x9A;
        abus.wram[0x1FFFE] = 0x78;
        abus.wram[0x1FFFF] = 0x56;
        abus.wram[0x10000] = 0x34;
        abus.wram[0x10001] = 0x12;
        assert_eq!(0x56, fetch_operand8(&mut abus, 0x7FFFFE));
        assert_eq!(0x34, fetch_operand8(&mut abus, 0x7FFFFF));
        assert_eq!(0x5678, fetch_operand16(&mut abus, 0x7FFFFD));
        assert_eq!(0x3456, fetch_operand16(&mut abus, 0x7FFFFE));
        assert_eq!(0x56789A, fetch_operand24(&mut abus, 0x7FFFFC));
        assert_eq!(0x123456, fetch_operand24(&mut abus, 0x7FFFFE));
    }

    #[test]
    fn cpu_read_system_area() {
        let mut abus = ABus::new_empty_rom();
        // WRAM
        for i in 0..0x2000 {
            abus.wram[i] = 0xAA;
        }
        for i in 0..0x2000 {
            assert_eq!(0xAA, cpu_read8(&mut abus, i));
        }
    }
}

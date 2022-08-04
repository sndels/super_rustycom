use std::fs::File;
use std::io::Write;
use std::u32;

use log::error;
use super_rustycom_core::abus::ABus;
use super_rustycom_core::apu::smp::Spc700;
use super_rustycom_core::cpu::W65c816s;

pub struct Debugger {
    pub breakpoint: u32,
    pub steps: u32,
    pub state: DebugState,
    pub disassemble: bool,
    pub quit: bool,
}

pub enum DebugState {
    Active,
    Step,
    Run,
}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger {
            breakpoint: 0x0,
            steps: 1,
            state: DebugState::Active,
            disassemble: false,
            quit: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Debugger::new();
    }
}

#[allow(dead_code)]
// Dumps given array and returns if the operation succeeded
fn dump_memory(file_path: &str, buf: &[u8]) -> bool {
    let mut f = match File::create(file_path) {
        Ok(file) => file,
        Err(err) => {
            error!("{}", err);
            return false;
        }
    };
    if let Err(err) = f.write_all(buf) {
        error!("{}", err);
        return false;
    };
    if let Err(err) = f.sync_all() {
        error!("{}", err);
        return false;
    };
    true
}

pub fn disassemble_current(cpu: &W65c816s, abus: &ABus) -> (String, u32) {
    disassemble(cpu.current_address(), cpu, abus, true)
}

pub fn disassemble_peek(cpu: &W65c816s, abus: &ABus, offset: u32) -> (String, u32) {
    disassemble(cpu.current_address() + offset, cpu, abus, false)
}

fn disassemble(
    addr: u32,
    cpu: &W65c816s,
    abus: &ABus,
    include_effective_addr: bool,
) -> (String, u32) {
    let opcode = abus.cpu_peek8(addr);
    let opname = OPNAMES[opcode as usize];
    let opmode = ADDR_MODES[opcode as usize];
    let operand24 = abus.peek_operand24(addr);
    let operand16 = operand24 as u16;
    let operand8 = operand24 as u8;

    // DRY macros
    macro_rules! str_noperand {
        () => {{
            format!(
                "${:02X}:{:04X} {:02X}          {}",
                addr >> 16,
                addr & 0xFFFF,
                opcode,
                opname
            )
        }};
    }

    macro_rules! str_operand8 {
        ($disassembly_fmt:literal, $address:expr) => {{
            if include_effective_addr {
                format!(
                    concat!(
                        "${:02X}:{:04X} {:02X} {:02X}       ",
                        $disassembly_fmt,
                        " [${:02X}:{:04X}]"
                    ),
                    addr >> 16,
                    addr & 0xFFFF,
                    opcode,
                    operand8,
                    opname,
                    operand8,
                    $address >> 16,
                    $address & 0xFFFF,
                )
            } else {
                format!(
                    concat!("${:02X}:{:04X} {:02X} {:02X}       ", $disassembly_fmt),
                    addr >> 16,
                    addr & 0xFFFF,
                    opcode,
                    operand8,
                    opname,
                    operand8,
                )
            }
        }};
        ($disassembly_fmt:literal) => {{
            format!(
                concat!("${:02X}:{:04X} {:02X} {:02X}       ", $disassembly_fmt),
                addr >> 16,
                addr & 0xFFFF,
                opcode,
                operand8,
                opname,
                operand8,
            )
        }};
    }

    macro_rules! str_operand16 {
        ($disassembly_fmt:literal, $address:expr) => {{
            if include_effective_addr {
                format!(
                    concat!(
                        "${:02X}:{:04X} {:02X} {:02X} {:02X}    ",
                        $disassembly_fmt,
                        " [${:02X}:{:04X}]"
                    ),
                    addr >> 16,
                    addr & 0xFFFF,
                    opcode,
                    operand16 & 0xFF,
                    operand16 >> 8,
                    opname,
                    operand16,
                    $address >> 16,
                    $address & 0xFFFF,
                )
            } else {
                format!(
                    concat!("${:02X}:{:04X} {:02X} {:02X} {:02X}    ", $disassembly_fmt),
                    addr >> 16,
                    addr & 0xFFFF,
                    opcode,
                    operand16 & 0xFF,
                    operand16 >> 8,
                    opname,
                    operand16,
                )
            }
        }};
        ($disassembly_fmt:literal) => {{
            format!(
                concat!("${:02X}:{:04X} {:02X} {:02X} {:02X}    ", $disassembly_fmt),
                addr >> 16,
                addr & 0xFFFF,
                opcode,
                operand16 & 0xFF,
                operand16 >> 8,
                opname,
                operand16,
            )
        }};
    }

    macro_rules! str_operand24 {
        ($disassembly_fmt:literal, $address:expr) => {{
            if include_effective_addr {
                format!(
                    concat!(
                        "${:02X}:{:04X} {:02X} {:02X} {:02X} {:02X} ",
                        $disassembly_fmt,
                        " [${:02X}:{:04X}]"
                    ),
                    addr >> 16,
                    addr & 0xFFFF,
                    opcode,
                    operand24 & 0xFF,
                    (operand24 >> 8) & 0xFF,
                    operand24 >> 16,
                    opname,
                    operand24,
                    $address >> 16,
                    $address & 0xFFFF,
                )
            } else {
                format!(
                    concat!(
                        "${:02X}:{:04X} {:02X} {:02X} {:02X} {:02X} ",
                        $disassembly_fmt
                    ),
                    addr >> 16,
                    addr & 0xFFFF,
                    opcode,
                    operand24 & 0xFF,
                    (operand24 >> 8) & 0xFF,
                    operand24 >> 16,
                    opname,
                    operand24,
                )
            }
        }};
        ($disassembly_fmt:literal) => {{
            format!(
                concat!(
                    "${:02X}:{:04X} {:02X} {:02X} {:02X} {:02X} ",
                    $disassembly_fmt
                ),
                addr >> 16,
                addr & 0xFFFF,
                opcode,
                operand24 & 0xFF,
                (operand24 >> 8) & 0xFF,
                operand24 >> 16,
                opname,
                operand24,
            )
        }};
    }

    match opmode {
        AddrMode::Abs => (
            str_operand16!("{} ${:04X}    ", cpu.peek_abs(addr, abus).0),
            3,
        ),
        AddrMode::AbsX => (
            str_operand16!("{} ${:04X},X  ", cpu.peek_abs_x(addr, abus).0),
            3,
        ),
        AddrMode::AbsY => (
            str_operand16!("{} ${:04X},Y  ", cpu.peek_abs_y(addr, abus).0),
            3,
        ),
        AddrMode::AbsPtr16 => (
            str_operand16!("{} (${:04X}  )", cpu.peek_abs_ptr16(addr, abus).0),
            3,
        ),
        AddrMode::AbsPtr24 => (
            str_operand16!("{} [${:04X}]  ", cpu.peek_abs_ptr24(addr, abus).0),
            3,
        ),
        AddrMode::AbsXPtr16 => (
            str_operand16!("{} (${:04X},X)", cpu.peek_abs_x_ptr16(addr, abus).0),
            3,
        ),
        AddrMode::Acc => (str_noperand!(), 1),
        AddrMode::Dir => (
            str_operand16!("{} ${:02X}      ", cpu.peek_dir(addr, abus).0),
            3,
        ),
        AddrMode::DirX => (
            str_operand16!("{} ${:02X},X    ", cpu.peek_dir_x(addr, abus).0),
            3,
        ),
        AddrMode::DirY => (
            str_operand16!("{} ${:02X},Y    ", cpu.peek_dir_y(addr, abus).0),
            3,
        ),
        AddrMode::DirPtr16 => (
            str_operand16!("{} (${:02X})    ", cpu.peek_dir_ptr16(addr, abus).0),
            3,
        ),
        AddrMode::DirPtr24 => (
            str_operand16!("{} [${:02X}]    ", cpu.peek_dir_ptr24(addr, abus).0),
            3,
        ),
        AddrMode::DirXPtr16 => (
            str_operand16!("{} (${:02X},X)  ", cpu.peek_dir_x_ptr16(addr, abus).0),
            3,
        ),
        AddrMode::DirPtr16Y => (
            str_operand16!("{} (${:02X}),Y  ", cpu.peek_dir_ptr16_y(addr, abus).0),
            3,
        ),
        AddrMode::DirPtr24Y => (
            str_operand16!("{} [${:02X}],Y  ", cpu.peek_dir_ptr24_y(addr, abus).0),
            3,
        ),
        AddrMode::Imm8 => (str_operand8!("{} #${:02X}     "), 2),
        AddrMode::Imm16 => (str_operand16!("{} #${:04X}   "), 3),
        AddrMode::ImmM => {
            if cpu.p_m() {
                (str_operand8!("{} #${:02X}     "), 2)
            } else {
                (str_operand16!("{} #${:04X}   "), 3)
            }
        }
        AddrMode::ImmX => {
            if cpu.p_x() {
                (str_operand8!("{} #${:02X}     "), 2)
            } else {
                (str_operand16!("{} #${:04X}   "), 3)
            }
        }
        AddrMode::Imp => (str_noperand!(), 1),
        AddrMode::Long => (str_operand24!("{} ${:06X}  "), 4),
        AddrMode::LongX => (
            str_operand24!("{} ${:06X}  ", cpu.peek_long_x(addr, abus).0),
            4,
        ),
        AddrMode::Rel8 => {
            let address = cpu.peek_rel8(addr, abus).0 & 0xFFFF;
            let disassembly = if include_effective_addr {
                format!(
                    "${:02X}:{:04X} {:02X} {:02X}       {} ${:02X}       [${:04X}]",
                    addr >> 16,
                    addr & 0xFFFF,
                    opcode,
                    operand8,
                    opname,
                    operand8,
                    address & 0xFFFF,
                )
            } else {
                format!(
                    "${:02X}:{:04X} {:02X} {:02X}       {} ${:02X}",
                    addr >> 16,
                    addr & 0xFFFF,
                    opcode,
                    operand8,
                    opname,
                    operand8,
                )
            };
            (disassembly, 2)
        }
        AddrMode::Rel16 => {
            let address = cpu.peek_rel16(addr, abus).0 & 0xFFFF;
            let disassembly = if include_effective_addr {
                format!(
                    "${:02X}:{:04X} {:02X} {:02X} {:02X}    {} ${:04X}     [${:02X}:{:04X}]",
                    addr >> 16,
                    addr & 0xFFFF,
                    opcode,
                    operand16 & 0xFF,
                    operand16 >> 8,
                    opname,
                    operand16,
                    address >> 16,
                    address & 0xFFFF,
                )
            } else {
                format!(
                    "${:02X}:{:04X} {:02X} {:02X} {:02X}    {} ${:04X}",
                    addr >> 16,
                    addr & 0xFFFF,
                    opcode,
                    operand16 & 0xFF,
                    operand16 >> 8,
                    opname,
                    operand16,
                )
            };
            (disassembly, 3)
        }
        AddrMode::SrcDst => {
            let disassembly = format!(
                "${:02X}:{:04X} {:02X} {:02X} {:02X}    {} #${:02X},#${:02X}",
                addr >> 16,
                addr & 0xFFFF,
                opcode,
                operand16 & 0xFF,
                operand16 >> 8,
                opname,
                operand16 >> 8,
                operand16 & 0xFF
            );
            (disassembly, 3)
        }
        AddrMode::Stack => (
            str_operand8!("{} ${:02X},S    ", cpu.peek_stack(addr, abus).0),
            2,
        ),
        AddrMode::StackPtr16Y => (
            str_operand8!("{} (${:02X},S),Y", cpu.peek_stack_ptr16_y(addr, abus).0),
            2,
        ),
    }
}

fn cpu_status_reg_str(cpu: &W65c816s) -> String {
    let mut status = String::new();
    status.push(if cpu.e() { 'E' } else { 'e' });
    status.push(if cpu.p_n() { 'N' } else { 'n' });
    status.push(if cpu.p_v() { 'V' } else { 'v' });
    status.push(if cpu.p_m() { 'M' } else { 'm' });
    status.push(if cpu.p_x() { 'X' } else { 'x' });
    status.push(if cpu.p_d() { 'D' } else { 'd' });
    status.push(if cpu.p_i() { 'I' } else { 'i' });
    status.push(if cpu.p_z() { 'Z' } else { 'z' });
    status.push(if cpu.p_c() { 'C' } else { 'c' });
    status
}

pub fn cpu_status_str(cpu: &W65c816s) -> [String; 12] {
    [
        format!("A: ${:04X}", cpu.a()),
        format!("X: ${:04X}", cpu.x()),
        format!("Y: ${:04X}", cpu.y()),
        format!("PB:${:02X}", cpu.pb()),
        format!("PC:${:04X}", cpu.pc()),
        format!("DB:${:02X}", cpu.db()),
        format!("D: ${:04X}", cpu.d()),
        format!("S: ${:04X}", cpu.s()),
        format!("P: {}", cpu_status_reg_str(cpu)),
        format!("E: {}", cpu.e()),
        format!("Stopped:{}", cpu.stopped()),
        format!("Waiting:{}", cpu.waiting()),
    ]
}

fn smp_status_reg_str(smp: &Spc700) -> String {
    let mut status = String::new();
    status.push(if smp.psw().n() { 'N' } else { 'n' });
    status.push(if smp.psw().v() { 'V' } else { 'v' });
    status.push(if smp.psw().p() { 'P' } else { 'p' });
    status.push(if smp.psw().b() { 'B' } else { 'b' });
    status.push(if smp.psw().h() { 'H' } else { 'h' });
    status.push(if smp.psw().i() { 'I' } else { 'i' });
    status.push(if smp.psw().z() { 'Z' } else { 'z' });
    status.push(if smp.psw().c() { 'C' } else { 'c' });
    status
}

pub fn smp_status_str(smp: &Spc700) -> [String; 7] {
    [
        format!("A:  ${:02X}", smp.a()),
        format!("X:  ${:02X}", smp.x()),
        format!("Y:  ${:02X}", smp.y()),
        format!("SP: ${:02X}", smp.sp()),
        format!("PSW:{}", smp_status_reg_str(smp)),
        format!("YA: ${:04X}", smp.ya()),
        format!("PC: ${:04X}", smp.pc()),
    ]
}

#[derive(Copy, Clone)]
enum AddrMode {
    Abs,
    AbsX,
    AbsY,
    AbsPtr16,
    AbsPtr24,
    AbsXPtr16,
    Acc,
    Dir,
    DirX,
    DirY,
    DirPtr16,
    DirPtr24,
    DirXPtr16,
    DirPtr16Y,
    DirPtr24Y,
    Imm8,
    Imm16,
    ImmM,
    ImmX,
    Imp,
    Long,
    LongX,
    Rel8,
    Rel16,
    SrcDst,
    Stack,
    StackPtr16Y,
}

#[rustfmt::skip]
const OPNAMES: [&str; 256] = [
    "BRK", "ORA", "COP", "ORA", "TSB", "ORA", "ASL", "ORA", "PHP", "ORA", "ASL",
    "PHD", "TSB", "ORA", "ASL", "ORA", "BPL", "ORA", "ORA", "ORA", "TRB", "ORA",
    "ASL", "ORA", "CLC", "ORA", "INC", "TCS", "TRB", "ORA", "ASL", "ORA", "JSR",
    "AND", "JSL", "AND", "BIT", "AND", "ROL", "AND", "PLP", "AND", "ROL", "PLD",
    "BIT", "AND", "ROL", "AND", "BMI", "AND", "AND", "AND", "BIT", "AND", "ROL",
    "AND", "SEC", "AND", "DEC", "TSC", "BIT", "AND", "ROL", "AND", "RTI", "EOR",
    "WDM", "EOR", "MVP", "EOR", "LSR", "EOR", "PHA", "EOR", "LSR", "PHK", "JMP",
    "EOR", "LSR", "EOR", "BVC", "EOR", "EOR", "EOR", "MVN", "EOR", "LSR", "EOR",
    "CLI", "EOR", "PHY", "TCD", "JMP", "EOR", "LSR", "EOR", "RTS", "ADC", "PER",
    "ADC", "STZ", "ADC", "ROR", "ADC", "PLA", "ADC", "ROR", "RTL", "JMP", "ADC",
    "ROR", "ADC", "BVS", "ADC", "ADC", "ADC", "STZ", "ADC", "ROR", "ADC", "SEI",
    "ADC", "PLY", "TDC", "ADC", "JMP", "ROR", "ADC", "BRA", "STA", "BRL", "STA",
    "STY", "STA", "STX", "STA", "DEY", "BIT", "TXA", "PHB", "STY", "STA", "STX",
    "STA", "BCC", "STA", "STA", "STA", "STY", "STA", "STX", "STA", "TYA", "STA",
    "TXS", "TXY", "STZ", "STA", "STZ", "STA", "LDY", "LDA", "LDX", "LDA", "LDY",
    "LDA", "LDX", "LDA", "TAY", "LDA", "TAX", "PLB", "LDY", "LDA", "LDX", "LDA",
    "BCS", "LDA", "LDA", "LDA", "LDY", "LDA", "LDX", "LDA", "CLV", "LDA", "TSX",
    "TYX", "LDY", "LDA", "LDX", "LDA", "CPY", "CMP", "REP", "CMP", "CPY", "CMP",
    "DEC", "CMP", "INY", "CMP", "DEX", "WAI", "CPY", "CMP", "DEC", "CMP", "BNE",
    "CMP", "CMP", "CMP", "PEI", "CMP", "DEC", "CMP", "CLD", "CMP", "PHX", "STP",
    "JMP", "CMP", "DEC", "CMP", "CPX", "SBC", "SEP", "SBC", "CPX", "SBC", "INC",
    "SBC", "INX", "SBC", "NOP", "XBA", "CPX", "SBC", "INC", "SBC", "BEQ", "SBC",
    "SBC", "SBC", "PEA", "SBC", "INC", "SBC", "SED", "SBC", "PLX", "XCE", "JSR",
    "SBC", "INC", "SBC"
];

#[rustfmt::skip]
const ADDR_MODES: [AddrMode; 256] = [
    AddrMode::Imp, AddrMode::DirXPtr16, AddrMode::Imm8, AddrMode::Stack, AddrMode::Dir,
    AddrMode::Dir, AddrMode::Dir, AddrMode::DirPtr24, AddrMode::Imp, AddrMode::ImmM,
    AddrMode::Acc, AddrMode::Imp, AddrMode::Abs, AddrMode::Abs, AddrMode::Abs,
    AddrMode::Long, AddrMode::Rel8, AddrMode::DirPtr16Y, AddrMode::DirPtr16,
    AddrMode::StackPtr16Y, AddrMode::Dir, AddrMode::DirX, AddrMode::DirX,
    AddrMode::DirPtr24Y, AddrMode::Imp, AddrMode::AbsY, AddrMode::Acc, AddrMode::Imp,
    AddrMode::Abs, AddrMode::AbsX, AddrMode::AbsX, AddrMode::LongX, AddrMode::Abs,
    AddrMode::DirXPtr16, AddrMode::Long, AddrMode::Stack, AddrMode::Dir, AddrMode::Dir,
    AddrMode::Dir, AddrMode::DirPtr24, AddrMode::Imp, AddrMode::ImmM, AddrMode::Acc,
    AddrMode::Imp, AddrMode::Abs, AddrMode::Abs, AddrMode::Abs, AddrMode::Long,
    AddrMode::Rel8, AddrMode::DirPtr16Y, AddrMode::DirPtr16, AddrMode::StackPtr16Y,
    AddrMode::DirX, AddrMode::DirX, AddrMode::DirX, AddrMode::DirPtr24Y, AddrMode::Imp,
    AddrMode::AbsY, AddrMode::Acc, AddrMode::Imp, AddrMode::AbsX, AddrMode::AbsX,
    AddrMode::AbsX, AddrMode::LongX, AddrMode::Imp, AddrMode::DirXPtr16, AddrMode::Imm8,
    AddrMode::Stack, AddrMode::SrcDst, AddrMode::Dir, AddrMode::Dir, AddrMode::DirPtr24,
    AddrMode::Imp, AddrMode::ImmM, AddrMode::Acc, AddrMode::Imp, AddrMode::Abs,
    AddrMode::Abs, AddrMode::Abs, AddrMode::Long, AddrMode::Rel8, AddrMode::DirPtr16Y,
    AddrMode::DirPtr16, AddrMode::StackPtr16Y, AddrMode::SrcDst, AddrMode::DirX,
    AddrMode::DirX, AddrMode::DirPtr24Y, AddrMode::Imp, AddrMode::AbsY, AddrMode::Imp,
    AddrMode::Imp, AddrMode::Long, AddrMode::AbsX, AddrMode::AbsX, AddrMode::LongX,
    AddrMode::Imp, AddrMode::DirXPtr16, AddrMode::Imm16, AddrMode::Stack, AddrMode::Dir,
    AddrMode::Dir, AddrMode::Dir, AddrMode::DirPtr24, AddrMode::Imp, AddrMode::ImmM,
    AddrMode::Acc, AddrMode::Imp, AddrMode::AbsPtr16, AddrMode::Abs, AddrMode::Abs,
    AddrMode::Long, AddrMode::Rel8, AddrMode::DirPtr16Y, AddrMode::DirPtr16,
    AddrMode::StackPtr16Y, AddrMode::DirX, AddrMode::DirX, AddrMode::DirX,
    AddrMode::DirPtr24Y, AddrMode::Imp, AddrMode::AbsY, AddrMode::Imp, AddrMode::Imp,
    AddrMode::AbsXPtr16, AddrMode::AbsX, AddrMode::AbsX, AddrMode::LongX, AddrMode::Rel8,
    AddrMode::DirXPtr16, AddrMode::Rel16, AddrMode::Stack, AddrMode::Dir, AddrMode::Dir,
    AddrMode::Dir, AddrMode::DirPtr24, AddrMode::Imp, AddrMode::ImmM, AddrMode::Imp,
    AddrMode::Imp, AddrMode::Abs, AddrMode::Abs, AddrMode::Abs, AddrMode::Long,
    AddrMode::Rel8, AddrMode::DirPtr16Y, AddrMode::DirPtr16, AddrMode::StackPtr16Y,
    AddrMode::DirX, AddrMode::DirX, AddrMode::DirY, AddrMode::DirPtr24Y, AddrMode::Imp,
    AddrMode::AbsY, AddrMode::Imp, AddrMode::Imp, AddrMode::Abs, AddrMode::AbsX,
    AddrMode::AbsX, AddrMode::LongX, AddrMode::ImmX, AddrMode::DirXPtr16, AddrMode::ImmX,
    AddrMode::Stack, AddrMode::Dir, AddrMode::Dir, AddrMode::Dir, AddrMode::DirPtr24,
    AddrMode::Imp, AddrMode::ImmM, AddrMode::Imp, AddrMode::Imp, AddrMode::Abs,
    AddrMode::Abs, AddrMode::Abs, AddrMode::Long, AddrMode::Rel8, AddrMode::DirPtr16Y,
    AddrMode::DirPtr16, AddrMode::StackPtr16Y, AddrMode::DirX, AddrMode::DirX,
    AddrMode::DirY, AddrMode::DirPtr16Y, AddrMode::Imp, AddrMode::AbsY, AddrMode::Imp,
    AddrMode::Imp, AddrMode::AbsX, AddrMode::AbsX, AddrMode::AbsY, AddrMode::LongX,
    AddrMode::ImmX, AddrMode::DirXPtr16, AddrMode::Imm8, AddrMode::Stack, AddrMode::Dir,
    AddrMode::Dir, AddrMode::Dir, AddrMode::DirPtr24, AddrMode::Imp, AddrMode::ImmM,
    AddrMode::Imp, AddrMode::Imp, AddrMode::Abs, AddrMode::Abs, AddrMode::Abs,
    AddrMode::Long, AddrMode::Rel8, AddrMode::DirPtr16Y, AddrMode::DirPtr16,
    AddrMode::StackPtr16Y, AddrMode::Dir, AddrMode::DirX, AddrMode::DirX,
    AddrMode::DirPtr24Y, AddrMode::Imp, AddrMode::AbsY, AddrMode::Imp, AddrMode::Imp,
    AddrMode::AbsPtr24, AddrMode::AbsX, AddrMode::AbsX, AddrMode::LongX, AddrMode::ImmX,
    AddrMode::DirXPtr16, AddrMode::Imm8, AddrMode::Stack, AddrMode::Dir, AddrMode::Dir,
    AddrMode::Dir, AddrMode::DirPtr24, AddrMode::Imp, AddrMode::ImmM, AddrMode::Imp,
    AddrMode::Imp, AddrMode::Abs, AddrMode::Abs, AddrMode::Abs, AddrMode::Long,
    AddrMode::Rel8, AddrMode::DirPtr16Y, AddrMode::DirPtr16, AddrMode::StackPtr16Y,
    AddrMode::Imm16, AddrMode::DirX, AddrMode::DirX, AddrMode::DirPtr24Y, AddrMode::Imp,
    AddrMode::AbsY, AddrMode::Imp, AddrMode::Imp, AddrMode::AbsXPtr16, AddrMode::AbsX,
    AddrMode::AbsX, AddrMode::LongX
];

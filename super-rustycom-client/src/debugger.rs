use std::fs::File;
use std::io::{self, Write};
use std::u32;

use super_rustycom_core::abus::ABus;
use super_rustycom_core::cpu::W65C816S;

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
    Quit,
}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger {
            breakpoint: 0x0,
            steps: 0,
            state: DebugState::Active,
            disassemble: true,
            quit: false,
        }
    }

    pub fn take_command(&mut self, cpu: &mut W65C816S, abus: &mut ABus) {
        println!("PRG at");
        disassemble_current(cpu, abus);
        print!("(debug) ");
        io::stdout().flush().unwrap();
        let mut command_str = String::new();
        io::stdin()
            .read_line(&mut command_str)
            .expect("Read failed");
        let arg_vec = command_str.trim().split_whitespace().collect::<Vec<&str>>();
        if arg_vec.len() > 0 {
            match arg_vec[0] {
                "disassemble" | "da" => {
                    self.disassemble = !self.disassemble;
                    if self.disassemble {
                        println!("Toggled continious disassembly on");
                    } else {
                        println!("Toggled continious disassembly off");
                    }
                }
                "dump" => {
                    if arg_vec.len() > 1 {
                        match arg_vec[1] {
                            "rom" => unimplemented!(),
                            "wram" => match dump_memory("wramdump.bin", &abus.wram()) {
                                Ok(_) => println!("WRAM dumped"),
                                Err(err) => println!("Dumping WRAM failed!\n{}", err),
                            },
                            "vram" => match dump_memory("vramdump.bin", &abus.vram()) {
                                Ok(_) => println!("VRAM dumped"),
                                Err(err) => println!("Dumping VRAM failed!\n{}", err),
                            },
                            "oam" => match dump_memory("oamdump.bin", &abus.oam()) {
                                Ok(_) => println!("OAM dumped"),
                                Err(err) => println!("Dumping OAM failed!\n{}", err),
                            },
                            "cgram" => match dump_memory("cgramdump.bin", &abus.cgram()) {
                                Ok(_) => println!("CGRAM dumped"),
                                Err(err) => println!("Dumping CGRAM failed!\n{}", err),
                            },
                            &_ => println!("Invalid parameter \"{}\"", arg_vec[1]),
                        }
                    } else {
                        match dump_memory("wramdump.bin", &abus.wram()) {
                            Ok(_) => println!("WRAM dumped"),
                            Err(err) => println!("Dumping WRAM failed!\n{}", err),
                        };
                        match dump_memory("vramdump.bin", &abus.vram()) {
                            Ok(_) => println!("VRAM dumped"),
                            Err(err) => println!("Dumping VRAM failed!\n{}", err),
                        };
                        match dump_memory("oamdump.bin", &abus.oam()) {
                            Ok(_) => println!("OAM dumped"),
                            Err(err) => println!("Dumping OAM failed!\n{}", err),
                        };
                        match dump_memory("cgramdump.bin", &abus.cgram()) {
                            Ok(_) => println!("CGRAM dumped"),
                            Err(err) => println!("Dumping CGRAM failed!\n{}", err),
                        };
                    }
                }
                "peek" => {
                    if arg_vec.len() == 2 {
                        let ret = u32::from_str_radix(arg_vec[1], 16);
                        match ret {
                            Ok(addr) => println!("${:06X}", abus.addr_wrapping_cpu_read24(addr)),
                            Err(_e) => println!("Invalid addr \"{}\"", arg_vec[1]),
                        }
                    }
                }
                "step" | "s" => {
                    self.state = DebugState::Step;
                    if arg_vec.len() == 1 {
                        self.steps = 1;
                    } else {
                        let steps = arg_vec[1].parse::<u32>();
                        match steps {
                            Ok(s) => self.steps = s,
                            Err(e) => println!("Error parsing step: {}", e),
                        }
                    }
                }
                "breakpoint" | "bp" => {
                    if arg_vec.len() > 1 {
                        let breakpoint = u32::from_str_radix(arg_vec[1], 16);
                        match breakpoint {
                            Ok(b) => self.breakpoint = b,
                            Err(e) => println!("Error parsing step: {}", e),
                        }
                    }
                }
                "cpu" => {
                    println!("A:       ${:04X}", cpu.a());
                    println!("X:       ${:04X}", cpu.x());
                    println!("Y:       ${:04X}", cpu.y());
                    println!("PB:      ${:02X}", cpu.pb());
                    println!("PC:      ${:04X}", cpu.pc());
                    println!("DB:      ${:02X}", cpu.db());
                    println!("D:       ${:04X}", cpu.d());
                    println!("S:       ${:04X}", cpu.s());
                    println!("P:       {}", status_str(cpu));
                    println!("E:       {}", cpu.e());
                    println!("Stopped: {}", cpu.stopped());
                    println!("Waiting: {}", cpu.waiting());
                }
                "run" | "r" => self.state = DebugState::Run,
                "reset" => cpu.reset(abus),
                "exit" => self.state = DebugState::Quit,
                "help" | "h" => print_help(),
                _ => println!("Unknown command \"{}\"", command_str.trim()),
            }
        } else {
            print_help();
        }
    }
}

fn print_help() {
    println!("disassemble, da        -- toggle on the fly disassembly");
    println!("dump ([param])         -- dump wram, vram, oam, cgram to files");
    println!("                          specify one to dump only it");
    println!("peek [addr]            -- peek 24bits from addr");
    println!("step, s ([param])      -- step to next instruction or by amount");
    println!("breakpoint, bp [param] -- set execution breakpoint at hex address");
    println!("cpu                    -- print out cpu state");
    println!("run, r                 -- run until breakpoint is hit");
    println!("reset                  -- reset cpu");
    println!("exit                   -- stop emulation");
}

fn dump_memory(file_path: &str, buf: &[u8]) -> Result<(), String> {
    let mut f = match File::create(file_path) {
        Ok(file) => file,
        Err(err) => return Err(format!("Creating file failed:\n{}", err)),
    };
    if let Err(err) = f.write_all(buf) {
        return Err(format!("Write failed:\n{}", err));
    };
    if let Err(err) = f.sync_all() {
        return Err(format!("Sync failed:\n{}", err));
    };
    Ok(())
}

pub fn disassemble_current(cpu: &W65C816S, abus: &mut ABus) {
    disassemble(cpu.current_address(), cpu, abus);
}

fn disassemble(addr: u32, cpu: &W65C816S, abus: &mut ABus) {
    let opcode = abus.cpu_read8(addr);
    let opname = OPNAMES[opcode as usize];
    let opmode = ADDR_MODES[opcode as usize];
    let operand24 = abus.fetch_operand24(addr);
    let operand16 = operand24 as u16;
    let operand8 = operand24 as u8;

    // DRY macros
    macro_rules! str_operand8 {
        () => {{
            format!("{0:02X}", operand8)
        }};
    }

    macro_rules! str_operand16 {
        () => {{
            format!("{0:02X} {1:02X}", operand16 & 0xFF, operand16 >> 8)
        }};
    }

    macro_rules! str_operand24 {
        () => {{
            format!(
                "{0:02X} {1:02X} {2:02X}",
                operand24 & 0xFF,
                (operand24 >> 8) & 0xFF,
                operand24 >> 16
            )
        }};
    }

    macro_rules! str_full_addr {
        ($address:expr) => {{
            format!("[${0:02X}:{1:04X}]", $address >> 16, $address & 0xFFFF)
        }};
    }

    print!(
        "${0:02X}:{1:04X} {2:02X}",
        addr >> 16,
        addr & 0xFFFF,
        opcode
    );
    let unique_strs: (String, String, String) = match opmode {
        AddrMode::Abs => (
            str_operand16!(),
            format!("{0} ${1:04X}", opname, operand16),
            str_full_addr!(cpu.abs(addr, abus).0),
        ),
        AddrMode::AbsX => (
            str_operand16!(),
            format!("{0} ${1:04X},X", opname, operand16),
            str_full_addr!(cpu.abs_x(addr, abus).0),
        ),
        AddrMode::AbsY => (
            str_operand16!(),
            format!("{0} ${1:04X},Y", opname, operand16),
            str_full_addr!(cpu.abs_y(addr, abus).0),
        ),
        AddrMode::AbsPtr16 => (
            str_operand16!(),
            format!("{0} (${1:04X})", opname, operand16),
            str_full_addr!(cpu.abs_ptr16(addr, abus).0),
        ),
        AddrMode::AbsPtr24 => (
            str_operand16!(),
            format!("{0} [${1:04X}]", opname, operand16),
            str_full_addr!(cpu.abs_ptr24(addr, abus).0),
        ),
        AddrMode::AbsXPtr16 => (
            str_operand16!(),
            format!("{0} (${1:04X},X)", opname, operand16),
            str_full_addr!(cpu.abs_x_ptr16(addr, abus).0),
        ),
        AddrMode::Acc => (String::new(), String::from(opname), String::new()),
        AddrMode::Dir => (
            str_operand16!(),
            format!("{0} ${1:02X}", opname, operand16),
            str_full_addr!(cpu.dir(addr, abus).0),
        ),
        AddrMode::DirX => (
            str_operand16!(),
            format!("{0} ${1:02X},X", opname, operand16),
            str_full_addr!(cpu.dir_x(addr, abus).0),
        ),
        AddrMode::DirY => (
            str_operand16!(),
            format!("{0} ${1:02X},Y", opname, operand16),
            str_full_addr!(cpu.dir_y(addr, abus).0),
        ),
        AddrMode::DirPtr16 => (
            str_operand16!(),
            format!("{0} (${1:02X})", opname, operand16),
            str_full_addr!(cpu.dir_ptr16(addr, abus).0),
        ),
        AddrMode::DirPtr24 => (
            str_operand16!(),
            format!("{0} [${1:02X}]", opname, operand16),
            str_full_addr!(cpu.dir_ptr24(addr, abus).0),
        ),
        AddrMode::DirXPtr16 => (
            str_operand16!(),
            format!("{0} (${1:02X},X)", opname, operand16),
            str_full_addr!(cpu.dir_x_ptr16(addr, abus).0),
        ),
        AddrMode::DirPtr16Y => (
            str_operand16!(),
            format!("{0} (${1:02X}),Y", opname, operand16),
            str_full_addr!(cpu.dir_ptr16_y(addr, abus).0),
        ),
        AddrMode::DirPtr24Y => (
            str_operand16!(),
            format!("{0} [${1:02X}],Y", opname, operand16),
            str_full_addr!(cpu.dir_ptr24_y(addr, abus).0),
        ),
        AddrMode::Imm8 => (
            str_operand8!(),
            format!("{0} #${1:02X}", opname, operand8),
            String::new(),
        ),
        AddrMode::Imm16 => (
            str_operand16!(),
            format!("{0} #${1:04X}", opname, operand16),
            String::new(),
        ),
        AddrMode::ImmM => {
            if cpu.p_m() {
                (
                    str_operand8!(),
                    format!("{0} #${1:02X}", opname, operand8),
                    String::new(),
                )
            } else {
                (
                    str_operand16!(),
                    format!("{0} #${1:04X}", opname, operand16),
                    String::new(),
                )
            }
        }
        AddrMode::ImmX => {
            if cpu.p_x() {
                (
                    str_operand8!(),
                    format!("{0} #${1:02X}", opname, operand8),
                    String::new(),
                )
            } else {
                (
                    str_operand16!(),
                    format!("{0} #${1:04X}", opname, operand16),
                    String::new(),
                )
            }
        }
        AddrMode::Imp => (String::new(), String::from(opname), String::new()),
        AddrMode::Long => (
            str_operand24!(),
            format!("{0} ${1:06X}", opname, operand24),
            String::new(),
        ),
        AddrMode::LongX => (
            str_operand24!(),
            format!("{0} ${1:06X}", opname, operand24),
            str_full_addr!(cpu.long_x(addr, abus).0),
        ),
        AddrMode::Rel8 => (
            str_operand8!(),
            format!("{0} ${1:02X}", opname, operand8),
            format!("[${0:04X}]", cpu.rel8(addr, abus).0 & 0xFFFF),
        ),
        AddrMode::Rel16 => (
            str_operand16!(),
            format!("{0} ${1:04X}", opname, operand16),
            format!("[${0:04X}]", cpu.rel16(addr, abus).0 & 0xFFFF),
        ),
        AddrMode::SrcDst => (
            str_operand16!(),
            format!(
                "{0} #${1:02X},#${2:02X}",
                opname,
                operand16 >> 8,
                operand16 & 0xFF
            ),
            String::new(),
        ),
        AddrMode::Stack => (
            str_operand8!(),
            format!("{0} ${1:02X},S", opname, operand8),
            str_full_addr!(cpu.stack(addr, abus).0),
        ),
        AddrMode::StackPtr16Y => (
            str_operand8!(),
            format!("{0} (${1:02X},S),Y", opname, operand8),
            str_full_addr!(cpu.stack_ptr16_y(addr, abus).0),
        ),
    };
    print!(
        " {0:<8} {1:<13} {2:<10}",
        unique_strs.0, unique_strs.1, unique_strs.2
    );
    println!(
        " A:{0:04X} X:{1:04X} Y:{2:04X} {3}",
        cpu.a(),
        cpu.x(),
        cpu.y(),
        status_str(cpu)
    );
}

fn status_str(cpu: &W65C816S) -> String {
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

#[cfg_attr(rustfmt, rustfmt_skip)]
const OPNAMES: [&'static str; 256] = [
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

#[cfg_attr(rustfmt, rustfmt_skip)]
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

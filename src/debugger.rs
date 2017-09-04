use std::io::{self, Write};
use std::u32;
use cpu::Cpu;
use abus::ABus;
use op;

pub struct Debugger {
    pub breakpoint:  u32,
    pub active: bool,
}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger{
            breakpoint: 0x0,
            active: true,
        }
    }

    pub fn take_command(&mut self, cpu: &mut Cpu, abus: &mut ABus) {
        print!("Cursor at: ");
        disassemble(cpu.current_address(), cpu, abus);
        print!("(debug) ");
        io::stdout().flush().unwrap();
        let mut command_str = String::new();
        io::stdin().read_line(&mut command_str).expect("Read failed");
        let arg_vec = command_str.trim().split_whitespace().collect::<Vec<&str>>();
        if arg_vec.len() > 0 {
            match arg_vec[0] {
                "step" | "s" => {
                    if arg_vec.len() == 1 {
                        cpu.step(abus);
                    } else {
                        let steps = arg_vec[1].parse::<usize>();
                        match steps {
                            Ok(s) => for _ in 0..s { cpu.step(abus); },
                            Err(e) => println!("Error parsing step: {}", e)
                        }
                    }
                }
                "breakpoint" | "bp" => {
                    if arg_vec.len() > 1 {
                        let breakpoint = u32::from_str_radix(arg_vec[1], 16);
                        match breakpoint {
                            Ok(b) => self.breakpoint = b,
                            Err(e) => println!("Error parsing step: {}", e)
                        }
                    }
                }
                "cpu" => {
                    println!("A:  ${:04X}", cpu.a());
                    println!("X:  ${:04X}", cpu.x());
                    println!("Y:  ${:04X}", cpu.y());
                    println!("PB: ${:02X}", cpu.pb());
                    println!("PC: ${:04X}", cpu.pc());
                    println!("DB: ${:02X}", cpu.db());
                    println!("D:  ${:04X}", cpu.d());
                    println!("S:  ${:04X}", cpu.s());
                    println!("P:  {}",      status_str(cpu));
                }
                "run" | "r" => self.active = false,
                // TODO: quit-comand
                _            => println!("Unknown command \"{}\"", command_str.trim())
            }
        }
    }
}

fn disassemble(addr: u32, cpu: &Cpu, abus: &mut ABus) {
    print_addr(addr);
    let opcode = abus.cpu_read8(addr);
    match opcode {
        op::CLC => print!("CLC"),
        op::JSR_20 => {
            print!("JSR");
            print_operand16(addr, abus);
        }
        op::RTS => print!("RTS"),
        op::SEI => print!("SEI"),
        op::STA_8D => {
            print!("STA");
            print_operand16(addr, abus);
        }
        op::STX_8E => {
            print!("STX");
            print_operand16(addr, abus);
        }
        op::TXS => print!("TXS"),
        op::STZ_9C => {
            print!("STZ");
            print_operand16(addr, abus);
        }
        op::LDX_A2 | op::LDX_AE => {
            print!("LDX");
            if cpu.p_x() {
                print_operand8(addr, abus);
            } else {
                print_operand16(addr, abus);
            }
        }
        op::LDA_A9 => {
            print!("LDA");
            if cpu.p_m() {
                print_operand8(addr, abus);
            } else {
                print_operand16(addr, abus);
            }
        }
        op::TAX => print!("TAX"),
        op::REP => {
            print!("REP");
            let operand = print_operand8(addr, abus);
            print_binary(operand);
        }
        op::BNE => {
            print!("BNE");
            print_operand8(addr, abus);
        }
        op::CPX_E0 => {
            print!("CPX");
            if cpu.p_x() {
                print_operand8(addr, abus);
            } else {
                print_operand16(addr, abus);
            }
        }
        op::SEP => {
            print!("SEP");
            let operand = print_operand8(addr, abus);
            print_binary(operand);
        }
        op::INX => print!("INX"),
        op::XCE => print!("XCE"),
        _ => panic!("Unknown opcode ${0:02X} at ${1:02X}:{2:04X}", opcode, addr >> 16, addr & 0xFFF)
    }
    println!("");
}

fn status_str(cpu: &Cpu) -> String {
    let mut status = String::new();
    status.push(if cpu.e()   { 'E' } else { 'e' });
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

fn print_addr(addr: u32) {
    print!("${0:02X}:{1:04X} ", addr >> 16, addr & 0xFFFF);
}

fn print_operand8(addr: u32, abus: &mut ABus) -> u8 {
    let operand = abus.fetch_operand8(addr);
    print!(" ${:02X}", operand);
    operand
}

fn print_operand16(addr: u32, abus: &mut ABus) -> u16 {
    let operand = abus.fetch_operand16(addr);
    print!(" ${:04X}", operand);
    operand
}

fn print_binary(value: u8) {
    print!(" [{:08b}]", value);
}

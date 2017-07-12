use std::io::{self, Write};
use std::u32;
use cpu;
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
        disassemble(cpu.get_pb_pc(), cpu, abus);
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
                "cpu" => cpu.print(),
                "run" | "r" => self.active = false,
                // TODO: quit-comand
                _            => println!("Unknown command \"{}\"", command_str.trim())
            }
        }
    }
}

fn disassemble(addr: u32, cpu: &Cpu, abus: &mut ABus) {
    let opcode = abus.read8(addr);
    match opcode {
        op::CLC => println!("{:#01$X} CLC", addr, 8),
        op::JSR_20 => {
            let sub_addr = abus.fetch_operand_16(addr);
            println!("{0:#01$X} JSR {2:#03$X}", addr, 8, sub_addr, 6);
        }
        op::SEI => println!("{:#01$X} SEI", addr, 8),
        op::STA_8D => {
            let read_addr = abus.fetch_operand_16(addr);
            println!("{0:#01$X} STA {2:#03$X}", addr, 8, read_addr, 6);
        }
        op::TXS => println!("{0:#01$X} TXS", addr, 8),
        op::STZ_9C => {
            let read_addr = abus.fetch_operand_16(addr);
            println!("{0:#01$X} STZ {2:#03$X}", addr, 8, read_addr, 6);
        }
        op::LDX_A2 => {
            if cpu.get_p_x() {
                let result = abus.fetch_operand_8(addr);
                println!("{0:#01$X} LSX {2:#03$X}", addr, 8, result, 4);
            } else {
                let result = abus.fetch_operand_16(addr);
                println!("{0:#01$X} LSX {2:#03$X}", addr, 8, result, 6);
            }
        }
        op::LDA_A9 => {
            if cpu.get_p_m() {
                let result = abus.fetch_operand_8(addr);
                println!("{0:#01$X} LDA {2:#03$X}", addr, 8, result, 4);
            } else {
                let result = abus.fetch_operand_16(addr);
                println!("{0:#01$X} LDA {2:#03$X}", addr, 8, result, 6);
            }
        }
        op::TAX => println!("{0:#01$X} TAX", addr, 8),
        op::REP => {
            let bits = abus.fetch_operand_8(addr);
            println!("{0:#01$X} REP {2:#03$X} [{2:b}]", addr, 8, bits, 4);
        }
        op::BNE => {
            let offset = abus.fetch_operand_8(addr);
            println!("{0:#01$X} BNE {2:#03$X}", addr, 8, offset, 4);
        }
        op::SEP => {
            let bits = abus.fetch_operand_8(addr);
            println!("{0:#01$X} SEP {2:#03$X} [{2:b}]", addr, 8, bits, 4);
        }
        op::INX => println!("{:#01$x} INX", addr, 8),
        op::XCE => println!("{:#01$x} XCE", addr, 8),
        _ => panic!("Unknown opcode {:X}!", opcode),
    }
}
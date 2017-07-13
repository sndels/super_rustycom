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
    let opcode = abus.read_8(addr);
    match opcode {
        op::CLC => println!("{0:#08X} CLC", addr),
        op::JSR_20 => {
            let sub_addr = abus.fetch_operand_16(addr);
            println!("{0:#08X} JSR {1:#06X}", addr, sub_addr);
        }
        op::SEI => println!("{0:#08X} SEI", addr),
        op::STA_8D => {
            let read_addr = abus.fetch_operand_16(addr);
            println!("{0:#08X} STA {1:#06X}", addr, read_addr);
        }
        op::TXS => println!("{0:#08X} TXS", addr),
        op::STZ_9C => {
            let read_addr = abus.fetch_operand_16(addr);
            println!("{0:#08X} STZ {1:#06X}", addr, read_addr);
        }
        op::LDX_A2 => {
            if cpu.get_p_x() {
                let result = abus.fetch_operand_8(addr);
                println!("{0:#08X} LSX {1:#04X}", addr, result);
            } else {
                let result = abus.fetch_operand_16(addr);
                println!("{0:#08X} LSX {1:#06X}", addr, result);
            }
        }
        op::LDA_A9 => {
            if cpu.get_p_m() {
                let result = abus.fetch_operand_8(addr);
                println!("{0:#08X} LDA {1:#04X}", addr, result);
            } else {
                let result = abus.fetch_operand_16(addr);
                println!("{0:#08X} LDA {1:#06X}", addr, result);
            }
        }
        op::TAX => println!("{0:#08X} TAX", addr),
        op::REP => {
            let bits = abus.fetch_operand_8(addr);
            println!("{0:#08X} REP {1:#04X} [{1:08b}]", addr, bits);
        }
        op::BNE => {
            let offset = abus.fetch_operand_8(addr);
            println!("{0:#08X} BNE {1:#04X}", addr, offset);
        }
        op::SEP => {
            let bits = abus.fetch_operand_8(addr);
            println!("{0:#08X} SEP {1:#04X} [{1:08b}]", addr, bits);
        }
        op::INX => println!("{0:#08X} INX", addr),
        op::XCE => println!("{0:#08X} XCE", addr),
        _ => panic!("Unknown opcode {0:02X} at {1:#08X}", opcode, addr)
    }
}

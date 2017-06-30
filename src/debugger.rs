use std::io::{self, Write};
use std::u32;
use cpu::Cpu;
use abus::ABus;

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
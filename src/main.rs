use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let rom_path = env::args().nth(1).expect("No rom defined");
    let mut rom_file = File::open(&rom_path).expect("Opening rom failed");
    let mut rom_bytes = Vec::new();
    let read_bytes = rom_file.read_to_end(&mut rom_bytes).expect("Reading rom to bytes failed");
    println!("Read {} bytes from {}", read_bytes, rom_path);
}

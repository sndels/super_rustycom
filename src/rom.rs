use std::fs::File;
use std::io::prelude::*;

use mmap;

enum RomMakeup {
    SlowLoRom = 0x20,
    // TODO: Support more types
}

enum RomChipset {
    Rom = 0x0,
    // TODO: Support more types
}

enum RomRamsize{
    Zero = 0x0,
    // TODO: Support more types
}

pub struct Rom {
    rom: Box<[u8]>,
    // TODO: Extra chips, memory mapper
}

impl Rom {
    pub fn new(rom_path: &String) -> Rom {
        // Load rom from file
        let mut rom_file = File::open(&rom_path).expect("Opening rom failed");
        let mut rom_bytes = Vec::new();
        let read_bytes = rom_file.read_to_end(&mut rom_bytes).expect("Reading rom to bytes failed");
        println!("Read {} bytes from {}", read_bytes, rom_path);

        // Check that the rom-type is supported
        assert!((rom_bytes[0x7FD5] | 0b0010_0000) == RomMakeup::SlowLoRom as u8); // Homebrews might have this as 0x00
        assert!(rom_bytes[0x7FD6] == RomChipset::Rom as u8);
        assert!(rom_bytes[0x7FD8] == RomRamsize::Zero as u8);

        Rom {
            rom: rom_bytes.into_boxed_slice(),
        }
    }

    #[cfg(test)]
    pub fn new_empty() -> Rom {
        Rom {
            rom: vec![0; 4194304].into_boxed_slice(),
        }
    }

    pub fn read_ws1_lo_rom_8(&self, bank: usize, bank_addr: usize) -> u8 {
        let offset = (bank_addr & 0xFFFF) - mmap::LOROM_FIRST;
        self.rom[bank * mmap::LOROM_FIRST + offset]
    }

    pub fn read_ws1_hi_rom_8(&self, bank: usize, bank_addr: usize) -> u8 {
        unimplemented!()
    }

    pub fn read_ws2_lo_rom_8(&self, bank: usize, bank_addr: usize) -> u8 {
        unimplemented!()
    }

    pub fn read_ws2_hi_rom_8(&self, bank: usize, bank_addr: usize) -> u8 {
        self.rom[((bank - mmap::WS2_HIROM_FIRST_BANK) << 16) | bank_addr]
    }

    #[cfg(not(test))]
    pub fn write_ws1_lo_rom_8(&mut self, value: u8, bank: usize, bank_addr: usize) {
        panic!("Write to rom at addr ${0:02X}:{1:04X}!", bank, bank_addr);
    }

    #[cfg(test)]
    pub fn write_ws1_lo_rom_8(&mut self, value: u8, bank: usize, bank_addr: usize) {
        let offset = (bank_addr & 0xFFFF) - mmap::LOROM_FIRST;
        self.rom[bank * mmap::LOROM_FIRST + offset] = value;
    }

    pub fn write_ws1_hi_rom_8(&mut self, value: u8, bank: usize, bank_addr: usize) {
        panic!("Write to rom at addr ${0:02X}:{1:04X}!", bank, bank_addr);
    }

    pub fn write_ws2_lo_rom_8(&mut self, value: u8, bank: usize, bank_addr: usize) {
        panic!("Write to rom at addr ${0:02X}:{1:04X}!", bank, bank_addr);
    }

    #[cfg(not(test))]
    pub fn write_ws2_hi_rom_8(&mut self, value: u8, bank: usize, bank_addr: usize) {
        panic!("Write to rom at addr ${0:02X}:{1:04X}!", bank, bank_addr);
    }

    #[cfg(test)]
    pub fn write_ws2_hi_rom_8(&mut self, value: u8, bank: usize, bank_addr: usize) {
        self.rom[((bank - mmap::WS2_HIROM_FIRST_BANK) << 16) | bank_addr] = value;
    }
}

use std::fs::File;
use std::io::prelude::*;

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
    lo_rom: Box<[u8]>,
    // TODO: hi_rom
    // TODO: Extra chips
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
            lo_rom: rom_bytes.into_boxed_slice(),
        }
    }
}

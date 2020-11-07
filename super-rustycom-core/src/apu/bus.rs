use crate::apu_io::ApuIo;

/// 64 kB of RAM including the IPL ROM at $FFC0-FFFF and mirrored I/O-ports at $00F0-$00FF
const RAM_SIZE: usize = 64 * 1024;

pub struct Bus {
    ram: Box<[u8]>,
    apu_io: ApuIo,
}

impl Bus {
    pub fn new() -> Bus {
        let mut ram = Box::new([0; RAM_SIZE]);
        ram[0xFFC0..].copy_from_slice(&IPL_ROM);
        Bus {
            ram,
            apu_io: ApuIo::new(),
        }
    }

    pub fn io(&self) -> &ApuIo {
        &self.apu_io
    }

    pub fn ram(&self) -> &[u8] {
        &self.ram
    }

    pub fn copy_cpu_io(&mut self, cpu_io: &ApuIo) {
        self.apu_io.copy_cpu(cpu_io);
    }

    pub fn access_write8(&mut self, addr: u16) -> &mut u8 {
        &mut self.ram[addr as usize]
    }

    pub fn access_page_write16(&mut self, addr: u16) -> impl FnMut(&mut Self, u16) {
        move |slef, value| {
            slef.ram[addr as usize] = value as u8;
            let msb_addr = (addr & 0xFF00) | ((addr as u8).wrapping_add(1) as u16);
            slef.ram[msb_addr as usize] = (value >> 8) as u8;
        }
    }

    pub fn read8(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    pub fn read16(&self, addr: u16) -> u16 {
        self.read8(addr) as u16 | ((self.read8(addr.wrapping_add(1)) as u16) << 8)
    }
}

// At $FFC0-$FFFF
// Takes care of init and (initial) data transfer, though ROMs leverage transfer again by jumping to $FFC0
const IPL_ROM: [u8; 64] = [
    0xCD, 0xEF, 0xBD, 0xE8, 0x00, 0xC6, 0x1D, 0xD0, 0xFC, 0x8F, 0xAA, 0xF4, 0x8F, 0xBB, 0xF5, 0x78,
    0xCC, 0xF4, 0xD0, 0xFB, 0x2F, 0x19, 0xEB, 0xF4, 0xD0, 0xFC, 0x7E, 0xF4, 0xD0, 0x0B, 0xE4, 0xF5,
    0xCB, 0xF4, 0xD7, 0x00, 0xFC, 0xD0, 0xF3, 0xAB, 0x01, 0x10, 0xEF, 0x7E, 0xF4, 0x10, 0xEB, 0xBA,
    0xF6, 0xDA, 0x00, 0xBA, 0xF4, 0xC4, 0xF4, 0xDD, 0x5D, 0xD0, 0xDB, 0x1F, 0x00, 0x00, 0xC0, 0xFF,
];

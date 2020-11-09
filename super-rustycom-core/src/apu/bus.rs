use crate::apu_io::ApuIo;

/// 64 kB of RAM including the IPL ROM at $FFC0-FFFF and mirrored I/O-ports at $00F0-$00FF
const RAM_SIZE: usize = 64 * 1024;

pub struct Bus {
    ram: Box<[u8]>,
    /// Store the values written by the cpu, ours are in the corresponding RAM addresses
    cpu_io: ApuIo,
}

impl Bus {
    pub fn default() -> Bus {
        let mut ram = Box::new([0; RAM_SIZE]);
        // Entrypoint
        ram[0x00] = 0xC0;
        ram[0x01] = 0xFF;
        ram[TEST] = 0x0A;
        ram[DSPADDR] = 0xFF;
        ram[DSPDATA] = 0x7F; // DSP[$7F]?
        ram[AUXIO4] = 0xFF; // UNUSED
        ram[AUXIO5] = 0xFF; // UNUSED
        ram[T0DIV] = 0xFF;
        ram[T1DIV] = 0xFF;
        ram[T2DIV] = 0xFF;
        ram[T0OUT] = 0x00;
        ram[T1OUT] = 0x00;
        ram[T2OUT] = 0x00;
        ram[0xFFC0..].copy_from_slice(&IPL_ROM);
        Bus {
            ram,
            cpu_io: ApuIo::default(),
        }
    }

    /// Returns whole RAM, includes all registers at the corresponding range
    pub fn ram(&self) -> &[u8] {
        &self.ram
    }

    /// Gets IO port values written by APU
    pub fn apu_io(&self) -> ApuIo {
        ApuIo::new(
            self.ram[0x00F4],
            self.ram[0x00F5],
            self.ram[0x00F6],
            self.ram[0x00F7],
        )
    }

    /// Updates CPU written values in the IO ports
    pub fn write_cpu_io(&mut self, io: ApuIo) {
        self.cpu_io = io;
    }

    /// Returns a mutable reference to the byte at `addr`
    pub fn mut_byte(&mut self, addr: u16) -> &mut u8 {
        // TODO: Model smp read-only (T0OUT-T2OUT)
        &mut self.ram[addr as usize]
    }

    /// Writes `value` at `addr` with page wrapping
    pub fn write_word(&mut self, addr: u16, value: u16) {
        *self.mut_byte(addr) = value as u8;
        let msb_addr = (addr & 0xFF00) | ((addr as u8).wrapping_add(1) as u16);
        *self.mut_byte(msb_addr) = (value >> 8) as u8;
    }

    /// Returns byte at `addr`
    pub fn byte(&self, addr: u16) -> u8 {
        // TODO: Model write-only (TEST, CONTROL, T0DIV-T2DIV)
        match addr as usize {
            // Cpu written IO is not in RAM
            CPUIO0..=CPUIO3 => self.cpu_io.read(((addr as u8) & 0xF) - 0x4),
            _ => self.ram[addr as usize],
        }
    }

    // Returns word at `addr` with page wrapping
    pub fn word(&self, addr: u16) -> u16 {
        self.byte(addr) as u16 | ((self.byte(addr.wrapping_add(1)) as u16) << 8)
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

const TEST: usize = 0x00F0;
#[allow(dead_code)]
const CONTROL: usize = 0x00F1;
const DSPADDR: usize = 0x00F2;
const DSPDATA: usize = 0x00F3;
const CPUIO0: usize = 0x00F4;
#[allow(dead_code)]
const CPUIO1: usize = 0x00F5;
#[allow(dead_code)]
const CPUIO2: usize = 0x00F6;
const CPUIO3: usize = 0x00F7;
const AUXIO4: usize = 0x00F8;
const AUXIO5: usize = 0x00F9;
const T0DIV: usize = 0x00FA;
const T1DIV: usize = 0x00FB;
const T2DIV: usize = 0x00FC;
const T0OUT: usize = 0x00FD;
const T1OUT: usize = 0x00FE;
const T2OUT: usize = 0x00FF;

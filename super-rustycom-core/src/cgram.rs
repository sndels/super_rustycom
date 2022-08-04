/// 512 bytes of color palette memory
const CGRAM_SIZE: usize = 512;

pub struct Cgram {
    cgadd: u8,
    odd_access: bool,
    cg_data_lsb: u8,
    mem: Box<[u8]>,
}

impl Cgram {
    pub fn mem(&self) -> &[u8] {
        &self.mem
    }

    pub fn write_addr(&mut self, addr: u8) {
        self.cgadd = addr;
        self.odd_access = false;
    }

    pub fn write_data(&mut self, value: u8) {
        if self.odd_access {
            let addr = (self.cgadd as u16) << 1;
            self.mem[addr as usize] = self.cg_data_lsb;
            self.mem[(addr + 1) as usize] = value;
        } else {
            self.cg_data_lsb = value;
        }
        self.increment();
    }

    pub fn read_data(&mut self) -> u8 {
        let value = self.peek_data();
        self.increment();
        value
    }

    pub fn peek_data(&self) -> u8 {
        let addr = ((self.cgadd as u16) << 1) | (self.odd_access as u16);
        self.mem[addr as usize]
    }

    pub fn increment(&mut self) {
        // Toggle 0->1 is inherently an increment, 1->0 needs to increment addr
        self.odd_access = !self.odd_access;
        if !self.odd_access {
            self.cgadd = self.cgadd.wrapping_add(1);
        }
    }
}

impl Default for Cgram {
    fn default() -> Self {
        Cgram {
            cgadd: 0x0,
            odd_access: true,
            cg_data_lsb: 0x0,
            mem: Box::new([0; CGRAM_SIZE]),
        }
    }
}

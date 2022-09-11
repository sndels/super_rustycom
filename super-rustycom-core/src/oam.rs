/// 544 bytes of Object Attribute Memory used for sprite info
const OAM_SIZE: usize = 544;

pub struct Oam {
    /// 9-bit
    reload: u16,
    /// "10-bit", store 9bit like in `reload` with odd_access to be used as bit0
    addr: u16,
    odd_access: bool,
    oam_data_lsb: u8,
    priority: u8,
    mem: Box<[u8]>,
}

impl Oam {
    pub fn mem(&self) -> &[u8] {
        &self.mem
    }

    pub fn write_oamddl(&mut self, addr: u8) {
        self.reload = self.reload & 0xFF00 | (addr as u16);
        self.addr = self.reload;
        self.odd_access = false;
    }

    pub fn write_oamddh(&mut self, addr: u8) {
        self.priority = addr >> 7;
        self.reload = (((addr & 0b1) as u16) << 8) | self.reload & 0xFF;
        self.addr = self.reload;
        self.odd_access = false;
    }

    pub fn write_data(&mut self, value: u8) {
        if !self.odd_access {
            self.oam_data_lsb = value;
        }
        let mut addr = (self.addr as u16) << 1;
        if addr > 0x1FF {
            if addr > 0x220 {
                addr -= 0x220;
                addr = addr % 0x20; // TODO: This can be done with a mask?
                addr += 0x200;
            }
            self.mem[addr as usize + self.odd_access as usize] = value;
        } else if self.odd_access {
            self.mem[addr as usize] = self.oam_data_lsb;
            self.mem[(addr + 1) as usize] = value;
        }
        self.increment();
    }

    pub fn read_data(&mut self) -> u8 {
        let value = self.peek_data();
        self.increment();
        value
    }

    pub fn peek_data(&self) -> u8 {
        let addr = ((self.addr as u16) << 1) | (self.odd_access as u16);
        self.mem[addr as usize]
    }

    pub fn increment(&mut self) {
        // Toggle 0->1 is inherently an increment, 1->0 needs to increment addr
        self.odd_access = !self.odd_access;
        if !self.odd_access {
            self.addr += 1;
            if self.addr > 0x3FF {
                self.addr = 0;
            }
        }
    }
}

impl Default for Oam {
    fn default() -> Self {
        Oam {
            reload: 0,
            addr: 0,
            odd_access: false,
            oam_data_lsb: 0,
            priority: 0,
            mem: Box::new([0; OAM_SIZE]),
        }
    }
}

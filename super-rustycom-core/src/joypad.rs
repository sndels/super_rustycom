pub struct JoyIo {
    joy_wr: u8,
    joy_a: u8,
    joy_b: u8,
    wr_io: u8,
    rd_io: u8,
    joy_1l: u8,
    joy_1h: u8,
    joy_2l: u8,
    joy_2h: u8,
    joy_3l: u8,
    joy_3h: u8,
    joy_4l: u8,
    joy_4h: u8,
}

impl JoyIo {
    pub fn new() -> JoyIo {
        JoyIo {
            joy_wr: 0x00,
            joy_a: 0x00,
            joy_b: 0x00,
            wr_io: 0xFF,
            rd_io: 0x00,
            joy_1l: 0x00,
            joy_1h: 0x00,
            joy_2l: 0x00,
            joy_2h: 0x00,
            joy_3l: 0x00,
            joy_3h: 0x00,
            joy_4l: 0x00,
            joy_4h: 0x00,
        }
    }

    pub fn joy_a(&self) -> u8 { self.joy_a }
    pub fn joy_b(&self) -> u8 { self.joy_b }
    pub fn rd_io(&self) -> u8 { self.rd_io }
    pub fn joy_1l(&self) -> u8 { self.joy_1l }
    pub fn joy_1h(&self) -> u8 { self.joy_1h }
    pub fn joy_2l(&self) -> u8 { self.joy_2l }
    pub fn joy_2h(&self) -> u8 { self.joy_2h }
    pub fn joy_3l(&self) -> u8 { self.joy_3l }
    pub fn joy_3h(&self) -> u8 { self.joy_3h }
    pub fn joy_4l(&self) -> u8 { self.joy_4l }
    pub fn joy_4h(&self) -> u8 { self.joy_4h }

    pub fn set_joy_wr(&mut self, value: u8) { self.joy_wr = value; }
    pub fn set_wr_io(&mut self, value: u8) { self.wr_io = value; }
}

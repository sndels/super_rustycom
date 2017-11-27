pub struct JoyIo {
    pub joy_wr: u8,
    pub joy_a: u8,
    pub joy_b: u8,
    pub wr_io: u8,
    pub rd_io: u8,
    pub joy_1l: u8,
    pub joy_1h: u8,
    pub joy_2l: u8,
    pub joy_2h: u8,
    pub joy_3l: u8,
    pub joy_3h: u8,
    pub joy_4l: u8,
    pub joy_4h: u8,
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
}

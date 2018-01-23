/// Models the multiplication and division registers in Ricoh 5A22
pub struct MpyDiv {
    multiplicand: u8,
    multiplier: u8,
    dividend: u16,
    divisor: u8,
    /// Also used for division remainder
    mpy_res: u16,
    div_res: u16,
}

impl MpyDiv {
    /// Initializes a new instance with default values
    pub fn new() -> MpyDiv {
        MpyDiv {
            multiplicand: 0xFF,
            multiplier: 0xFF,
            dividend: 0xFFFF,
            divisor: 0xFF,
            mpy_res: 0x0000, // TODO: Check result inits
            div_res: 0x0000,
        }
    }

    /// Sets `multiplicand` to `value`
    pub fn set_multiplicand(&mut self, value: u8) { self.multiplicand = value; }

    /// Sets `multiplier` to `value` and starts multiplication
    ///
    /// Result will be set to `mpy_res` and value of `div_res` will be that of `multiplier`.
    /// Currently does the operation immediately, timig should be added?
    pub fn set_multiplier_and_start_multiply(&mut self, value: u8) {
        self.multiplier = value;
        self.mpy_res = (self.multiplicand as u16) * (self.multiplier as u16); // TODO: Timing
        self.div_res = self.multiplier as u16;
    }

    /// Sets low byte of `dividend` to `value`
    pub fn set_dividend_low(&mut self, value: u8) {
        self.dividend = (self.dividend & 0xFF00) | value as u16;
    }

    /// Sets high byte of `dividend` to `value`
    pub fn set_dividend_high(&mut self, value: u8) {
        self.dividend = ((value as u16) << 8) | (self.dividend & 0x00FF);
    }

    /// Sets `divisor` to `value` and starts division
    ///
    /// Result will be set to `div_res` and remainder to `mpy_res`.
    /// Currently does the operation immediately, timig should be added?
    pub fn set_divisor_and_start_division(&mut self, value: u8) {
        self.divisor = value; // TODO: Timing
        if self.divisor == 0 {
            self.div_res = 0xFFFF;
            self.mpy_res = self.dividend;
        } else {
            self.div_res = self.dividend / self.divisor as u16;
            self.mpy_res = self.dividend % self.divisor as u16;
        }
    }

    /// Returns the low byte of the multiplication result / division remainder
    pub fn get_mpy_res_low(&self) -> u8 { self.mpy_res as u8 }

    /// Returns the high byte of the multiplication result / division remainder
    pub fn get_mpy_res_high(&self) -> u8 { (self.mpy_res >> 8) as u8 }

    /// Returns the low byte of the division result
    pub fn get_div_res_low(&self) -> u8 { self.div_res as u8 }

    /// Returns the high byte of the division result
    pub fn get_div_res_high(&self) -> u8 { (self.div_res >> 8) as u8 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpy() {
        let mut mpy_div = MpyDiv::new();
        // Straight multiplication
        mpy_div.set_multiplicand(0xFA);
        mpy_div.set_multiplier_and_start_multiply(0xFB);
        assert_eq!(0xF51E, mpy_div.mpy_res);
        assert_eq!(0x1E, mpy_div.get_mpy_res_low());
        assert_eq!(0xF5, mpy_div.get_mpy_res_high());
        // Side-effect
        assert_eq!(0x00FB, mpy_div.div_res); // According to fullsnes

        // Only b triggers mpy
        mpy_div.set_multiplicand(0xFB);
        assert_eq!(0xF51E, mpy_div.mpy_res);

        // By zero
        mpy_div.set_multiplier_and_start_multiply(0x00);
        assert_eq!(0x0000, mpy_div.mpy_res);
    }

    #[test]
    fn div() {
        let mut mpy_div = MpyDiv::new();
        // Straight division
        mpy_div.set_dividend_low(0xFB);
        mpy_div.set_dividend_high(0xFA);
        mpy_div.set_divisor_and_start_division(0x1A);
        assert_eq!(0x09A7, mpy_div.div_res);
        assert_eq!(0xA7, mpy_div.get_div_res_low());
        assert_eq!(0x09, mpy_div.get_div_res_high());
        assert_eq!(0x0005, mpy_div.mpy_res);

        // Only divisor triggers div
        mpy_div.set_dividend_low(0xFD);
        mpy_div.set_dividend_high(0xFC);
        assert_eq!(0x09A7, mpy_div.div_res);
        assert_eq!(0x0005, mpy_div.mpy_res);

        // Zero remainder
        mpy_div.set_divisor_and_start_division(0x01);
        assert_eq!(0xFCFD, mpy_div.div_res);
        assert_eq!(0x0000, mpy_div.mpy_res);

        // Only remainder
        mpy_div.set_dividend_low(0x09);
        mpy_div.set_dividend_high(0x00);
        mpy_div.set_divisor_and_start_division(0x1A);
        assert_eq!(0x0000, mpy_div.div_res);
        assert_eq!(0x0009, mpy_div.mpy_res);

        // Division by zero
        mpy_div.set_dividend_low(0xFB);
        mpy_div.set_dividend_high(0xFA);
        mpy_div.set_divisor_and_start_division(0x00);
        assert_eq!(0xFFFF, mpy_div.div_res);
        assert_eq!(0xFAFB, mpy_div.mpy_res);
    }
}

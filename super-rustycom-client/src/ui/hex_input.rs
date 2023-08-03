// Adapted from https://github.com/emilk/egui/discussions/1112

use egui::{Response, Ui};

/// Hexadecimal input.
pub struct HexInput<'a, T>
where
    T: UpperHexPadded + num_traits::Num,
{
    target: &'a mut T,
}

impl<'a, T> HexInput<'a, T>
where
    T: UpperHexPadded + num_traits::Num,
{
    pub fn new(target: &'a mut T) -> Self {
        Self { target }
    }
}

impl<'a, T> egui::Widget for HexInput<'a, T>
where
    T: UpperHexPadded + num_traits::Num,
{
    fn ui(self, ui: &mut Ui) -> Response {
        let mut str_hex = self.target.upper_hex_padded();

        let text_edit = egui::TextEdit::singleline(&mut str_hex);

        let response = ui.add(text_edit);

        if response.changed() {
            if let Ok(v) = T::from_str_radix(&str_hex, 16) {
                *self.target = v;
            }
        }
        response
    }
}

pub trait UpperHexPadded {
    fn upper_hex_padded(&self) -> String {
        unimplemented!()
    }
}

impl UpperHexPadded for u32 {
    fn upper_hex_padded(&self) -> String {
        format!("{:#08X}", self)
    }
}

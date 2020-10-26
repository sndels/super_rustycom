mod font;

use self::font::Font;

pub struct TextRenderer {
    font: Font,
    line_spacing: usize,
    char_spacing: usize,
}

impl TextRenderer {
    pub fn new() -> TextRenderer {
        TextRenderer {
            font: Font::new(),
            line_spacing: 1,
            char_spacing: 1,
        }
    }

    pub fn draw(&self, text: String, mut pixel_buffer: Vec<&mut [u32]>) {
        let mut start_pixel_row = self.line_spacing;
        let mut start_pixel_column = self.char_spacing;
        let row_count = pixel_buffer.len();
        let column_count = pixel_buffer[0].len();
        for c in text.chars() {
            if c == '\n' {
                start_pixel_column = self.char_spacing;
                start_pixel_row += self.font.height + self.line_spacing;
                if start_pixel_row >= row_count {
                    break;
                }
            } else {
                self.draw_char(
                    c,
                    start_pixel_column,
                    &mut pixel_buffer[start_pixel_row..start_pixel_row + self.font.height],
                );

                start_pixel_column += self.font.width + self.char_spacing;
                if start_pixel_column >= column_count {
                    start_pixel_column = self.char_spacing;
                    start_pixel_row += self.font.height + self.line_spacing;
                    if start_pixel_row >= row_count {
                        break;
                    }
                }
            }
        }
    }

    fn draw_char(&self, c: char, start_pixel_column: usize, pixel_rows: &mut [&mut [u32]]) {
        let char_bits = self.font.chars.get(&c).unwrap();
        for font_column in 0..self.font.width {
            let output_column = start_pixel_column + font_column;
            for row in 0..self.font.height {
                pixel_rows[row][output_column] = 0xFFFFFFFF * char_bits[row][font_column];
            }
        }
    }
}

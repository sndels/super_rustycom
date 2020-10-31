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

    pub fn line_height(&self) -> usize {
        self.font.height + self.line_spacing
    }

    /// Draws the text in the given pixel buffer line by line.
    /// Overflowing characters in either dimension are ignored.
    pub fn draw<'a, T>(&self, text: T, color: u32, mut pixel_buffer: Vec<&mut [u32]>)
    where
        T: IntoIterator<Item = &'a String>,
    {
        let window_height = pixel_buffer.len();
        if window_height == 0 {
            return;
        }
        let window_width = pixel_buffer[0].len();
        if window_width == 0
            || window_width <= (self.font.width + self.char_spacing)
            || window_height <= (self.font.height + self.line_spacing)
        {
            return;
        }

        let mut start_pixel_row = 0;
        for line in text {
            // Make sure we don't run out of vertical pixels
            if start_pixel_row + self.font.height >= window_height {
                break;
            }

            let mut start_pixel_column = 0;
            for c in line.chars() {
                // Don't draw if we've ran out of space on the line
                if start_pixel_column + self.font.width >= window_width {
                    break;
                }

                self.draw_char(
                    c,
                    color,
                    start_pixel_column,
                    &mut pixel_buffer[start_pixel_row..start_pixel_row + self.font.height],
                );
                start_pixel_column += self.font.width + self.char_spacing;
            }
            start_pixel_row += self.font.height + self.line_spacing;
        }
    }

    fn draw_char(
        &self,
        c: char,
        color: u32,
        start_pixel_column: usize,
        pixel_rows: &mut [&mut [u32]],
    ) {
        let char_bits = self.font.chars[c as usize];
        for font_column in 0..self.font.width {
            let output_column = start_pixel_column + font_column;
            for row in 0..self.font.height {
                pixel_rows[row][output_column] = color * char_bits[row][font_column];
            }
        }
    }
}

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
        let window_width = pixel_buffer[0].len();
        let window_height = pixel_buffer.len();
        assert!(window_width > self.font.width + self.char_spacing);
        assert!(window_height > self.font.height + self.line_spacing);

        let mut start_pixel_row = 0;
        let mut start_pixel_column = 0;
        // Let's use a peekable iterator instead of straight loop since we need this in column logic
        let mut iter = text.chars().into_iter().peekable();
        while let Some(c) = iter.next() {
            macro_rules! change_line {
                () => {
                    start_pixel_column = 0;
                    start_pixel_row += self.font.height + self.line_spacing;
                };
            };

            // Make sure we don't run out of vertical pixels
            if start_pixel_row + self.font.height >= window_height {
                break;
            }

            if c == '\n' {
                change_line!();
            } else {
                self.draw_char(
                    c,
                    start_pixel_column,
                    &mut pixel_buffer[start_pixel_row..start_pixel_row + self.font.height],
                );

                start_pixel_column += self.font.width + self.char_spacing;
                if iter.peek() != Some(&'\n')
                    && start_pixel_column + self.font.width >= window_width
                {
                    change_line!();
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

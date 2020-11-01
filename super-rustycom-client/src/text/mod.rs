mod font;

use self::font::Font;
use log::warn;

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

    pub fn window_size<'a, T>(&self, text: T) -> (usize, usize)
    where
        T: ExactSizeIterator<Item = &'a String>,
    {
        if text.len() == 0 {
            return (0, 0);
        }
        let line_count = text.len();
        let max_row_length = text.max_by(|x, y| x.len().cmp(&y.len())).unwrap().len();
        // Last line/character don't need spacing after them
        let height = (self.font.height + self.line_spacing) * line_count - self.line_spacing;
        // Saturate in case we call with all empty lines
        let width = ((self.font.width + self.char_spacing) * max_row_length)
            .saturating_sub(self.char_spacing);
        (width, height)
    }

    /// Draws the text in the given pixel buffer line by line.
    /// Overflowing characters in either dimension are ignored.
    pub fn draw<'a, T>(&self, text: T, color: u32, mut buffer_window: Vec<&mut [u32]>)
    where
        T: IntoIterator<Item = &'a String>,
    {
        let window_height = buffer_window.len();
        if window_height == 0 || window_height < self.font.height {
            warn!(
                "Tried rendering text with window height ({}) smaller than font height ({})",
                window_height, self.font.height
            );
            return;
        }
        let window_width = buffer_window[0].len();
        if window_width == 0 || window_width < self.font.width {
            warn!(
                "Tried rendering text with window width ({}) smaller than font width ({})",
                window_width, self.font.width
            );
            return;
        }

        let mut start_pixel_row = 0;
        // We really don't want to dump all lines to the log if we ran out of width
        let mut warned_about_line_length = false;
        for line in text {
            // Make sure we don't run out of vertical pixels
            if start_pixel_row + self.font.height > window_height {
                warn!("Ran out of window lines before line '{}'", line);
                break;
            }

            let mut start_pixel_column = 0;
            for c in line.chars() {
                // Don't draw if we've ran out of space on the line
                if start_pixel_column + self.font.width > window_width {
                    if !warned_about_line_length {
                        warn!("Ran out of window columns on line '{}'", line);
                        warned_about_line_length = true
                    }
                    break;
                }

                self.draw_char(
                    c,
                    color,
                    start_pixel_column,
                    &mut buffer_window[start_pixel_row..start_pixel_row + self.font.height],
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

use crate::config::Config;

pub struct Framebuffer {
    buffer: Vec<u32>,
    width: usize,
    height: usize,
}

impl Framebuffer {
    pub fn new(config: &Config) -> Framebuffer {
        Framebuffer {
            buffer: vec![0; config.resolution.width * config.resolution.height],
            width: config.resolution.width,
            height: config.resolution.height,
        }
    }

    pub fn buffer(&self) -> &Vec<u32> {
        &self.buffer
    }

    pub fn clear(&mut self, color: u32) {
        self.buffer = vec![color; self.width * self.height];
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.buffer = vec![0; width * height];
        self.width = width;
        self.height = height;
    }

    /// Returns mutable slices to the requested window in the buffer.
    /// Clamps to borders if given too large dimensions.
    /// Returns empty Vec if top and/or left is out of bounds.
    pub fn window(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) -> Vec<&mut [u32]> {
        if left >= self.width || top >= self.height {
            return Vec::new();
        }
        let clamped_width = if left + width < self.width {
            width
        } else {
            width - (left + width - self.width)
        };
        let clamped_height = if top + height < self.height {
            height
        } else {
            height - (top + height - self.height)
        };

        let mut slices = Vec::new();
        let (_, mut rest) = self.buffer.split_at_mut(top * self.width + left);
        // Do n-1 loops as priming after the last split will fail if the window
        // extends to the last line of pixels
        for _ in 0..clamped_height.saturating_sub(1) {
            let (head, tail) = rest.split_at_mut(clamped_width);
            slices.push(head);
            // Split at next pixel in window to prime the next iteration
            let (_, tail) = tail.split_at_mut(self.width - clamped_width);
            rest = tail;
        }
        let (head, _) = rest.split_at_mut(clamped_width);
        slices.push(head);

        slices
    }
}

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
            width - (left + width - self.width) - 1
        };
        let clamped_height = if top + height < self.height {
            height
        } else {
            height - (top + height - self.height) - 1
        };

        let mut slices = Vec::new();
        let (_, mut rest) = self.buffer.split_at_mut(top * self.width + left);
        for _ in 0..clamped_height {
            let (head, tail) = rest.split_at_mut(clamped_width);
            slices.push(head);
            let (_, tail) = tail.split_at_mut(self.width - clamped_width);
            rest = tail;
        }
        slices
    }
}

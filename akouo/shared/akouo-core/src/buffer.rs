// Gapless playback buffer manager

use crate::Result;

pub struct AudioBuffer {
    samples: Vec<i16>,
    position: usize,
}

impl AudioBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
            position: 0,
        }
    }

    pub fn write(&mut self, data: &[i16]) -> Result<usize> {
        self.samples.extend_from_slice(data);
        Ok(data.len())
    }

    pub fn read(&mut self, output: &mut [i16]) -> Result<usize> {
        let available = self.samples.len() - self.position;
        let to_read = available.min(output.len());

        output[..to_read].copy_from_slice(&self.samples[self.position..self.position + to_read]);

        self.position += to_read;

        if self.position >= self.samples.len() {
            self.samples.clear();
            self.position = 0;
        }

        Ok(to_read)
    }

    pub fn available(&self) -> usize {
        self.samples.len() - self.position
    }
}

use std::time::Instant;

pub struct TimeSource {
    start: Instant,
}

impl TimeSource {
    pub fn new() -> TimeSource {
        TimeSource {
            start: Instant::now(),
        }
    }

    pub fn elapsed_ns(&self) -> u64 {
        let elapsed = self.start.elapsed();
        elapsed.as_secs() * 1000000000 + elapsed.subsec_nanos() as u64
    }
}

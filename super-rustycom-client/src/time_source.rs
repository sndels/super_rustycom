use std::time::Instant;

// NTSC clock runs at 21.447MHz so this is really 46.56... but we don't care about absolute accuracy
const TICK_NANOS: u128 = 47;

pub struct TimeSource {
    start: Instant,
}

impl TimeSource {
    pub fn new() -> TimeSource {
        TimeSource {
            start: Instant::now(),
        }
    }

    pub fn elapsed_ticks(&self) -> u128 {
        let nanos = self.start.elapsed().as_nanos();
        nanos / TICK_NANOS
    }
}

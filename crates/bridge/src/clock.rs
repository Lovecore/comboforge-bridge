use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Monotonic microseconds since helper start, plus a wall anchor for the
/// page's offset estimation. Timestamps are taken at the POLL instant, not
/// at send -- that is the accuracy the browser's rAF grid cannot offer.
pub struct Clock {
    start: Instant,
}

impl Clock {
    pub fn start() -> Self {
        Clock {
            start: Instant::now(),
        }
    }

    pub fn t_us(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }

    pub fn wall_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

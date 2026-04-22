use std::time::Instant;

#[derive(Debug, Clone)]
pub struct SessionClock {
    origin: Instant,
}

impl SessionClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }

    pub fn restart(&mut self) {
        self.origin = Instant::now();
    }

    pub fn now_ms(&self) -> u64 {
        let elapsed = self.origin.elapsed().as_millis();
        elapsed.min(u128::from(u64::MAX)) as u64
    }
}

impl Default for SessionClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SessionClock;

    #[test]
    fn clock_reports_elapsed_time() {
        let clock = SessionClock::new();
        assert!(clock.now_ms() <= 1_000);
    }
}

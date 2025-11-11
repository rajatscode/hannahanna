// Clock abstraction for testing time-dependent code
use std::time::{Duration, Instant};

/// Trait for abstracting time operations to enable testing
pub trait Clock: Send + Sync {
    /// Get the current instant
    fn now(&self) -> Instant;

    /// Sleep for the given duration
    fn sleep(&self, duration: Duration);
}

/// System clock implementation using real time
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    /// Fake clock for testing that doesn't use real time
    #[derive(Clone)]
    pub struct FakeClock {
        time: Arc<Mutex<Instant>>,
    }

    impl FakeClock {
        pub fn new() -> Self {
            Self {
                time: Arc::new(Mutex::new(Instant::now())),
            }
        }

        /// Advance the clock by the given duration
        pub fn advance(&self, duration: Duration) {
            let mut time = self.time.lock().unwrap();
            *time = *time + duration;
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> Instant {
            *self.time.lock().unwrap()
        }

        fn sleep(&self, duration: Duration) {
            // In tests, we advance time manually instead of sleeping
            self.advance(duration);
        }
    }
}

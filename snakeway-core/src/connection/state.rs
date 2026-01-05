use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct RouteConnectionState {
    active: AtomicUsize,
    max: Option<usize>,
}

impl RouteConnectionState {
    pub fn new(max: Option<usize>) -> Self {
        Self {
            active: AtomicUsize::new(0),
            max,
        }
    }

    /// Attempt to acquire one connection slot.
    /// On success, the counter is incremented atomically.
    pub fn try_acquire(&self) -> bool {
        match self.max {
            None => {
                // Unlimited
                self.active.fetch_add(1, Ordering::Relaxed);
                true
            }
            Some(max) => {
                let mut current = self.active.load(Ordering::Relaxed);
                loop {
                    if current >= max {
                        return false;
                    }
                    match self.active.compare_exchange_weak(
                        current,
                        current + 1,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => return true,
                        Err(actual) => current = actual,
                    }
                }
            }
        }
    }

    pub fn release(&self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn active(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }
}

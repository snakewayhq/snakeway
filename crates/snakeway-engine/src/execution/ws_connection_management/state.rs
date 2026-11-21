use std::sync::atomic::{AtomicUsize, Ordering};

/// Encodes "no limit" in the atomic max to allow a reload to change the limit
/// between acquisitions without a lock.
const UNLIMITED: usize = usize::MAX;

#[derive(Debug)]
pub(crate) struct WsRouteConnectionState {
    active: AtomicUsize,
    max: AtomicUsize,
}

impl WsRouteConnectionState {
    pub(crate) fn new(max: Option<usize>) -> Self {
        Self {
            active: AtomicUsize::new(0),
            max: AtomicUsize::new(max.unwrap_or(UNLIMITED)),
        }
    }

    /// Store the limit that applies to acquisitions from now on. Already
    /// admitted connections are not affected.
    pub(crate) fn set_max(&self, max: Option<usize>) {
        self.max.store(max.unwrap_or(UNLIMITED), Ordering::Relaxed);
    }

    /// Attempt to acquire one connection slot against the limit in force now.
    /// On success, the counter is incremented atomically.
    pub(crate) fn try_acquire(&self) -> bool {
        let max = self.max.load(Ordering::Relaxed);
        if max == UNLIMITED {
            self.active.fetch_add(1, Ordering::Relaxed);
            return true;
        }

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

    pub(crate) fn release(&self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }

    pub(crate) fn active(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }

    pub(crate) fn max(&self) -> Option<usize> {
        match self.max.load(Ordering::Relaxed) {
            UNLIMITED => None,
            max => Some(max),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raised_limit_applies_to_the_next_acquisition() {
        // Arrange
        let state = WsRouteConnectionState::new(Some(1));
        assert!(state.try_acquire());
        state.release();

        // Act
        state.set_max(Some(5));

        // Assert: two sequential acquisitions succeed under the raised limit.
        assert!(state.try_acquire());
        state.release();
        assert!(state.try_acquire());
        state.release();
    }

    #[test]
    fn a_lowered_limit_refuses_the_next_acquisition_over_it() {
        // Arrange
        let state = WsRouteConnectionState::new(Some(5));
        state.try_acquire();
        state.try_acquire();
        state.set_max(Some(1));

        // Act
        let admitted = state.try_acquire();

        // Assert
        assert!(!admitted);
        state.release();
        state.release();
    }

    #[test]
    fn a_limit_can_replace_unlimited() {
        // Arrange: one connection is admitted while unlimited.
        let state = WsRouteConnectionState::new(None);
        assert!(state.try_acquire());
        assert_eq!(state.active(), 1);

        // Act
        state.set_max(Some(1));

        // Assert: the second connection is refused at the new limit, and the
        // admitted one is not torn down.
        assert!(!state.try_acquire());
        assert_eq!(state.active(), 1);
        state.release();
    }
}

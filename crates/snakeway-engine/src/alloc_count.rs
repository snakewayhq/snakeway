//! Test-only allocator that counts heap allocations made inside a measured closure.
//!
//! Counting is per thread, so tests that run in parallel do not see each other's allocations.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountingAllocator;

thread_local! {
    static IS_COUNTING_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
    static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
}

fn count_allocation_if_tracking() {
    let _ = IS_COUNTING_ALLOCATIONS.try_with(|tracking| {
        if tracking.get() {
            let _ = ALLOCATION_COUNT.try_with(|count| count.set(count.get() + 1));
        }
    });
}

// SAFETY: every method delegates to the system allocator with the same arguments.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation_if_tracking();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation_if_tracking();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_allocation_if_tracking();
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Runs `f` and returns its result with the number of allocations and reallocations it made.
pub(crate) fn count_allocations<R>(f: impl FnOnce() -> R) -> (R, usize) {
    ALLOCATION_COUNT.with(|count| count.set(0));
    IS_COUNTING_ALLOCATIONS.with(|tracking| tracking.set(true));
    let result = f();
    IS_COUNTING_ALLOCATIONS.with(|tracking| tracking.set(false));
    (result, ALLOCATION_COUNT.with(Cell::get))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_each_allocation_inside_the_closure() {
        // Arrange
        let outside = String::from("made before measuring");

        // Act
        let (inner, count) = count_allocations(|| (Box::new(7_u32), String::from("abc")));

        // Assert
        assert_eq!(count, 2);
        assert_eq!(*inner.0, 7);
        assert_eq!(inner.1, "abc");
        assert_eq!(outside, "made before measuring");
    }

    #[test]
    fn a_closure_that_does_not_allocate_counts_zero() {
        // Arrange
        let values = [3_u32, 4, 5];

        // Act
        let (sum, count) = count_allocations(|| values.iter().sum::<u32>());

        // Assert
        assert_eq!(count, 0);
        assert_eq!(sum, 12);
    }
}

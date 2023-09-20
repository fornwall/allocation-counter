/*!
This crate provides a method to count allocations while running some code.

# Example
Add as a dependency - since including the trait replaces the global memory allocator, you most likely want it gated behind a feature:

```toml
[features]
count-allocations = ["allocation-counter"]

[dependencies]
allocation-counter = { version = "0", optional = true }
```

Tests can now be written to assert that the number of desired memory allocations are not exceeded:

```
#[cfg(feature = "count-allocations")]
#[test]
pub fn no_memory_allocations() {
    let allocations = allocation_counter::count(|| {
        code_that_should_not_allocate_memory();
    });
    assert_eq!(allocations, 0);
}
```

Run the tests with the necessary feature enabled:

```sh
cargo test --features count-allocations
```
*/
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;

/// Run a closure while counting the performed memory allocations.
///
/// Will only measure those done by the current thread, so take care when
/// interpreting the returned count for multithreaded programs.
///
/// # Arguments
///
/// - `run_while_counting` - The code to run while counting allocations
///
/// # Examples
///
/// ```
/// let allocations = allocation_counter::count(|| {
///      "hello, world".to_string();
/// });
/// assert_eq!(allocations, 1);
/// ```
pub fn count<F: FnOnce()>(run_while_counting: F) -> u64 {
    let initial_count = ALLOCATIONS.with(|f| *f.borrow());

    run_while_counting();

    ALLOCATIONS.with(|f| *f.borrow()) - initial_count
}

thread_local! {
    static ALLOCATIONS: RefCell<u64> = RefCell::new(0);
}

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCATIONS.with(|f| {
            *f.borrow_mut() += 1;
        });

        System.alloc(l)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, l: Layout) {
        System.dealloc(ptr, l);
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator {};

#[test]
fn test_closure() {
    let allocations = count(|| {
        // Do nothing.
    });
    assert_eq!(allocations, 0);

    let allocations = count(|| {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });
    assert_eq!(allocations, 1);

    let allocations = count(|| {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });
    assert_eq!(allocations, 1);

    let allocations = count(|| {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });
    assert_eq!(allocations, 2);
}

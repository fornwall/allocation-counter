/*!
This crate provides a method to count and test the number of allocations while running some code.

# Example
Add as a dependency - since including the trait replaces the global memory allocator, you most likely want it gated behind a feature.

```toml
[features]
count-allocations = ["allocation-counter"]

[dependencies]
allocation-counter = { version = "0", optional = true }
```

Tests can now be written to assert that the number of desired memory allocations are not exceeded.

```
#[cfg(feature = "count-allocations")]
#[test]
pub fn no_memory_allocations() {
    # fn code_that_should_not_allocate_memory() {}
    # fn code_that_should_not_allocate_much() {}
    let allocations = allocation_counter::count(|| {
        code_that_should_not_allocate_memory();
    });
    assert_eq!(allocations, 0);

    // Or use this utility method in this case:
    allocation_counter::assert_no_allocations(|| {
        code_that_should_not_allocate_memory();
    });

    // Can also allow a certain number of allocations:
    allocation_counter::assert_max_allocations(10 || {
        code_that_should_not_allocate_much();
    });

    // Can also assert on a range, useful to adjust
    // test expectations over time:
    allocation_counter::assert_num_allocations(500..600 || {
        code_that_should_not_allocate_much();
    });

    // It's possible to opt out of counting allocations
    // for certain parts of the code flow:
    allocation_counter::assert_no_allocations(|| {
        code_that_should_not_allocate();
        allocation_counter::avoid_counting(|| {
            external_code_that_should_not_be_tested();
        });
        code_that_should_not_allocate();
    });
}
```

Run the tests with the necessary feature enabled.

```sh
cargo test --features count-allocations
```
*/
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;

/// Run a closure while counting the performed memory allocations.
///
/// Will only measure those done by the current thread, so take care when
/// interpreting the returned count for multithreaded code.
///
/// # Arguments
///
/// - `run_while_counting` - The code to run while counting allocations
///
/// # Examples
///
/// ```
/// # fn code_that_should_not_allocate_memory() {}
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

/// Run a closure and assert that no memory allocations were made.
///
/// Will only measure those done by the current thread, so take care when
/// testing the memory allocations of multithreaded code.
///
/// # Arguments
///
/// - `run_while_counting` - The code to run while counting allocations
///
/// # Examples
///
/// ```
/// # fn code_that_should_not_allocate_memory() {}
/// allocation_counter::assert_no_allocations(|| {
///     code_that_should_not_allocate_memory();
/// });
/// ```
pub fn assert_no_allocations<F: FnOnce()>(run_while_counting: F) {
    assert_max_allocations(0, run_while_counting);
}

/// Run a closure and assert that the number of memory allocations are below a limit.
///
/// Will only measure those done by the current thread, so take care when
/// testing the memory allocations of multithreaded code.
///
/// # Arguments
///
/// - `max_allocations` - The maximum number of allocations allowed
/// - `run_while_counting` - The code to run while counting allocations
///
/// # Examples
///
/// ```
/// # fn code_that_should_not_allocate_much() {}
/// allocation_counter::assert_max_allocations(12, || {
///     code_that_should_not_allocate_much();
/// });
/// ```
pub fn assert_max_allocations<F: FnOnce()>(max_allocations: u64, run_while_counting: F) {
    let initial_count = ALLOCATIONS.with(|f| *f.borrow());

    run_while_counting();

    let num_allocations = ALLOCATIONS.with(|f| *f.borrow()) - initial_count;
    assert!(
        num_allocations <= max_allocations,
        "Unexpected memory allocations (more than {}): {}",
        max_allocations,
        num_allocations
    );
}

/// Run a closure and assert that the number of memory allocations are inside a range.
///
/// Will only measure those done by the current thread, so take care when
/// testing the memory allocations of multithreaded code.
///
/// # Arguments
///
/// - `allowed_allocations` - The range of allocations allowed
/// - `run_while_counting` - The code to run while counting allocations
///
/// # Examples
///
/// ```
/// # fn code_that_should_not_allocate_much() {}
/// allocation_counter::assert_max_allocations(12, || {
///     code_that_should_not_allocate_much();
/// });
/// ```
pub fn assert_num_allocations<F: FnOnce()>(
    allowed_allocations: std::ops::Range<u64>,
    run_while_counting: F,
) {
    let initial_count = ALLOCATIONS.with(|f| *f.borrow());

    run_while_counting();

    let num_allocations = ALLOCATIONS.with(|f| *f.borrow()) - initial_count;
    assert!(
        allowed_allocations.contains(&num_allocations),
        "Unexpected memory allocations (outside of {:?}): {}",
        allowed_allocations,
        num_allocations
    );
}

/// Opt out of counting allocations while running some code.
///
/// Useful to avoid certain parts of the code flow that should not be counted.
///
/// # Arguments
///
/// - `run_while_not_counting` - The code to run while not counting allocations
///
/// # Examples
///
/// ```
/// # fn code_that_should_not_allocate() {}
/// # fn external_code_that_should_not_be_tested() {}
/// allocation_counter::assert_no_allocations(|| {
///     code_that_should_not_allocate();
///     allocation_counter::avoid_counting(|| {
///         external_code_that_should_not_be_tested();
///     });
///     code_that_should_not_allocate();
/// });
/// ```
pub fn avoid_counting<F: FnOnce()>(run_while_not_counting: F) {
    DO_COUNT.with(|b| {
        *b.borrow_mut() += 1;
        run_while_not_counting();
        *b.borrow_mut() -= 1;
    });
}

thread_local! {
    static ALLOCATIONS: RefCell<u64> = RefCell::new(0);
}
thread_local! {
    static DO_COUNT: RefCell<u32> = RefCell::new(0);
}

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        DO_COUNT.with(|b| {
            if *b.borrow() == 0 {
                ALLOCATIONS.with(|f| {
                    *f.borrow_mut() += 1;
                });
            }
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
fn test_basic() {
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

    assert_no_allocations(|| {
        // Do nothing
    });

    assert_max_allocations(2, || {
        // Do nothing
    });

    assert_max_allocations(2, || {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });

    assert_num_allocations(1..3, || {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });

    assert_num_allocations(2..3, || {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });
}

#[test]
#[should_panic(expected = "Unexpected memory allocations (more than 0): 1")]
fn test_assert_no_allocations_panic() {
    assert_no_allocations(|| {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });
}

#[test]
#[should_panic(expected = "Unexpected memory allocations (more than 1): 2")]
fn test_assert_max_allocations_panic() {
    assert_max_allocations(1, || {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });
}

#[test]
#[should_panic(expected = "Unexpected memory allocations (outside of 10..12): 2")]
fn test_assert_num_allocations_panic() {
    assert_num_allocations(10..12, || {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });
}

#[test]
fn test_avoid_counting() {
    let allocations = count(|| {
        // Do nothing.
    });
    assert_eq!(allocations, 0);

    let allocations = count(|| {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
        avoid_counting(|| {
            let v: Vec<u32> = vec![12];
            assert_eq!(v.len(), 1);
            avoid_counting(|| {
                let v: Vec<u32> = vec![12];
                assert_eq!(v.len(), 1);
            });
        });
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });
    assert_eq!(allocations, 3);

    assert_no_allocations(|| {
        avoid_counting(|| {
            let v: Vec<u32> = vec![12];
            assert_eq!(v.len(), 1);
        });
    });
}

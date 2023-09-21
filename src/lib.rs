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

pub(crate) mod allocator;

#[derive(Clone, Copy, Default)]
pub struct AllocationInfo {
    num_allocations: u64,
    total_bytes_allocated: u64,
    max_bytes_allocated: u64,
    current_bytes_allocated: i64,
}

impl AllocationInfo {
    pub const fn num_allocations(&self) -> u64 {
        self.num_allocations
    }

    pub const fn total_bytes_allocated(&self) -> u64 {
        self.total_bytes_allocated
    }

    pub const fn current_bytes_allocated(&self) -> i64 {
        self.current_bytes_allocated
    }

    pub const fn max_bytes_allocated(&self) -> u64 {
        self.max_bytes_allocated
    }
}

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
    measure(run_while_counting).num_allocations()
}

pub fn measure<F: FnOnce()>(run_while_counting: F) -> AllocationInfo {
    allocator::ALLOCATIONS.with(|info_stack| {
        let mut info_stack = info_stack.borrow_mut();
        info_stack.depth += 1;
        assert!(
            (info_stack.depth as usize) < allocator::MAX_DEPTH,
            "Too deep allocation measuring nesting"
        );
        let depth = info_stack.depth;
        info_stack.elements[depth as usize] = AllocationInfo::default();
    });

    run_while_counting();

    allocator::ALLOCATIONS.with(|info_stack| {
        let mut info_stack = info_stack.borrow_mut();
        let depth = info_stack.depth;
        let popped = info_stack.elements[depth as usize];
        info_stack.depth -= 1;
        let depth = info_stack.depth as usize;
        info_stack.elements[depth].num_allocations += popped.num_allocations;
        info_stack.elements[depth].total_bytes_allocated += popped.total_bytes_allocated;
        info_stack.elements[depth].current_bytes_allocated += popped.current_bytes_allocated;
        info_stack.elements[depth].max_bytes_allocated += popped.max_bytes_allocated;
        popped
    })
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
    allocator::DO_COUNT.with(|b| {
        *b.borrow_mut() += 1;
        run_while_not_counting();
        *b.borrow_mut() -= 1;
    });
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
    let num_allocations = count(run_while_counting);
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
    let num_allocations = count(run_while_counting);
    assert!(
        allowed_allocations.contains(&num_allocations),
        "Unexpected memory allocations (outside of {:?}): {}",
        allowed_allocations,
        num_allocations
    );
}

#[test]
fn test_basic() {
    let allocations = count(|| {
        // Do nothing.
    });
    assert_eq!(allocations, 0);

    let info = measure(|| {
        // Do nothing.
    });
    assert_eq!(info.num_allocations(), 0);
    assert_eq!(info.total_bytes_allocated(), 0);
    assert_eq!(info.current_bytes_allocated(), 0);

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

    let info = measure(|| {
        let _a = std::hint::black_box(Box::new(1_u32));
        let _b = std::hint::black_box(Box::new(1_u32));
    });
    assert_eq!(info.num_allocations(), 2);
    assert_eq!(info.total_bytes_allocated(), 8);
    assert_eq!(info.current_bytes_allocated(), 0);

    let info = measure(|| {
        let _a = std::hint::black_box(Box::new(1_u32));
        let b = std::hint::black_box(Box::new(1_u32));
        std::mem::forget(b);
    });
    assert_eq!(info.num_allocations(), 2);
    assert_eq!(info.total_bytes_allocated(), 8);
    assert_eq!(info.current_bytes_allocated(), 4);
    assert_eq!(info.max_bytes_allocated, 8);

    let info = measure(|| {
        let a = std::hint::black_box(Box::new(1_u32));
        let b = std::hint::black_box(Box::new(1_u32));
        let _c = std::hint::black_box(Box::new(*a + *b));
    });
    assert_eq!(info.num_allocations(), 3);
    assert_eq!(info.total_bytes_allocated(), 12);
    assert_eq!(info.current_bytes_allocated(), 0);
    assert_eq!(info.max_bytes_allocated, 12);

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

#[test]
fn test_nested_counting() {
    let info = measure(|| {
        let _a = std::hint::black_box(Box::new(1_u32));
        let info = measure(|| {
            let _b = std::hint::black_box(Box::new(1_u32));
        });
        assert_eq!(info.num_allocations(), 1);
        assert_eq!(info.total_bytes_allocated(), 4);
        assert_eq!(info.current_bytes_allocated(), 0);
    });
    assert_eq!(info.num_allocations(), 2);
    assert_eq!(info.total_bytes_allocated(), 8);
    assert_eq!(info.current_bytes_allocated(), 0);
}

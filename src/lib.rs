/*!
This crate provides a method to measure memory allocations while running some code.

It can be used either exploratory (obtaining insights in how much memory allocations
are being made), or as a tool to assert desired allocation behaviour in tests.

# Usage
Add as a dependency - since including the trait replaces the global memory allocator,
you most likely want it gated behind a feature.

```toml
[features]
count-allocations = ["allocation-counter"]

[dependencies]
allocation-counter = { version = "0", optional = true }
```

The [measure()] function is now available, which can measure memory allocations made
when the supplied function or closure runs.

Tests can be conditional on the feature:

```
#[cfg(feature = "count-allocations")]
#[test]
{
    // [...]
}
```

The test code itself could look like:

```no_run
# fn code_that_should_not_allocate() {}
# fn code_that_should_allocate_a_little() {}
# fn external_code_that_should_not_be_tested() {}
// Verify that no memory allocations are made:
let info = allocation_counter::measure(|| {
    code_that_should_not_allocate();
});
assert_eq!(info.count_total, 0);

// Let's use a case where some allocations are expected.
let info = allocation_counter::measure(|| {
    code_that_should_allocate_a_little();
});

// Using a lower bound can help track behaviour over time:
assert!((500..600).contains(&info.count_total));
assert!((10_000..20_000).contains(&info.bytes_total));

// Limit peak memory usage:
assert!((100..200).contains(&info.count_max));
assert!((1_000..2_000).contains(&info.bytes_max));

// We don't want any leaks:
assert_eq!(0, info.count_current);
assert_eq!(0, info.bytes_current);

// It's possible to opt out of counting allocations
// for certain parts of the code flow:
let info = allocation_counter::measure(|| {
    code_that_should_not_allocate();
    allocation_counter::opt_out(|| {
        external_code_that_should_not_be_tested();
    });
    code_that_should_not_allocate();
});
assert_eq!(0, info.count_total);
```

Run the tests with the necessary feature enabled.

```sh
cargo test --features count-allocations
```
*/

pub(crate) mod allocator;

/// The allocation information obtained by a [measure()] call.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct AllocationInfo {
    /// The total number of allocations made during a [measure()] call.
    pub count_total: u64,

    /// The current (net result) number of allocations during a [measure()] call.
    ///
    /// A non-zero value of this field means that the function did not deallocate all allocations, as shown below.
    ///
    /// ```
    /// let info = allocation_counter::measure(|| {
    ///     let b = std::hint::black_box(Box::new(1_u32));
    ///     std::mem::forget(b);
    /// });
    /// assert_eq!(info.count_current, 1);
    /// ```
    pub count_current: i64,

    /// The max number of allocations held during a point in time during a [measure()] call.
    pub count_max: u64,

    /// The total amount of bytes allocated during a [measure()] call.
    pub bytes_total: u64,

    /// The current (net result) amount of bytes allocated during a [measure()] call.
    ///
    /// A non-zero value of this field means that not all memory was deallocated, as shown below.
    ///
    /// ```
    /// let info = allocation_counter::measure(|| {
    ///     let b = std::hint::black_box(Box::new(1_u32));
    ///     std::mem::forget(b);
    /// });
    /// assert_eq!(info.bytes_current, 4);
    /// ```
    pub bytes_current: i64,

    /// The max amount of bytes allocated at one time during a [measure()] call.
    pub bytes_max: u64,
}

impl std::ops::AddAssign for AllocationInfo {
    fn add_assign(&mut self, other: Self) {
        self.count_total += other.count_total;
        self.count_current += other.count_current;
        self.count_max += other.count_max;
        self.bytes_total += other.bytes_total;
        self.bytes_current += other.bytes_current;
        self.bytes_max += other.bytes_max;
    }
}

/// Run a closure or function while measuring the performed memory allocations.
///
/// Will only measure those allocations done by the current thread, so take care
/// when interpreting the returned count for multithreaded code.
///
/// Use [opt_out()] to opt of of counting allocations temporarily.
///
/// Nested `measure()` calls are supported up to a max depth of 64.
///
/// # Arguments
///
/// - `run_while_measuring` - The code to run while measuring allocations
///
/// # Examples
///
/// ```
/// # fn code_that_should_not_allocate_memory() {}
/// let actual = allocation_counter::measure(|| {
///      "hello, world".to_string();
/// });
/// let expected = allocation_counter::AllocationInfo {
///     count_total: 1,
///     count_current: 0,
///     count_max: 1,
///     bytes_total: 12,
///     bytes_current: 0,
///     bytes_max: 12,
/// };
/// assert_eq!(actual, expected);
/// ```
pub fn measure<F: FnOnce()>(run_while_measuring: F) -> AllocationInfo {
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

    run_while_measuring();

    allocator::ALLOCATIONS.with(|info_stack| {
        let mut info_stack = info_stack.borrow_mut();
        let depth = info_stack.depth;
        let popped = info_stack.elements[depth as usize];
        info_stack.depth -= 1;
        let depth = info_stack.depth as usize;
        info_stack.elements[depth] += popped;
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
/// let info = allocation_counter::measure(|| {
///     code_that_should_not_allocate();
///     allocation_counter::opt_out(|| {
///         external_code_that_should_not_be_tested();
///     });
///     code_that_should_not_allocate();
/// });
/// assert_eq!(info.count_total, 0);
/// ```
pub fn opt_out<F: FnOnce()>(run_while_not_counting: F) {
    allocator::DO_COUNT.with(|b| {
        *b.borrow_mut() += 1;
        run_while_not_counting();
        *b.borrow_mut() -= 1;
    });
}

#[test]
fn test_measure() {
    let info = measure(|| {
        // Do nothing.
    });
    assert_eq!(info.bytes_current, 0);
    assert_eq!(info.bytes_total, 0);
    assert_eq!(info.bytes_max, 0);
    assert_eq!(info.count_current, 0);
    assert_eq!(info.count_total, 0);
    assert_eq!(info.count_max, 0);

    let info = measure(|| {
        {
            let _a = std::hint::black_box(Box::new(1_u32));
        }
        {
            let _b = std::hint::black_box(Box::new(1_u32));
        }
    });
    assert_eq!(info.bytes_current, 0);
    assert_eq!(info.bytes_total, 8);
    assert_eq!(info.bytes_max, 4);
    assert_eq!(info.count_current, 0);
    assert_eq!(info.count_total, 2);
    assert_eq!(info.count_max, 1);

    let info = measure(|| {
        {
            let _a = std::hint::black_box(Box::new(1_u32));
        }
        let b = std::hint::black_box(Box::new(1_u32));
        std::mem::forget(b);
    });
    assert_eq!(info.bytes_current, 4);
    assert_eq!(info.bytes_total, 8);
    assert_eq!(info.bytes_max, 4);
    assert_eq!(info.count_current, 1);
    assert_eq!(info.count_total, 2);
    assert_eq!(info.count_max, 1);

    let info = measure(|| {
        let a = std::hint::black_box(Box::new(1_u32));
        let b = std::hint::black_box(Box::new(1_u32));
        let _c = std::hint::black_box(Box::new(*a + *b));
    });
    assert_eq!(info.bytes_current, 0);
    assert_eq!(info.bytes_total, 12);
    assert_eq!(info.bytes_max, 12);
    assert_eq!(info.count_current, 0);
    assert_eq!(info.count_total, 3);
    assert_eq!(info.count_max, 3);
}

#[test]
fn test_opt_out() {
    let allocations = measure(|| {
        // Do nothing.
    });
    assert_eq!(allocations.count_total, 0);

    let allocations = measure(|| {
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
        opt_out(|| {
            let v: Vec<u32> = vec![12];
            assert_eq!(v.len(), 1);
            opt_out(|| {
                let v: Vec<u32> = vec![12];
                assert_eq!(v.len(), 1);
            });
        });
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
        let v: Vec<u32> = vec![12];
        assert_eq!(v.len(), 1);
    });
    assert_eq!(allocations.count_total, 3);

    let info = measure(|| {
        opt_out(|| {
            let v: Vec<u32> = vec![12];
            assert_eq!(v.len(), 1);
        });
    });
    assert_eq!(0, info.count_total);
}

#[test]
fn test_nested_counting() {
    let info = measure(|| {
        let _a = std::hint::black_box(Box::new(1_u32));
        let info = measure(|| {
            let _b = std::hint::black_box(Box::new(1_u32));
        });
        assert_eq!(info.bytes_current, 0);
        assert_eq!(info.bytes_total, 4);
        assert_eq!(info.bytes_max, 4);
        assert_eq!(info.count_current, 0);
        assert_eq!(info.count_total, 1);
        assert_eq!(info.count_max, 1);
    });
    assert_eq!(info.bytes_current, 0);
    assert_eq!(info.bytes_total, 8);
    assert_eq!(info.bytes_max, 8);
    assert_eq!(info.count_current, 0);
    assert_eq!(info.count_total, 2);
    assert_eq!(info.count_max, 2);
}

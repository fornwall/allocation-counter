[![Crates.io](https://img.shields.io/crates/v/allocation-counter.svg)](https://crates.io/crates/allocation-counter)
[![Docs](https://docs.rs/allocation-counter/badge.svg)](https://docs.rs/allocation-counter/)
[![Build](https://github.com/fornwall/allocation-counter/workflows/CI/badge.svg)](https://github.com/fornwall/allocation-counter/actions?query=workflow%3A%22CI%22)

# allocation-counter
Rust library to run code while counting allocations. Can be used to assert that the desired amount of memory allocations is not exceeded in tests.

It works by replacing the System allocator with a custom one which increases a thread local counter on each memory allocation before delegating to the normal system allocator.

# Example
Add as a dependency - since including the trait replaces the global memory allocator, you most likely want it gated behind a feature:

```toml
[features]
count-allocations = ["allocation-counter"]

[dependencies]
allocation-counter = { version = "0", optional = true }
```

Tests can now be written to assert that the number of desired memory allocations are not exceeded:

```rust
#[cfg(feature = "count-allocations")]
#[test]
pub fn no_memory_allocations() {
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

Run the tests with the necessary feature enabled:

```sh
cargo test --features count-allocations
```

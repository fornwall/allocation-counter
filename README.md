[![Crates.io](https://img.shields.io/crates/v/allocation-counter.svg)](https://crates.io/crates/allocation-counter)
[![Docs](https://docs.rs/allocation-counter/badge.svg)](https://docs.rs/allocation-counter/)
[![Build](https://github.com/fornwall/allocation-counter/workflows/CI/badge.svg)](https://github.com/fornwall/allocation-counter/actions?query=workflow%3A%22CI%22)

# allocation-counter
Rust library to run code while counting allocations. Can be used to assert that the desired amount of memory allocations is not exceeded in tests.

It works by replacing the System allocator with a custom one which increases a thread local counter on each memory allocation before delegating to the normal system allocator.

See the below example and the [crate documentation](https://docs.rs/allocation-counter/latest/allocation_counter/) for more information.

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
}
```

Run the tests with the necessary feature enabled:

```sh
cargo test --features count-allocations
```

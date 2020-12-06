[![Crates.io](https://img.shields.io/crates/v/allocation-counter.svg)](https://crates.io/crates/allocation-counter)
[![Build](https://github.com/fornwall/allocation-counter/workflows/CI/badge.svg)](https://github.com/fornwall/allocation-counter/actions?query=workflow%3A%22CI%22)


# allocation-counter
Run some Rust code while counting allocations. Can be used to assert that the desired amount of memory allocations is not exceeded in tests.

It works by replacing the System allocator with a custom one which increases a thread local counter on each memory allocation before delegating to the normal system allocator.

# Example usage:
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
}
```

Run the tests with the necessary feature enabled:

```sh
cargo test --features count-allocations
```

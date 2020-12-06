[![Crates.io](https://img.shields.io/crates/v/allocation-counter.svg)](https://crates.io/crates/allocation-counter)
[![Build](https://github.com/fornwall/allocation-counter/workflows/CI/badge.svg)](https://github.com/fornwall/allocation-counter/actions?query=workflow%3A%22CI%22)


# allocation-counter
Run some Rust code while counting allocations.

# Example usage:
Add as a dependency:

```toml
[features]
count-allocations = ["allocation-counter"]

[dependencies]
allocation-counter = { version = "0", optional = true }
```

Since including the trait replaces the global memory allocator, you most likely want it gated behind a feature.

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

You can now assert that the function does not allocate memory by running tests with the necessary feature enabled:

```sh
cargo test --features count-allocations
```

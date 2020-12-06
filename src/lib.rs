// #![feature(test, const_fn)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;

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

/// Run a closure while counting the memory allocations done.
pub fn count<F: FnOnce()>(run_while_counting: F) -> u64 {
    ALLOCATIONS.with(|f| {
        *f.borrow_mut() = 0;
    });

    run_while_counting();

    ALLOCATIONS.with(|f| *f.borrow())
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator {};

#[test]
fn test_closure() {
    let allocations = count(|| {
        let mut v: Vec<u32> = Vec::new();
        v.push(12);
        assert_eq!(v.len(), 1);
    });
    assert_eq!(allocations, 1);

    let allocations = count(|| {
        let mut v: Vec<u32> = Vec::new();
        v.push(12);
        assert_eq!(v.len(), 1);
    });
    assert_eq!(allocations, 1);

    let allocations = count(|| {
        let mut v: Vec<u32> = Vec::new();
        v.push(12);
        assert_eq!(v.len(), 1);
        let mut v: Vec<u32> = Vec::new();
        v.push(12);
        assert_eq!(v.len(), 1);
    });
    assert_eq!(allocations, 2);
}

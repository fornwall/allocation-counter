use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;

/// Run a closure while counting the performed memory allocations.
///
/// Will only measure those done by the current thread, so take care when
/// interpreting the returned count for multithreaded programs.
///
/// Usage:
///
/// ```rust
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

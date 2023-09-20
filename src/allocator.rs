use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;

thread_local! {
    pub static ALLOCATIONS: RefCell<u64> = RefCell::new(0);
}
thread_local! {
    pub static DO_COUNT: RefCell<u32> = RefCell::new(0);
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

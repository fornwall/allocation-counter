use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;

pub const MAX_DEPTH: usize = 32;

pub struct AllocationInfoStack {
    pub depth: u32,
    pub elements: [crate::AllocationInfo; MAX_DEPTH],
}

thread_local! {
    pub static ALLOCATIONS: RefCell<AllocationInfoStack> = RefCell::new(AllocationInfoStack {
        depth: 0,
        elements: [crate::AllocationInfo::default(); MAX_DEPTH],
    });
}
thread_local! {
    pub static DO_COUNT: RefCell<u32> = RefCell::new(0);
}

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        DO_COUNT.with(|b| {
            if *b.borrow() == 0 {
                ALLOCATIONS.with(|info_stack| {
                    let mut info_stack = info_stack.borrow_mut();
                    let depth = info_stack.depth;
                    let info = &mut info_stack.elements[depth as usize];
                    info.num_allocations += 1;
                    info.total_bytes_allocated += l.size() as u64;
                    info.current_bytes_allocated += l.size() as i64;
                    if info.current_bytes_allocated > 0 {
                        info.max_bytes_allocated = info
                            .max_bytes_allocated
                            .max(info.current_bytes_allocated as u64);
                    }
                });
            }
        });

        System.alloc(l)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, l: Layout) {
        DO_COUNT.with(|b| {
            if *b.borrow() == 0 {
                ALLOCATIONS.with(|info_stack| {
                    let mut info_stack = info_stack.borrow_mut();
                    let depth = info_stack.depth;
                    let info = &mut info_stack.elements[depth as usize];
                    info.current_bytes_allocated -= l.size() as i64;
                });
            }
        });

        System.dealloc(ptr, l);
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator {};

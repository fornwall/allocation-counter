use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;

pub const MAX_DEPTH: usize = 64;

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

                    info.count_total += 1;
                    info.count_current += 1;
                    if info.count_current > 0 {
                        info.count_max = info.count_max.max(info.count_current as u64);
                    }
                    info.bytes_total += l.size() as u64;
                    info.bytes_current += l.size() as i64;
                    if info.bytes_current > 0 {
                        info.bytes_max = info.bytes_max.max(info.bytes_current as u64);
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
                    info.count_current -= 1;
                    info.bytes_current -= l.size() as i64;
                });
            }
        });

        System.dealloc(ptr, l);
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator {};

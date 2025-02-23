use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use super::hooks::{original_malloc, original_free};

struct MyAllocator;

unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        original_malloc(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        original_free(ptr as *mut std::ffi::c_void);
    }
}

// Create a static instance of the allocator
#[global_allocator]
static GLOBAL: MyAllocator = MyAllocator;

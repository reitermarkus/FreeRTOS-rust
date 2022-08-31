use crate::shim::*;
use core::alloc::{GlobalAlloc, Layout};

/**
Use with:

    #[global_allocator]
    static GLOBAL: FreeRtosAllocator = FreeRtosAllocator;
*/

pub struct FreeRtosAllocator;

unsafe impl GlobalAlloc for FreeRtosAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        pvPortMalloc(layout.size() as _).cast()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        vPortFree(ptr.cast())
    }
}

use crate::base::*;
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
        let res = pvPortMalloc(layout.size() as u32);
        return res as *mut u8;
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
      vPortFree(ptr as FreeRtosVoidPtr)
    }
}

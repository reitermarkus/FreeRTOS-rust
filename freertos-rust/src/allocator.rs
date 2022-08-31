use crate::shim::*;
use core::alloc::{GlobalAlloc, Layout};

/// # Usage
///
/// ```
/// use freertos_rust::FreeRtosAllocator;
///
/// #[global_allocator]
/// static ALLOC: FreeRtosAllocator = FreeRtosAllocator;
/// ```
pub struct FreeRtosAllocator;

unsafe impl GlobalAlloc for FreeRtosAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        pvPortMalloc(layout.size() as _).cast()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        vPortFree(ptr.cast())
    }
}

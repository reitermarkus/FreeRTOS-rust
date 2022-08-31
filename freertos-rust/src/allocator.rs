use core::alloc::{GlobalAlloc, Layout};

use crate::shim::*;

/// An allocator based on the FreeRTOS Memory Management API.
///
/// The actual implementation on which `heap_*` feature is enabled.
///
/// # Usage
///
/// ```
/// use freertos_rust::Allocator;
///
/// #[global_allocator]
/// static ALLOC: Allocator = Allocator;
/// ```
pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    pvPortMalloc(layout.size() as _).cast()
  }

  unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
    vPortFree(ptr.cast())
  }
}

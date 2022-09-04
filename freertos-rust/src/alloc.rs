use core::alloc::{GlobalAlloc, Layout};
use core::marker::PhantomPinned;

use crate::shim::{pvPortMalloc, vPortFree};

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

#[export_name = "vApplicationMallocFailedHook"]
extern "C" fn malloc_failed_hook() {
  panic!("`malloc` failed");
}

/// Marker type for dynamically allocated types.
#[non_exhaustive]
pub struct Dynamic {}

/// Marker type for statically allocated types.
#[non_exhaustive]
pub struct Static {
  _pinned: PhantomPinned,
}

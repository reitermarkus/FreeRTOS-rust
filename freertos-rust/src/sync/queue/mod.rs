use core::{
  marker::PhantomData,
  mem::{MaybeUninit, size_of, self},
  ops::Deref,
  ptr,
};

use crate::{
  ffi::{QueueHandle_t, UBaseType_t},
  shim::{vQueueDelete, StaticQueue_t},
};
#[cfg(freertos_feature = "dynamic_allocation")]
use crate::shim::xQueueCreate;
#[cfg(freertos_feature = "static_allocation")]
use crate::shim::xQueueCreateStatic;

mod handle;
pub use handle::QueueHandle;

/// A fixed-size queue. Items are copied and owned by the queue.
///
/// # Example
///
/// ```
/// extern crate alloc;
/// use alloc::sync::Arc;
/// use core::{time::Duration};
///
/// use freertos_rust::sync::Queue;
///
/// let queue = Queue::<u32, 8>::new();
/// queue.send(42, Duration::MAX);
///
/// assert_eq!(queue.receive(Duration::MAX), Ok(42));
/// ```
pub struct Queue<T, const SIZE: usize> {
  handle: QueueHandle_t,
  item_type: PhantomData<T>,
}

unsafe impl<T: Send, const SIZE: usize> Send for Queue<T, SIZE> {}
unsafe impl<T: Send, const SIZE: usize> Sync for Queue<T, SIZE> {}

#[cfg(freertos_feature = "dynamic_allocation")]
impl<T, const SIZE: usize> Queue<T, SIZE> {
    /// Create a new dynamic queue.
    #[allow(clippy::new-without-default)]
    pub fn new() -> Self {
      let ptr = unsafe {
        xQueueCreate(
          (mem::size_of::<T>() * SIZE) as UBaseType_t,
          size_of::<T>() as UBaseType_t,
        )
      };
      assert!(!ptr.is_null());

      Self {
        handle: ptr,
        item_type: PhantomData,
      }
    }
}

impl<T, const SIZE: usize> Deref for Queue<T, SIZE> {
  type Target = QueueHandle<T>;

  fn deref(&self) -> &Self::Target {
    unsafe { QueueHandle::<T>::from_ptr(self.handle) }
  }
}

impl<T, const SIZE: usize> Drop for Queue<T, SIZE> {
  fn drop(&mut self) {
    unsafe { vQueueDelete(self.handle) }
  }
}

/// A statically allocated fixed-size queue. Items are copied and owned by the queue.
///
/// # Examples
///
/// ```
/// use core::{mem::MaybeUninit, time::Duration};
///
/// use freertos_rust::sync::StaticQueue;
///
/// let queue = StaticQueue::new(unsafe {
///   static mut QUEUE: MaybeUninit<StaticQueue<u32, 8>> = MaybeUninit::uninit();
///   &mut QUEUE
/// });
/// queue.send(42, Duration::MAX);
///
/// assert_eq!(queue.receive(Duration::MAX), Ok(42));
/// ```
pub struct StaticQueue<T, const SIZE: usize> {
  data: StaticQueue_t,
  items: [MaybeUninit<T>; SIZE],
}

unsafe impl<T: Send, const SIZE: usize> Send for StaticQueue<T, SIZE> {}
unsafe impl<T: Send, const SIZE: usize> Sync for StaticQueue<T, SIZE> {}

#[cfg(freertos_feature = "static_allocation")]
impl<T, const SIZE: usize> StaticQueue<T, SIZE> {
  /// Create a new static queue.
  pub fn new(queue: &'static mut MaybeUninit<Self>) -> &'static Self {
    let queue_ptr = queue.as_mut_ptr();

    unsafe {
      let ptr = xQueueCreateStatic(
        SIZE as UBaseType_t,
        size_of::<T>() as UBaseType_t,
        ptr::addr_of_mut!((*queue_ptr).items).cast(),
        ptr::addr_of_mut!((*queue_ptr).data),
      );
      debug_assert!(!ptr.is_null());
      debug_assert_eq!(ptr, ptr::addr_of!((*queue_ptr).data) as QueueHandle_t);
      queue.assume_init_ref()
    }
  }
}

impl<T, const SIZE: usize> Deref for StaticQueue<T, SIZE> {
  type Target = QueueHandle<T>;

  fn deref(&self) -> &Self::Target {
    unsafe { QueueHandle::<T>::from_ptr(ptr::addr_of!(self.data) as QueueHandle_t) }
  }
}

impl<T, const SIZE: usize> Drop for StaticQueue<T, SIZE> {
  fn drop(&mut self) {
    unsafe { vQueueDelete(self.as_ptr()) }
  }
}

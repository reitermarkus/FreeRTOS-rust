use core::fmt;
use core::ffi::CStr;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr;

use crate::FreeRtosError;
use crate::InterruptContext;
use crate::ffi::Pointee;
use crate::ffi::QueueHandle_t;
use crate::shim::{
  pdTRUE,
  errQUEUE_FULL,
  vQueueAddToRegistry,
  xQueueSend,
  xQueueSendFromISR,
  xQueueReceive,
  uxQueueMessagesWaiting,
};
use crate::Ticks;

/// A handle for managing a queue.
///
/// See [`Queue`](crate::sync::Queue) for the preferred owned version.
///
/// This type is compatible with a raw FreeRTOS [`QueueHandle_t`].
#[repr(transparent)]
pub struct QueueHandle<T> {
  handle: Pointee<QueueHandle_t>,
  item_type: PhantomData<T>,
}

impl<T> fmt::Debug for QueueHandle<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.as_ptr().fmt(f)
  }
}

impl<T> QueueHandle<T> {
  /// Create a `QueueHandle` from a raw handle.
  ///
  /// # Safety
  ///
  /// - `ptr` must point to a valid queue.
  /// - The queue must not be deleted for the lifetime `'a` of the returned `QueueHandle`.
  #[inline]
  pub const unsafe fn from_ptr<'a>(ptr: QueueHandle_t) -> &'a Self {
    debug_assert!(!ptr.is_null());
    &*ptr.cast()
  }

  /// Get the raw queue handle.
  #[inline]
  pub const fn as_ptr(&self) -> QueueHandle_t {
    ptr::addr_of!(self.handle).cast_mut()
  }

  /// Assign a name to the queue and add it to the registry.
  #[inline]
  pub fn add_to_registry(&self, name: &'static CStr) {
    unsafe { vQueueAddToRegistry(self.as_ptr(), name.as_ptr()) }
  }

  /// Get the number of messages in the queue.
  #[inline]
  #[allow(clippy::len_without_is_empty)]
  pub fn len(&self) -> usize {
    unsafe { uxQueueMessagesWaiting(self.as_ptr()) as usize }
  }
}

impl<T: Sized + Send> QueueHandle<T> {
  /// Send an item to the end of the queue. Wait for the queue to have empty space for it.
  #[inline]
  pub fn send(&self, item: T, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
    unsafe {
      let mut item = MaybeUninit::new(item);
      let res = xQueueSend(self.as_ptr(), item.as_ptr().cast(), timeout.into().into());
      match res {
        pdTRUE => Ok(()),
        errQUEUE_FULL => {
          item.assume_init_drop();
          Err(FreeRtosError::QueueFull)
        },
        _ => unreachable!(),
      }
    }
  }

  /// Send an item to the end of the queue, from an interrupt.
  #[inline]
  pub fn send_from_isr(
      &self,
      ic: &InterruptContext,
      item: T,
  ) -> Result<(), FreeRtosError> {

    unsafe {
      let mut item: MaybeUninit<T> = MaybeUninit::new(item);
      let res = xQueueSendFromISR(self.as_ptr(), item.as_ptr().cast(), ic.as_ptr());

      match res {
        pdTRUE => Ok(()),
        errQUEUE_FULL => {
          item.assume_init_drop();
          Err(FreeRtosError::QueueFull)
        },
        _ => unreachable!(),
      }
    }
  }

  /// Wait for an item to be available on the queue.
  #[inline]
  pub fn receive(&self, timeout: impl Into<Ticks>) -> Result<T, FreeRtosError> {
    let mut item = MaybeUninit::<T>::zeroed();

    unsafe {
      let res = xQueueReceive(self.as_ptr(), item.as_mut_ptr().cast(), timeout.into().into());
      match res {
        pdTRUE => Ok(item.assume_init()),
        _ => Err(FreeRtosError::Timeout),
      }
    }
  }
}

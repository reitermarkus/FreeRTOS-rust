use core::fmt;
use core::ffi::CStr;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr;

use crate::FreeRtosError;
use crate::InterruptContext;
use crate::shim::QueueHandle_t;
use crate::lazy_init::PtrType;
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
/// See [`Queue`](crate::Queue) for the preferred owned version.
#[repr(transparent)]
pub struct QueueHandle<T>(<QueueHandle_t as PtrType>::Type, PhantomData<T>);

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
  pub const unsafe fn from_ptr<'a>(ptr: QueueHandle_t) -> &'a Self {
    &*ptr.cast::<Self>()
  }

  /// Get the raw queue handle.
  pub const fn as_ptr(&self) -> QueueHandle_t {
    ptr::addr_of!(self.0).cast_mut()
  }

  /// Assign a name to the queue and add it to the registry.
  pub fn add_to_registry(&self, name: &'static CStr) {
    unsafe { vQueueAddToRegistry(self.as_ptr(), name.as_ptr()) }
  }

  /// Send an item to the end of the queue. Wait for the queue to have empty space for it.
  #[inline]
  pub fn send(&self, item: T, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
    let res = unsafe {
      xQueueSend(self.as_ptr(), ptr::addr_of!(item).cast(), timeout.into().into())
    };

    match res {
      pdTRUE => Ok(()),
      errQUEUE_FULL => Err(FreeRtosError::QueueFull),
      _ => Err(FreeRtosError::Timeout),
    }
  }

  /// Send an item to the end of the queue, from an interrupt.
  #[inline]
  pub fn send_from_isr(
      &self,
      ic: &mut InterruptContext,
      item: T,
  ) -> Result<(), FreeRtosError> {
    let res = unsafe {
      xQueueSendFromISR(self.as_ptr(), ptr::addr_of!(item).cast(), ic.as_ptr())
    };

    match res {
      pdTRUE => Ok(()),
      errQUEUE_FULL => Err(FreeRtosError::QueueFull),
      _ => unreachable!(),
    }
  }

  /// Wait for an item to be available on the queue.
  #[inline]
  pub fn receive(&self, timeout: impl Into<Ticks>) -> Result<T, FreeRtosError> {
    let mut item = MaybeUninit::<T>::zeroed();

    let res = unsafe {
      xQueueReceive(self.as_ptr(), item.as_mut_ptr().cast(), timeout.into().into())
    };

    match res {
      pdTRUE => Ok(unsafe { item.assume_init() }),
      _ => Err(FreeRtosError::Timeout),
    }
  }

  /// Get the number of messages in the queue.
  pub fn len(&self) -> u32 {
    unsafe { uxQueueMessagesWaiting(self.as_ptr()) }
  }
}

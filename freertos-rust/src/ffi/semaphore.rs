use core::fmt;

use crate::FreeRtosError;
use crate::InterruptContext;
use crate::Ticks;
use crate::lazy_init::PtrType;
use crate::shim::errQUEUE_FULL;
use crate::shim::pdFALSE;
use crate::shim::pdTRUE;
use crate::shim::xSemaphoreGive;
use crate::shim::xSemaphoreGiveFromISR;
use crate::shim::xSemaphoreTake;
use crate::shim::xSemaphoreTakeFromISR;

pub use crate::shim::SemaphoreHandle_t;

/// A handle for low-level management of a semaphore.
///
/// See [`Semaphore`](crate::Semaphore) for the preferred owned version.
#[repr(transparent)]
pub struct SemaphoreHandle(<SemaphoreHandle_t as PtrType>::Type);

impl fmt::Debug for SemaphoreHandle {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.as_ptr().fmt(f)
  }
}

impl SemaphoreHandle {
  /// Create a a `SemaphoreHandle` from a raw handle.
  ///
  /// # Safety
  ///
  /// - `ptr` must point to a valid semaphore.
  /// - The semaphore must not be deleted for the lifetime `'a` of the returned `SemaphoreHandle`.
  pub const unsafe fn from_ptr<'a>(ptr: SemaphoreHandle_t) -> &'a Self {
    &*ptr.cast::<Self>()
  }

  /// Get the raw semaphore handle.
  pub const fn as_ptr(&self) -> SemaphoreHandle_t {
    self as *const _ as SemaphoreHandle_t
  }

  /// Increment the semaphore.
  #[inline]
  pub fn give(&self) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreGive(self.as_ptr()) } {
      pdTRUE => Ok(()),
      errQUEUE_FULL => Err(FreeRtosError::QueueFull),
      _ => unreachable!(),
    }
  }

  /// Increment the semaphore from within an interrupt service routine.
  #[inline]
  pub fn give_from_isr(&self, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreGiveFromISR(self.as_ptr(), ic.as_ptr()) } {
      pdTRUE => Ok(()),
      errQUEUE_FULL => Err(FreeRtosError::QueueFull),
      _ => unreachable!(),
    }
  }

  /// Decrement the semaphore.
  #[inline]
  pub fn take(&self, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreTake(self.as_ptr(), timeout.into().as_ticks()) } {
      pdTRUE => Ok(()),
      pdFALSE => Err(FreeRtosError::Timeout),
      _ => unreachable!(),
    }
  }

  /// Decrement the semaphore from within an interrupt service routine.
  #[inline]
  pub fn take_from_isr(&self, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreTakeFromISR(self.as_ptr(), ic.as_ptr()) } {
      pdTRUE => Ok(()),
      pdFALSE => Err(FreeRtosError::Unavailable),
      _ => unreachable!(),
    }
  }
}

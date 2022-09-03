use core::fmt;

use crate::{
  FreeRtosError,
  InterruptContext,
  Ticks,
  lazy_init::PtrType,
  shim::{
    errQUEUE_FULL,
    pdFALSE,
    pdTRUE,
    SemaphoreHandle_t,
    xSemaphoreGive,
    xSemaphoreGiveFromISR,
    xSemaphoreGiveRecursive,
    xSemaphoreTake,
    xSemaphoreTakeRecursive,
    xSemaphoreTakeFromISR,
  },
};

use super::SemaphoreGuard;

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

  /// Unlock the mutex recursively.
  #[inline]
  pub(crate) fn give_recursive(&self) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreGiveRecursive(self.as_ptr()) } {
      pdTRUE => Ok(()),
      pdFALSE => Err(FreeRtosError::QueueFull),
      _ => unreachable!(),
    }
  }

  /// Increment the semaphore or unlock the mutex from within an interrupt service routine.
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

  /// Lock the mutex recursively.
  #[inline]
  pub(crate) fn take_recursive(&self, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreTakeRecursive(self.as_ptr(), timeout.into().as_ticks()) } {
      pdTRUE => Ok(()),
      pdFALSE => Err(FreeRtosError::Timeout),
      _ => unreachable!(),
    }
  }

  /// Decrement the semaphore or lock the mutex from within an interrupt service routine.
  #[inline]
  pub fn take_from_isr(&self, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreTakeFromISR(self.as_ptr(), ic.as_ptr()) } {
      pdTRUE => Ok(()),
      pdFALSE => Err(FreeRtosError::Unavailable),
      _ => unreachable!(),
    }
  }

  /// Lock this semaphore in RAII fashion.
  pub fn lock(&self, timeout: impl Into<Ticks>) -> Result<SemaphoreGuard<'_>, FreeRtosError> {
    self.take(timeout)?;

    Ok(SemaphoreGuard { handle: self })
  }
}

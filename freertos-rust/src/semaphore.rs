use core::ptr::NonNull;

use crate::base::*;
use crate::shim::*;
use crate::units::*;

/// A counting or binary semaphore
pub struct Semaphore {
    handle: NonNull<CVoid>,
}

unsafe impl Send for Semaphore {}
unsafe impl Sync for Semaphore {}

impl Semaphore {
    /// Create a new binary semaphore
    pub fn new_binary() -> Result<Semaphore, FreeRtosError> {
        let handle = unsafe { freertos_rs_create_binary_semaphore() };

        match NonNull::new(handle) {
          Some(handle) => {
            let sem = Semaphore { handle };

            // Binary semaphores are locked by default, which is incompatible with
            // the `SemaphoreGuard` API, so unlock it.
            sem.give()?;

            Ok(sem)
          },
          None => Err(FreeRtosError::OutOfMemory),
        }
    }

    /// Create a new counting semaphore
    pub fn new_counting(max: u32, initial: u32) -> Result<Semaphore, FreeRtosError> {
        let handle = unsafe { freertos_rs_create_counting_semaphore(max, initial) };
        match NonNull::new(handle) {
          Some(handle) => Ok(Semaphore { handle }),
          None => Err(FreeRtosError::OutOfMemory),
        }
    }

    #[inline]
    pub unsafe fn from_raw_handle(handle: FreeRtosSemaphoreHandle) -> Self {
        Self { handle: NonNull::new_unchecked(handle) }
    }

    pub fn as_raw_handle(&self) -> FreeRtosSemaphoreHandle {
        self.handle.as_ptr()
    }

    fn give(&self) -> Result<(), FreeRtosError> {
      let res = unsafe { freertos_rs_give_mutex(self.handle.as_ptr()) };

      if res != 0 {
        return Err(FreeRtosError::MutexTimeout);
      }

      Ok(())
    }

    pub unsafe fn take<D: DurationTicks>(&self, max_wait: D) -> Result<(), FreeRtosError> {
      unsafe {
        let res = freertos_rs_take_mutex(self.handle.as_ptr(), max_wait.to_ticks());

        if res != 0 {
            return Err(FreeRtosError::Timeout);
        }

        Ok(())
      }
    }

    /// Lock this semaphore in a RAII fashion
    pub fn lock<D: DurationTicks>(&self, max_wait: D) -> Result<SemaphoreGuard, FreeRtosError> {
        unsafe { self.take(max_wait)? }
        Ok(SemaphoreGuard { handle: self.handle })
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            freertos_rs_delete_semaphore(self.handle.as_ptr());
        }
    }
}

/// Holds the lock to the semaphore until we are dropped
#[derive(Debug)]
pub struct SemaphoreGuard {
    handle: NonNull<CVoid>,
}

impl Drop for SemaphoreGuard {
    fn drop(&mut self) {
        unsafe { freertos_rs_give_mutex(self.handle.as_ptr()) };
    }
}

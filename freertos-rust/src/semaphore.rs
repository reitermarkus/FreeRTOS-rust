use core::mem::MaybeUninit;
use core::{ptr::NonNull, mem};

use crate::base::*;
use crate::shim::*;
use crate::units::*;

pub type StaticSemaphore = StaticSemaphore_t;

#[macro_export]
macro_rules! static_semaphore {
    () => {{
      static mut STATIC_SEMAPHORE: ::core::mem::MaybeUninit<$crate::StaticSemaphore> = ::core::mem::MaybeUninit::uninit();
      static mut SEMAPHORE: ::core::mem::MaybeUninit::<Semaphore> = ::core::mem::MaybeUninit::uninit();
      unsafe {
        $crate::Semaphore::new_binary_static(&mut STATIC_SEMAPHORE, &mut SEMAPHORE)
      }
    }};
    ($max:expr, $initial:expr) => {
      static mut STATIC_SEMAPHORE: ::core::mem::MaybeUninit<$crate::StaticSemaphore> = ::core::mem::MaybeUninit::uninit();
      static mut SEMAPHORE: ::core::mem::MaybeUninit::<Semaphore> = ::core::mem::MaybeUninit::uninit();
      unsafe {
        $crate::Semaphore::new_couting_static($max, $initial, &mut STATIC_SEMAPHORE, &mut SEMAPHORE)
      }
    };
}

/// A counting or binary semaphore
pub struct Semaphore {
    handle: NonNull<CVoid>,
}

unsafe impl Send for Semaphore {}
unsafe impl Sync for Semaphore {}

impl Semaphore {
    pub fn new_binary_static(
      static_semaphore: &'static mut MaybeUninit<StaticSemaphore>,
      semaphore: &'static mut MaybeUninit<Self>,
    ) -> &'static mut Self {
      unsafe {
      let handle = freertos_rs_create_binary_semaphore_static(
        static_semaphore.as_mut_ptr(),
      );

        semaphore.write(Self::from_raw_handle(handle))
      }
    }

    /// Create a new binary semaphore
    pub fn new_binary() -> Result<Semaphore, FreeRtosError> {
        let handle = unsafe { freertos_rs_create_binary_semaphore() };

        match NonNull::new(handle) {
          Some(handle) => {
            let sem = Semaphore { handle };
            Ok(sem)
          },
          None => Err(FreeRtosError::OutOfMemory),
        }
    }

    pub fn new_counting_static(
      max: u32,
      initial: u32,
      static_semaphore: &'static mut MaybeUninit<StaticSemaphore>,
      semaphore: &'static mut MaybeUninit<Self>,
    ) -> &'static mut Self {
      unsafe {
        assert!(initial <= max);

        let handle = freertos_rs_create_counting_semaphore_static(
          max, initial,
          static_semaphore.as_mut_ptr(),
        );

          semaphore.write(Self::from_raw_handle(handle))
        }
    }

    /// Create a new counting semaphore
    pub fn new_counting(max: u32, initial: u32) -> Result<Semaphore, FreeRtosError> {
      assert!(initial <= max);

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

    #[inline]
    pub fn as_raw_handle(&self) -> FreeRtosSemaphoreHandle {
        self.handle.as_ptr()
    }

    #[inline]
    pub fn give(&self) -> Result<(), FreeRtosError> {
      unsafe { semaphore_give(self.handle) }
    }

    #[inline]
    pub fn take<D: DurationTicks>(&self, max_wait: D) -> Result<(), FreeRtosError> {
      unsafe { semaphore_take(self.handle, max_wait) }
    }

    /// Lock this semaphore in a RAII fashion
    pub fn lock<D: DurationTicks>(&self, max_wait: D) -> Result<SemaphoreGuard, FreeRtosError> {
        self.take(max_wait)?;

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
        let _ = unsafe { semaphore_give(self.handle) };
    }
}

unsafe fn semaphore_give(handle: NonNull<CVoid>) -> Result<(), FreeRtosError> {
  let res = freertos_rs_give_mutex(handle.as_ptr());

      if res != 0 {
        return Err(FreeRtosError::MutexTimeout);
      }

      Ok(())
}

unsafe fn semaphore_take<D: DurationTicks>(handle: NonNull<CVoid>, max_wait: D) -> Result<(), FreeRtosError> {
    let res = freertos_rs_take_mutex(handle.as_ptr(), max_wait.to_ticks());

    if res != 0 {
        return Err(FreeRtosError::Timeout);
    }

    Ok(())
}

use core::fmt;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

use crate::shim::portMAX_DELAY;
use crate::FreeRtosError;
use crate::sync::SemaphoreHandle;
use crate::shim::{
  SemaphoreHandle_t,
};
use crate::Ticks;

use super::{
  MutexGuard,
  RecursiveMutexGuard,
};

macro_rules! impl_mutex_handle {
  ($handle:ident, $guard:ident, $take:ident, $give:ident $(,)?) => {
    /// A handle for low-level management of a semaphore.
    ///
    /// See [`Semaphore`](crate::Semaphore) for the preferred owned version.
    ///
    /// This type is compatible with a raw FreeRTOS mutex if `T` is zero-sized.
    pub struct $handle<T: ?Sized = ()> {
      // TODO: Assert, same layout as `AtomicPtr<<SemaphoreHandle_t as PtrType>::Type>`.
      ptr: SemaphoreHandle_t,
      data: UnsafeCell<T>,
    }

    impl<T: ?Sized> fmt::Debug for $handle<T> {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ptr().fmt(f)
      }
    }

    impl $handle {
      #[doc = concat!("Create a a `", stringify!($handle), "` from a raw handle.")]
      ///
      /// # Safety
      ///
      /// - `ptr` must point to a valid queue.
      /// - `T` must be zero-sized.
      #[doc = concat!("- The mutex must not be deleted for the lifetime of the returned `" , stringify!($handle), "`.")]
      pub const unsafe fn from_ptr(ptr: SemaphoreHandle_t) -> Self {
        Self {
          ptr,
          data: UnsafeCell::new(()),
        }
      }
    }

    impl<T: ?Sized> $handle<T> {
      /// Get the raw queue handle.
      pub const fn as_ptr(&self) -> SemaphoreHandle_t {
        self.ptr
      }

      /// Get a reference to the contained.
      ///
      /// # Safety
      ///
      /// - The mutex must be locked.
      pub(super) const unsafe fn data(&self) -> &T {
        &*self.data.get()
      }

      /// Get a mutable reference to the contained.
      ///
      /// # Safety
      ///
      /// - The mutex must be locked non-recursively.
      #[allow(unused)]
      pub(super) const unsafe fn data_mut(&self) -> &mut T {
        &mut *self.data.get()
      }

      #[inline]
      const fn handle(&self) -> &SemaphoreHandle {
        unsafe { SemaphoreHandle::from_ptr(self.as_ptr()) }
      }

      #[inline]
      fn take(&self, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
        self.handle().$take(timeout)
      }

      #[inline]
      pub(super) fn give(&self) -> Result<(), FreeRtosError> {
        self.handle().$give()
      }
    }

    impl<T: ?Sized> $handle<T> {
      /// Lock the pinned mutex.
      #[inline]
      pub fn lock(&self) -> Result<$guard<'_, T>, FreeRtosError> {
        self.timed_lock(Ticks::new(portMAX_DELAY))
      }

      /// Try locking the pinned mutex and return immediately.
      #[inline(always)]
      pub fn try_lock(&self) -> Result<$guard<'_, T>, FreeRtosError> {
        self.timed_lock(Ticks::new(0))
      }

      /// Try locking the pinned mutex until the given `timeout`.
      pub fn timed_lock(&self, timeout: impl Into<Ticks>) -> Result<$guard<'_, T>, FreeRtosError> {
        self.take(timeout)?;
        Ok($guard { handle: self })
      }
    }
  };
}


impl_mutex_handle!(
  MutexHandle,
  MutexGuard,
  take,
  give,
);
impl_mutex_handle!(
  RecursiveMutexHandle,
  RecursiveMutexGuard,
  take_recursive,
  give_recursive,
);

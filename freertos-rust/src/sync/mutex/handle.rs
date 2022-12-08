use core::fmt;
use core::cell::UnsafeCell;

use crate::{
  shim::portMAX_DELAY,
  FreeRtosError,
  sync::SemaphoreHandle,
  ffi::SemaphoreHandle_t,
  Ticks,
  InterruptContext,
};

use super::{
  MutexGuard,
  IsrMutexGuard,
  RecursiveMutexGuard,
};

macro_rules! impl_mutex_handle {
  (
    $mutex:ident,
    $handle:ident,
    $guard:ident,
    $take:ident,
    $(($take_from_isr:ident),)?
    $give:ident $(,)?
    $(($give_from_isr:ident),)?
  ) => {
    /// A handle for managing a mutex.
    ///
    #[doc = concat!("See [`", stringify!($mutex), "`](crate::sync::", stringify!($mutex), ") for the preferred owned version.")]
    ///
    #[doc = concat!("`", stringify!($handle), "<()>` is compatible with a raw FreeRTOS [`SemaphoreHandle_t`].")]
    pub struct $handle<T: ?Sized = ()> {
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
      #[inline]
      pub const unsafe fn from_ptr(ptr: SemaphoreHandle_t) -> Self {
        Self {
          ptr,
          data: UnsafeCell::new(()),
        }
      }
    }

    impl<T: ?Sized> $handle<T> {
      /// Get the raw queue handle.
      #[inline]
      pub const fn as_ptr(&self) -> SemaphoreHandle_t {
        self.ptr
      }

      /// Get a reference to the contained.
      ///
      /// # Safety
      ///
      /// - The mutex must be locked.
      #[inline]
      pub(super) const unsafe fn data(&self) -> &T {
        &*self.data.get()
      }

      /// Get a mutable reference to the contained.
      ///
      /// # Safety
      ///
      /// - The mutex must be locked non-recursively.
      #[allow(unused)]
      #[inline]
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

      $(
        /// Unlock the mutex from within an interrupt service routine.
        #[inline]
        pub(super) fn give_from_isr(&self, ic: &InterruptContext) -> Result<(), FreeRtosError> {
          self.handle().$give_from_isr(ic)
        }
      )*

      $(
        /// Lock the mutex from within an interrupt service routine.
        #[inline]
        pub fn lock_from_isr<'ic>(&self, ic: &'ic InterruptContext) -> Result<IsrMutexGuard<'ic, '_, T>, FreeRtosError> {
          self.handle().$take_from_isr(ic)?;
          Ok(IsrMutexGuard { ic, handle: self })
        }
      )*

      /// Lock the mutex.
      #[inline]
      pub fn lock(&self) -> Result<$guard<'_, T>, FreeRtosError> {
        self.timed_lock(Ticks::new(portMAX_DELAY))
      }

      /// Try locking the mutex and return immediately.
      #[inline]
      pub fn try_lock(&self) -> Result<$guard<'_, T>, FreeRtosError> {
        self.timed_lock(Ticks::new(0))
      }

      /// Try locking the mutex until the given `timeout`.
      pub fn timed_lock(&self, timeout: impl Into<Ticks>) -> Result<$guard<'_, T>, FreeRtosError> {
        self.take(timeout)?;
        Ok($guard { handle: self })
      }
    }
  };
}

impl_mutex_handle!(
  Mutex,
  MutexHandle,
  MutexGuard,
  take,
  (take_from_isr),
  give,
  (give_from_isr),
);
impl_mutex_handle!(
  RecursiveMutex,
  RecursiveMutexHandle,
  RecursiveMutexGuard,
  take_recursive,
  give_recursive,
);

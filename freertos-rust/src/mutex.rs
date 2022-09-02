use core::cell::UnsafeCell;
use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::time::Duration;
use core::pin::Pin;

use crate::alloc::{Dynamic, Static};
use crate::error::FreeRtosError;
use crate::lazy_init::{LazyInit, LazyPtr};
use crate::shim::*;
use crate::ticks::*;

macro_rules! guard_impl_deref_mut {
  (MutexGuard) => {
    impl<T: ?Sized, A> DerefMut for MutexGuard<'_, T, A>
    where
      (Mutex<()>, A): LazyInit<SemaphoreHandle_t>,
    {
      /// Mutably dereferences the locked value.
      fn deref_mut(&mut self) -> &mut T {
          unsafe { &mut *self.lock.data.get() }
      }
    }
  };
  ($guard:ident) => {};
}

macro_rules! guard_deref_mut_doc {
  (MutexGuard) => { " and [`DerefMut`]" };
  ($guard:ident) => { "" };
}

macro_rules! impl_mutex {
  (
    $(#[$attr:meta])*
    $mutex:ident,
    $guard:ident,
    $create:ident,
    $create_static:ident,
    $take:ident,
    $give:ident,
    $variant_name:expr,
  ) => {
    $(#[$attr])*
    pub struct $mutex<T: ?Sized, A = Dynamic>
    where
      ($mutex<()>, A): LazyInit<SemaphoreHandle_t>,
    {
      handle: LazyPtr<($mutex<()>, A), SemaphoreHandle_t>,
      _alloc_type: PhantomData<A>,
      data: UnsafeCell<T>,
    }

    impl LazyInit<SemaphoreHandle_t> for ($mutex<()>, Dynamic) {
      fn init(_data: &UnsafeCell<MaybeUninit<Self::Data>>) -> Self::Ptr {
        unsafe {
          let ptr = $create();
          assert!(!ptr.is_null());
          Self::Ptr::new_unchecked(ptr)
        }
      }

      #[inline]
      fn destroy(ptr: Self::Ptr) {
        unsafe { vSemaphoreDelete(ptr.as_ptr()) }
      }
    }

    impl LazyInit<SemaphoreHandle_t> for ($mutex<()>, Static) {
      type Data = StaticQueue_t;

      fn init(data: &UnsafeCell<MaybeUninit<Self::Data>>) -> Self::Ptr {
        unsafe {
          let data = &mut *data.get();
          let ptr = $create_static(data.as_mut_ptr());
          assert!(!ptr.is_null());
          Self::Ptr::new_unchecked(ptr)
        }
      }

      fn cancel_init_supported() -> bool {
        false
      }

      #[inline]
      fn destroy(ptr: Self::Ptr) {
        drop(ptr)
      }
    }

    unsafe impl<T: ?Sized + Send, A> Send for $mutex<T, A>
    where
      ($mutex<()>, A): LazyInit<SemaphoreHandle_t>,
    {}
    unsafe impl<T: ?Sized + Send, A> Sync for $mutex<T, A>
    where
      ($mutex<()>, A): LazyInit<SemaphoreHandle_t>,
    {}

    impl<T, A> $mutex<T, A>
    where
      ($mutex<()>, A): LazyInit<SemaphoreHandle_t>,
    {
      #[doc = concat!("Create a new `", stringify!($mutex), "` with the given inner value.")]
      pub const fn new(t: T) -> Self {
        Self {
          handle: LazyPtr::new(),
          _alloc_type: PhantomData,
          data: UnsafeCell::new(t),
        }
      }
    }

    impl<T, A> $mutex<T, A>
    where
      ($mutex<()>, A): LazyInit<SemaphoreHandle_t>,
    {
      /// Consume the mutex and return its inner value.
      pub fn into_inner(self) -> T {
        self.data.into_inner()
      }
    }

    impl<T: ?Sized> $mutex<T, Dynamic> {
      /// Lock the mutex.
      #[inline]
      pub fn lock(&self) -> Result<$guard<'_, T, Dynamic>, FreeRtosError> {
        self.timed_lock(Duration::MAX)
      }

      /// Try locking the mutex and return immediately.
      #[inline]
      pub fn try_lock(&self) -> Result<$guard<'_, T, Dynamic>, FreeRtosError> {
        self.timed_lock(Duration::ZERO)
      }

      /// Try locking the mutex until the given `timeout`.
      pub fn timed_lock(&self, timeout: impl Into<Ticks>) -> Result<$guard<'_, T, Dynamic>, FreeRtosError> {
        let timeout = timeout.into();

        let res = unsafe {
          $take(self.handle.as_ptr(), timeout.as_ticks())
        };

        if res == pdTRUE {
          return Ok($guard { lock: self, _not_send_and_sync: PhantomData })
        }

        Err(FreeRtosError::Timeout)
      }
    }

    impl<T: ?Sized> $mutex<T, Static> {
      /// Lock the pinned mutex.
      #[inline]
      pub fn lock(self: Pin<&Self>) -> Result<$guard<'_, T, Static>, FreeRtosError> {
        self.timed_lock(Duration::MAX)
      }

      /// Try locking the pinned mutex and return immediately.
      #[inline]
      pub fn try_lock(self: Pin<&Self>) -> Result<$guard<'_, T, Static>, FreeRtosError> {
        self.timed_lock(Duration::ZERO)
      }

      /// Try locking the pinned mutex until the given `timeout`.
      pub fn timed_lock(self: Pin<&Self>, timeout: impl Into<Ticks>) -> Result<$guard<'_, T, Static>, FreeRtosError> {
        let timeout = timeout.into();

        let res = unsafe {
          $take(self.handle.as_ptr(), timeout.as_ticks())
        };

        if res == pdTRUE {
          return Ok($guard { lock: self.get_ref(), _not_send_and_sync: PhantomData })
        }

        Err(FreeRtosError::Timeout)
      }
    }

    impl<T: ?Sized + fmt::Debug> fmt::Debug for $mutex<T> {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = f.debug_struct(stringify!($mutex));

        d.field("handle", &self.handle.as_ptr());

        match self.try_lock() {
          Ok(guard) => d.field("data", &&*guard),
          Err(_) => d.field("data", &format_args!("<locked>")),
        };

        d.finish()
      }
    }

    /// An RAII implementation of a “scoped lock” of a
    #[doc = concat!($variant_name, " mutex.")]
    ///  When this structure is
    /// dropped (falls out of scope), the lock will be unlocked.
    ///
    /// The data protected by the mutex can be accessed through this guard via its [`Deref`]
    #[doc = concat!(guard_deref_mut_doc!($guard), "implementations.")]
    ///
    #[must_use = concat!("if unused the `", stringify!($mutex), "` will unlock immediately")]
    // #[must_not_suspend = "holding a `Mutex` across suspend points can cause deadlocks, delays, \
    //                       and cause Futures to not implement `Send`"]
    #[clippy::has_significant_drop]
    pub struct $guard<'m, T: ?Sized, A>
    where
      ($mutex<()>, A): LazyInit<SemaphoreHandle_t>,
    {
      lock: &'m $mutex<T, A>,
      _not_send_and_sync: PhantomData<*const ()>,
    }

    unsafe impl<T: ?Sized + Sync, A> Sync for $guard<'_, T, A>
    where
      ($mutex<()>, A): LazyInit<SemaphoreHandle_t>,
    {}

    impl<T: ?Sized, A> Deref for $guard<'_, T, A>
    where
      ($mutex<()>, A): LazyInit<SemaphoreHandle_t>,
    {
      type Target = T;

      /// Dereferences the locked value.
      #[inline]
      fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
      }
    }

    guard_impl_deref_mut!($guard);

    impl<T: ?Sized, A> Drop for $guard<'_, T, A>
    where
      ($mutex<()>, A): LazyInit<SemaphoreHandle_t>,
    {
      /// Unlocks the mutex.
      #[inline]
      fn drop(&mut self) {
        unsafe { $give(self.lock.handle.as_ptr()); }
      }
    }
  };
}

impl_mutex!(
  /// A mutual exclusion primitive useful for protecting shared data.
  Mutex,
  MutexGuard,
  xSemaphoreCreateMutex,
  xSemaphoreCreateMutexStatic,
  xSemaphoreTake,
  xSemaphoreGive,
  "",
);

impl_mutex!(
  /// A mutual exclusion primitive useful for protecting shared data which can be locked recursively.
  ///
  /// [`RecursiveMutexGuard`] does not give mutable references to the contained data,
  /// use a [`RefCell`](core::cell::RefCell) if you need this.
  RecursiveMutex,
  RecursiveMutexGuard,
  xSemaphoreCreateRecursiveMutex,
  xSemaphoreCreateRecursiveMutexStatic,
  xSemaphoreTakeRecursive,
  xSemaphoreGiveRecursive,
  "recursive",
);

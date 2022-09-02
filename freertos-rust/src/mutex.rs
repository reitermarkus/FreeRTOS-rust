use core::cell::UnsafeCell;
use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::time::Duration;
use core::pin::Pin;

use crate::ffi::SemaphoreHandle;
use crate::alloc::{Dynamic, Static};
use crate::error::FreeRtosError;
use crate::lazy_init::{LazyInit, LazyPtr};
use crate::shim::*;
use crate::ticks::*;

macro_rules! guard_impl_deref_mut {
  (MutexGuard) => {
    impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
      /// Mutably dereferences the locked value.
      fn deref_mut(&mut self) -> &mut T {
          unsafe { &mut *self.data.get() }
      }
    }
  };
  ($guard:ident) => {};
}

macro_rules! guard_deref_mut_doc {
  (MutexGuard) => { " and [`DerefMut`]" };
  ($guard:ident) => { "" };
}

macro_rules! impl_inner {
  ($take:ident, $guard:ident, $self_ty:ty $(, $get_ref:ident)?) => {
    /// Lock the pinned mutex.
    #[inline]
    pub fn lock(self: $self_ty) -> Result<$guard<'_, T>, FreeRtosError> {
      self.timed_lock(Duration::MAX)
    }

    /// Try locking the pinned mutex and return immediately.
    #[inline]
    pub fn try_lock(self: $self_ty) -> Result<$guard<'_, T>, FreeRtosError> {
      self.timed_lock(Duration::ZERO)
    }

    /// Try locking the pinned mutex until the given `timeout`.
    pub fn timed_lock(self: $self_ty, timeout: impl Into<Ticks>) -> Result<$guard<'_, T>, FreeRtosError> {
      let this = self$(.$get_ref())*;

      let handle = unsafe { SemaphoreHandle::from_ptr(this.handle.as_ptr()) };
      unsafe { handle.$take(timeout)? };

      Ok($guard { handle, data: &this.data })
    }
  };
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
      Self: LazyInit<SemaphoreHandle_t>,
    {
      handle: LazyPtr<Self, SemaphoreHandle_t>,
      _alloc_type: PhantomData<A>,
      data: UnsafeCell<T>,
    }

    impl<T: ?Sized> LazyInit<SemaphoreHandle_t> for $mutex<T, Dynamic> {
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

    impl<T: ?Sized> LazyInit<SemaphoreHandle_t> for $mutex<T, Static> {
      type Data = StaticSemaphore_t;

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
      Self: LazyInit<SemaphoreHandle_t>,
    {}
    unsafe impl<T: ?Sized + Send, A> Sync for $mutex<T, A>
    where
      Self: LazyInit<SemaphoreHandle_t>,
    {}

    impl<T, A> $mutex<T, A>
    where
      Self: LazyInit<SemaphoreHandle_t>,
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
      Self: LazyInit<SemaphoreHandle_t>,
    {
      /// Consume the mutex and return its inner value.
      pub fn into_inner(self) -> T {
        self.data.into_inner()
      }
    }

    impl<T: ?Sized> $mutex<T, Dynamic> {
      impl_inner!($take, $guard, &Self);
    }

    impl<T: ?Sized> $mutex<T, Static> {
      impl_inner!($take, $guard, Pin<&Self>, get_ref);
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
    pub struct $guard<'m, T: ?Sized> {
      handle: &'m SemaphoreHandle,
      data: &'m UnsafeCell<T>,
    }

    unsafe impl<T: ?Sized + Sync> Sync for $guard<'_, T> {}

    impl<T: ?Sized> Deref for $guard<'_, T> {
      type Target = T;

      /// Dereferences the locked value.
      #[inline]
      fn deref(&self) -> &T {
        unsafe { &*self.data.get() }
      }
    }

    guard_impl_deref_mut!($guard);

    impl<T: ?Sized> Drop for $guard<'_, T> {
      /// Unlocks the mutex.
      #[inline]
      fn drop(&mut self) {
        let _ = unsafe { self.handle.$give() };
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
  take,
  give,
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
  take_recursive,
  give_recursive,
  "recursive",
);

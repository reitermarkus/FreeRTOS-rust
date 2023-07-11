use core::cell::UnsafeCell;
use core::fmt;
use core::ptr;
use core::marker::PhantomData;
use core::mem::{MaybeUninit, ManuallyDrop};
use core::ops::{Deref, DerefMut};

use crate::ffi::SemaphoreHandle_t;
use crate::shim::*;
use crate::InterruptContext;

mod handle;
pub use handle::{MutexHandle, RecursiveMutexHandle};

macro_rules! guard_impl_deref_mut {
  (MutexGuard) => {
    impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
      /// Mutably dereferences the locked value.
      fn deref_mut(&mut self) -> &mut T {
        unsafe { self.handle.data_mut() }
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
    $handle:ident,
    $guard:ident,
    $create:ident,
    $create_static:ident,
    $variant_name:expr,
  ) => {
    $(#[$attr])*
    pub struct $mutex<T: ?Sized> {
      handle: ManuallyDrop<$handle<T>>,
    }

    unsafe impl<T: ?Sized + Send> Send for $mutex<T> {}
    unsafe impl<T: ?Sized + Send> Sync for $mutex<T> {}

    #[cfg(freertos_feature = "dynamic_allocation")]
    impl<T> $mutex<T> {
      #[doc = concat!("Create a new dynamic `", stringify!($mutex), "` with the given inner value.")]
      pub fn new(data: T) -> Self {
        unsafe {
          let ptr = $create();
          assert!(!ptr.is_null());
          Self {
            handle: ManuallyDrop::new($handle {
              ptr,
              data: UnsafeCell::new(data),
            })
          }
        }
      }
    }

    #[cfg(freertos_feature = "static_allocation")]
    impl<T> $mutex<T> {
      #[doc = concat!("Create a new static `", stringify!($mutex), "` with the given inner value.")]
      /// Create a new static queue.
      ///
      /// # Safety
      ///
      /// The returned mutex must have a `'static` lifetime.
      ///
      /// # Examples
      ///
      /// ```
      #[doc = concat!("use freertos_rust::{alloc::Static, sync::", stringify!($mutex), "};")]
      ///
      /// // SAFETY: Assignment to a `static` ensures a `'static` lifetime.
      #[doc = concat!("static MUTEX: ", stringify!($mutex), "<u32, Static> = unsafe {")]
      #[doc = concat!("  ", stringify!($mutex), "::new_static(123)")]
      /// };
      /// ```
      pub fn new_static(mutex: &'static mut MaybeUninit<StaticMutex>, data: T) -> Self {
        let mutex_ptr = mutex.as_mut_ptr();

        unsafe {
          let ptr = $create_static(ptr::addr_of_mut!((*mutex_ptr).data));
          debug_assert!(!ptr.is_null());
          debug_assert_eq!(ptr, ptr::addr_of_mut!((*mutex_ptr).data) as SemaphoreHandle_t);

          Self {
            handle: ManuallyDrop::new($handle {
              ptr,
              data: UnsafeCell::new(data),
            })
          }
        }
      }
    }

    impl<T> $mutex<T> {
      /// Consume the mutex and return its inner value.
      pub fn into_inner(mut self) -> T {
        unsafe {
          vSemaphoreDelete(self.as_ptr());
          ManuallyDrop::take(&mut self.handle).data.into_inner()
        }
      }
    }

    impl<T: ?Sized> Deref for $mutex<T> {
      type Target = $handle<T>;

      fn deref(&self) -> &Self::Target {
        &self.handle
      }
    }

    impl<T: ?Sized> Drop for $mutex<T> {
      fn drop(&mut self) {
        unsafe {
          vSemaphoreDelete(self.as_ptr());
          ManuallyDrop::drop(&mut self.handle);
        }
      }
    }

    impl<T: fmt::Debug> fmt::Debug for $mutex<T> {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = f.debug_struct(stringify!($mutex));

        d.field("handle", &self.as_ptr());

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
      handle: &'m $handle<T>,
    }

    unsafe impl<T: ?Sized + Sync> Sync for $guard<'_, T> {}

    impl<T: ?Sized> Deref for $guard<'_, T> {
      type Target = T;

      /// Dereferences the locked value.
      #[inline]
      fn deref(&self) -> &T {
        // SAFETY: Mutex is locked.
        unsafe { self.handle.data() }
      }
    }

    guard_impl_deref_mut!($guard);

    impl<T: ?Sized> Drop for $guard<'_, T> {
      /// Unlocks the mutex.
      #[inline]
      fn drop(&mut self) {
        let _ = self.handle.give();
      }
    }
  };
}

impl_mutex!(
  /// A mutual exclusion primitive useful for protecting shared data.
  Mutex,
  MutexHandle,
  MutexGuard,
  xSemaphoreCreateMutex,
  xSemaphoreCreateMutexStatic,
  "",
);

unsafe impl<'m, T: ?Sized> Send for MutexGuard<'m, T> {}

impl<'m, T: ?Sized> MutexGuard<'m, T> {
  /// Converts this `MutexGuard` into a `IsrMutexGuard`.
  pub fn into_isr<'ic>(self, ic: &'ic InterruptContext) -> IsrMutexGuard<'ic, 'm, T> {
    let this = ManuallyDrop::new(self);
    IsrMutexGuard { ic, handle: this.handle }
  }
}

impl<'ic, 'm, T: ?Sized> From<IsrMutexGuard<'ic, 'm, T>> for MutexGuard<'m, T> {
  fn from(guard: IsrMutexGuard<'ic, 'm, T>) -> Self {
    let guard = ManuallyDrop::new(guard);
    MutexGuard { handle: guard.handle }
  }
}

/// An RAII implementation of a “scoped lock” of a mutex.
///
///  When this structure is
/// dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this guard via its [`Deref`]
/// and [`DerefMut`] implementations.
///
#[must_use = "if unused the `Mutex` will unlock immediately"]
// #[must_not_suspend = "holding a `Mutex` across suspend points can cause deadlocks, delays, \
//                       and cause Futures to not implement `Send`"]
#[clippy::has_significant_drop]
pub struct IsrMutexGuard<'ic, 'm, T: ?Sized> {
  ic: &'ic InterruptContext,
  handle: &'m MutexHandle<T>,
}

impl<T: ?Sized> Deref for IsrMutexGuard<'_, '_, T> {
  type Target = T;

  /// Dereferences the locked value.
  #[inline]
  fn deref(&self) -> &T {
    // SAFETY: Mutex is locked.
    unsafe { self.handle.data() }
  }
}

impl<T: ?Sized> DerefMut for IsrMutexGuard<'_, '_, T> {
  /// Dereferences the locked value.
  #[inline]
  fn deref_mut(&mut self) -> &mut T {
    // SAFETY: Mutex is locked.
    unsafe { self.handle.data_mut() }
  }
}

impl<T: ?Sized> Drop for IsrMutexGuard<'_, '_, T> {
  fn drop(&mut self) {
      let _ = self.handle.give_from_isr(self.ic);
  }
}

impl_mutex!(
  /// A mutual exclusion primitive useful for protecting shared data which can be locked recursively.
  ///
  /// [`RecursiveMutexGuard`] does not give mutable references to the contained data,
  /// use a [`RefCell`](core::cell::RefCell) if you need this.
  RecursiveMutex,
  RecursiveMutexHandle,
  RecursiveMutexGuard,
  xSemaphoreCreateRecursiveMutex,
  xSemaphoreCreateRecursiveMutexStatic,
  "recursive",
);

pub struct StaticMutex {
  data: StaticSemaphore_t,
}

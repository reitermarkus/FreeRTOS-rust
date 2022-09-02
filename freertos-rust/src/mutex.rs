use core::cell::UnsafeCell;
use core::fmt;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::time::Duration;

use crate::error::FreeRtosError;
use crate::lazy_init::{LazyInit, LazyPtr};
use crate::shim::*;
use crate::ticks::*;

/// A mutual exclusion primitive useful for protecting shared data.
pub struct Mutex<T: ?Sized> {
  handle: LazyPtr<Mutex<()>>,
  data: UnsafeCell<T>,
}

impl LazyInit for Mutex<()> {
  type Ptr = QueueDefinition;

  fn init() -> NonNull<QueueDefinition> {
    unsafe {
      let ptr = xSemaphoreCreateMutex();
      assert!(!ptr.is_null());
      NonNull::new_unchecked(ptr)
    }
  }

  #[inline]
  fn destroy(ptr: NonNull<QueueDefinition>) {
    unsafe { vSemaphoreDelete(ptr.as_ptr()) }
  }
}

/// A mutual exclusion primitive useful for protecting shared data which can be locked recursively.
///
/// `RecursiveMutexGuard` does not give mutable references to the contained data,
/// use a `RefCell` if you need this.
pub struct RecursiveMutex<T: ?Sized> {
  handle: LazyPtr<RecursiveMutex<()>>,
  data: UnsafeCell<T>,
}

impl LazyInit for RecursiveMutex<()> {
  type Ptr = QueueDefinition;

  fn init() -> NonNull<QueueDefinition> {
    unsafe {
      let ptr = xSemaphoreCreateRecursiveMutex();
      assert!(!ptr.is_null());
      NonNull::new_unchecked(ptr)
    }
  }

  #[inline]
  fn destroy(ptr: NonNull<QueueDefinition>) {
    unsafe { vSemaphoreDelete(ptr.as_ptr()) }
  }
}

macro_rules! impl_mutex {
  ($name:ident, $guard:ident) => {
    unsafe impl<T: ?Sized + Send> Send for $name<T> {}
    unsafe impl<T: ?Sized + Send> Sync for $name<T> {}

    impl<T> $name<T> {
      /// Create a new mutex with the given inner value.
      pub const fn new(t: T) -> Self {
        Self {
          handle: LazyPtr::new(),
          data: UnsafeCell::new(t),
        }
      }

      /// Consume the mutex and return its inner value.
      pub fn into_inner(self) -> T {
        self.data.into_inner()
      }
    }

    impl<T: ?Sized> $name<T> {
      pub fn lock(&self) -> Result<$guard<'_, T>, FreeRtosError> {
        self.timed_lock(Duration::MAX)
      }

      pub fn try_lock(&self) -> Result<$guard<'_, T>, FreeRtosError> {
        self.timed_lock(Duration::ZERO)
      }
    }

    impl<T: ?Sized + fmt::Debug> fmt::Debug for $name<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
          let mut d = f.debug_struct(stringify!($name));

          d.field("handle", &self.handle.as_ptr());

          match self.try_lock() {
            Ok(guard) => d.field("data", &&*guard),
            Err(_) => d.field("data", &format_args!("<locked>")),
          };

          d.finish()
        }
    }
  };
}

impl_mutex!(Mutex, MutexGuard);
impl_mutex!(RecursiveMutex, RecursiveMutexGuard);

impl<T: ?Sized> Mutex<T> {
  pub fn timed_lock(&self, timeout: impl Into<Ticks>) -> Result<MutexGuard<'_, T>, FreeRtosError> {
    let timeout = timeout.into();

    let res = unsafe {
      xSemaphoreTake(self.handle.as_ptr(), timeout.as_ticks())
    };

    if res == pdTRUE {
      return Ok(MutexGuard { lock: self, _not_send_and_sync: PhantomData })
    }

    Err(FreeRtosError::Timeout)
  }
}

impl<T: ?Sized> RecursiveMutex<T> {
  pub fn timed_lock(&self, timeout: impl Into<Ticks>) -> Result<RecursiveMutexGuard<'_, T>, FreeRtosError> {
    let timeout = timeout.into();

    let res = unsafe {
      xSemaphoreTakeRecursive(self.handle.as_ptr(), timeout.as_ticks())
    };

    if res == pdTRUE {
      return Ok(RecursiveMutexGuard { lock: self, _not_send_and_sync: PhantomData })
    }

    Err(FreeRtosError::Timeout)
  }
}

/// An RAII implementation of a “scoped lock” of a mutex. When this structure is
/// dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this
/// guard via its `Deref` and `DerefMut` implementations.
#[must_use = "if unused the `Mutex` will unlock immediately"]
// #[must_not_suspend = "holding a `Mutex` across suspend points can cause deadlocks, delays, \
//                       and cause Futures to not implement `Send`"]
#[clippy::has_significant_drop]
pub struct MutexGuard<'m, T: ?Sized> {
    lock: &'m Mutex<T>,
    _not_send_and_sync: PhantomData<*const ()>,
}

unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
  type Target = T;

  fn deref(&self) -> &T {
      unsafe { &*self.lock.data.get() }
  }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
  fn deref_mut(&mut self) -> &mut T {
      unsafe { &mut *self.lock.data.get() }
  }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
  #[inline]
  fn drop(&mut self) {
    unsafe { xSemaphoreGive(self.lock.handle.as_ptr()); }
  }
}

/// An RAII implementation of a “scoped lock” of a recursive mutex. When this structure is
/// dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this
/// guard via its `Deref` implementations.
#[must_use = "if unused the `RecursiveMutex` will unlock immediately"]
// #[must_not_suspend = "holding a `RecursiveMutex` across suspend points can cause deadlocks, delays, \
//                       and cause Futures to not implement `Send`"]
#[clippy::has_significant_drop]
pub struct RecursiveMutexGuard<'m, T: ?Sized> {
  lock: &'m RecursiveMutex<T>,
  _not_send_and_sync: PhantomData<*const ()>,
}

unsafe impl<T: ?Sized + Sync> Sync for RecursiveMutexGuard<'_, T> {}

impl<T: ?Sized> Deref for RecursiveMutexGuard<'_, T> {
  type Target = T;

  fn deref(&self) -> &T {
    unsafe { &*self.lock.data.get() }
  }
}

impl<T: ?Sized> Drop for RecursiveMutexGuard<'_, T> {
  #[inline]
  fn drop(&mut self) {
    unsafe { xSemaphoreGiveRecursive(self.lock.handle.as_ptr()); }
  }
}

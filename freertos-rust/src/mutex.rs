use core::ptr::NonNull;

use crate::base::*;
use crate::lazy_init::LazyInit;
use crate::lazy_init::LazyPtr;
use crate::prelude::v1::*;
use crate::shim::*;
use crate::units::*;

/// A mutual exclusion primitive useful for protecting shared data.
pub struct Mutex<T: ?Sized> {
  handle: LazyPtr<Mutex<()>>,
  data: UnsafeCell<T>,
}

impl LazyInit for Mutex<()> {
  fn init() -> NonNull<CVoid> {
    unsafe {
      let ptr = freertos_rs_create_mutex();
      assert!(!ptr.is_null());
      NonNull::new_unchecked(ptr)
    }
  }

  #[inline]
  fn destroy(ptr: NonNull<CVoid>) {
    unsafe { freertos_rs_delete_semaphore(ptr.as_ptr()) }
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
  fn init() -> NonNull<CVoid> {
    unsafe {
      let ptr = freertos_rs_create_recursive_mutex();
      assert!(!ptr.is_null());
      NonNull::new_unchecked(ptr)
    }
  }

  #[inline]
  fn destroy(ptr: NonNull<CVoid>) {
    unsafe { freertos_rs_delete_semaphore(ptr.as_ptr()) }
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
        self.timed_lock(Duration::infinite())
      }

      pub fn try_lock(&self) -> Result<$guard<'_, T>, FreeRtosError> {
        self.timed_lock(Duration::zero())
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
  pub fn timed_lock<D: DurationTicks>(&self, max_wait: D) -> Result<MutexGuard<'_, T>, FreeRtosError> {
    let res = unsafe {
      freertos_rs_take_mutex(self.handle.as_ptr(), max_wait.to_ticks())
    };

    if res != 0 {
      return Err(FreeRtosError::MutexTimeout);
    }

    Ok(MutexGuard { lock: self, _not_send_and_sync: PhantomData })
  }
}

impl<T: ?Sized> RecursiveMutex<T> {
  pub fn timed_lock<D: DurationTicks>(&self, max_wait: D) -> Result<RecursiveMutexGuard<'_, T>, FreeRtosError> {
    let res = unsafe {
      freertos_rs_take_recursive_mutex(self.handle.as_ptr(), max_wait.to_ticks())
    };

    if res != 0 {
      return Err(FreeRtosError::MutexTimeout);
    }

    Ok(RecursiveMutexGuard { lock: self, _not_send_and_sync: PhantomData })
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
    unsafe { freertos_rs_give_mutex(self.lock.handle.as_ptr()); }
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
    unsafe { freertos_rs_give_recursive_mutex(self.lock.handle.as_ptr()); }
  }
}

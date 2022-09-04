use core::cell::UnsafeCell;
use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::ptr;

use crate::alloc::{Dynamic, Static};
use crate::lazy_init::{LazyInit, LazyPtr};
use crate::shim::*;

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
    pub struct $mutex<T: ?Sized, A = Dynamic>
    where
      Self: LazyInit,
    {
      alloc_type: PhantomData<A>,
      handle: LazyPtr<Self, T>,
    }

    impl<T: ?Sized> LazyInit for $mutex<T, Dynamic> {
      type Handle = SemaphoreHandle_t;

      fn init(_storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
        unsafe {
          let ptr = $create();
          assert!(!ptr.is_null());
          Self::Ptr::new_unchecked(ptr)
        }
      }

      #[inline]
      fn destroy(ptr: Self::Ptr, _storage: &mut MaybeUninit<Self::Storage>) {
        unsafe { vSemaphoreDelete(ptr.as_ptr()) }
      }
    }

    impl<T: ?Sized> LazyInit for $mutex<T, Static> {
      type Storage = StaticSemaphore_t;
      type Handle = SemaphoreHandle_t;

      fn init(storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
        unsafe {
          let storage = &mut *storage.get();
          let ptr = $create_static(storage.as_mut_ptr());
          assert!(!ptr.is_null());
          Self::Ptr::new_unchecked(ptr)
        }
      }

      fn cancel_init_supported() -> bool {
        false
      }

      #[inline]
      fn destroy(ptr: Self::Ptr, storage: &mut MaybeUninit<Self::Storage>) {
        unsafe {
          vSemaphoreDelete(ptr.as_ptr());
          storage.assume_init_drop();
        }
      }
    }

    unsafe impl<T: ?Sized + Send, A> Send for $mutex<T, A>
    where
      Self: LazyInit,
    {}
    unsafe impl<T: ?Sized + Send, A> Sync for $mutex<T, A>
    where
      Self: LazyInit,
    {}

    impl<T> $mutex<T, Dynamic>
    where
      Self: LazyInit,
    {
      #[doc = concat!("Create a new dynamic `", stringify!($mutex), "` with the given inner value.")]
      pub const fn new(data: T) -> Self {
        Self {
          alloc_type: PhantomData,
          handle: LazyPtr::new(data),
        }
      }
    }

    impl<T> $mutex<T, Static>
    where
      Self: LazyInit,
    {
      #[doc = concat!("Create a new static `", stringify!($mutex), "` with the given inner value.")]
      /// Create a new static queue.
      ///
      /// # Safety
      ///
      /// The returned mutex must be [pinned](core::pin) before using it.
      ///
      /// # Examples
      ///
      /// ```
      /// use core::pin::Pin;
      /// use freertos_rust::sync::Mutex;
      ///
      /// // SAFETY: Assignment to a `static` ensures the semaphore will never move.
      #[doc = concat!("pub static MUTEX: ", stringify!($mutex), "<u32> = unsafe {")]
      #[doc = concat!("  Pin::new_unchecked(", stringify!($mutex), "::new_static(123))")]
      /// }
      /// ```
      pub const unsafe fn new_static(data: T) -> Self {
        Self {
          alloc_type: PhantomData,
          handle: LazyPtr::new(data),
        }
      }
    }


    impl<T, A> $mutex<T, A>
    where
      Self: LazyInit,
    {
      /// Consume the mutex and return its inner value.
      pub fn into_inner(self) -> T {
        self.handle.into_data()
      }
    }

    impl<T, A> Deref for $mutex<T, A>
    where
      Self: LazyInit,
    {
      type Target = $handle<T>;

      fn deref(&self) -> &Self::Target {
        // Ensure mutex is initialized.
        self.handle.as_ptr();

        unsafe { &*self.handle.ptr_ptr().cast() }
      }
    }

    impl<T: fmt::Debug, A> fmt::Debug for $mutex<T, A>
    where
      Self: LazyInit,
    {
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

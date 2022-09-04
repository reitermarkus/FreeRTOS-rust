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
      handle: LazyPtr<Self>,
    }

    impl<T: ?Sized> LazyInit for $mutex<T, Dynamic> {
      type Handle = SemaphoreHandle_t;
      type Data = T;

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
      type Data = T;

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
          ptr::drop_in_place(storage.as_mut_ptr());
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
      Self: LazyInit<Data = T>,
    {
      #[doc = concat!("Create a new dynamic `", stringify!($mutex), "` with the given inner value.")]
      pub const fn new(data: T) -> Self {
        Self {
          alloc_type: PhantomData,
          handle: LazyPtr::new(data),
        }
      }
    }

    impl $mutex<(), Dynamic> {
      pub const unsafe fn from_ptr(ptr: SemaphoreHandle_t) -> Self {
        Self {
          alloc_type: PhantomData,
          handle: unsafe { LazyPtr::new_unchecked(ptr, ()) },
        }
      }
    }

    impl<T> $mutex<T, Static>
    where
      Self: LazyInit<Data = T>,
    {
      #[doc = concat!("Create a new static `", stringify!($mutex), "` with the given inner value.")]
      /// Create a new static queue.
      ///
      /// # Safety
      ///
      /// The returned mutex must be pinned before using it.
      ///
      /// # Examples
      ///
      /// ```
      /// use freertos_rust::pin_static;
      ///
      /// pin_static!(pub static MUTEX = Mutex::<u32>::new_static(123));
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
      Self: LazyInit<Data = T>,
    {
      /// Consume the mutex and return its inner value.
      pub fn into_inner(self) -> T {
        self.handle.into_data()
      }
    }

    impl<T, A> Deref for $mutex<T, A>
    where
      Self: LazyInit<Data = T>,
    {
      type Target = $handle<T>;

      fn deref(&self) -> &Self::Target {
        unsafe {
          let storage = ptr::addr_of!(self.handle).cast::<<Self as LazyInit>::Storage>();
          let handle = storage.add(1).cast::<Self::Target>();
          &*handle
        }
      }
    }

    impl<T: fmt::Debug> fmt::Debug for $mutex<T>
    where
      Self: LazyInit<Data = T>,
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

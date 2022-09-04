use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::ptr;

use crate::alloc::{Dynamic, Static};
use crate::lazy_init::{LazyPtr, LazyInit};
use crate::shim::*;

mod guard;
pub use guard::SemaphoreGuard;
mod handle;
pub use handle::SemaphoreHandle;

/// Marker type for a binary semaphore.
#[non_exhaustive]
pub struct Binary {}

/// Marker type for a counting semaphore.
#[non_exhaustive]
pub struct Counting<const MAX: u32, const INITIAL: u32> {}

/// A counting or binary semaphore.
pub struct Semaphore<T, A = Dynamic>
where
  Self: LazyInit,
{
  alloc_type: PhantomData<A>,
  handle: LazyPtr<Self>,
}

macro_rules! impl_semaphore {
  (
    $semaphore:ident $(<const $max:ident: $max_ty:ident, const $initial:ident: $initial_ty:ident>)?,
    $create:ident,
    $create_static:ident,
    $new_fn:ident,
    $new_fn_static:ident,
    $variant_name:ident,
  ) => {
    impl<$(const $max: $max_ty, const $initial: $initial_ty,)*> Semaphore<$semaphore$(<$max, $initial>)*, Dynamic>
    where
      Self: LazyInit,
    {
      #[doc = concat!("Create a new dynamic ", stringify!($variant_name), " semaphore.")]
      pub const fn $new_fn() -> Self {
        $(assert!($initial <= $max);)*

        Self { alloc_type: PhantomData, handle: LazyPtr::new(()) }
      }
    }

    impl<$(const $max: $max_ty, const $initial: $initial_ty,)*> Semaphore<$semaphore$(<$max, $initial>)*, Static>
    where
      Self: LazyInit,
    {
      #[doc = concat!("Create a new static ", stringify!($variant_name), " semaphore.")]
      ///
      /// # Safety
      ///
      /// The returned mutex must be [pinned](core::pin) before using it.
      ///
      /// # Examples
      ///
      /// ```
      /// use core::pin::Pin;
      /// use freertos_rust::sync::Semaphore;
      ///
      /// // SAFETY: Assignment to a `static` ensures the semaphore will never move.
      #[doc = concat!("pub static SEMAPHORE: Pin<Semaphore<", stringify!($semaphore), ", Static>> = unsafe {")]
      #[doc = concat!("  Pin::new_unchecked(Semaphore::", stringify!($new_fn_static), "())")]
      /// };
      /// ```
      pub const unsafe fn $new_fn_static() -> Self {
        $(assert!($initial <= $max);)*

        Self { alloc_type: PhantomData, handle: LazyPtr::new(()) }
      }
    }

    impl<$(const $max: $max_ty, const $initial: $initial_ty,)* A> Deref for Semaphore<$semaphore$(<$max, $initial>)*, A>
    where
      Self: LazyInit<Handle = SemaphoreHandle_t>,
    {
      type Target = SemaphoreHandle;

      fn deref(&self) -> &Self::Target {
        // Ensure semaphore is initialized.
        let handle = self.handle.as_ptr();
        unsafe { SemaphoreHandle::from_ptr(handle) }
      }
    }

    impl$(<const $max: $max_ty, const $initial: $initial_ty>)* LazyInit for Semaphore<$semaphore$(<$max, $initial>)*, Dynamic> {
      type Handle = SemaphoreHandle_t;

      fn init(_storage: &UnsafeCell<MaybeUninit<()>>) -> Self::Ptr {
        let ptr = unsafe { $create($($max, $initial)*) };
        assert!(!ptr.is_null());

        unsafe { Self::Ptr::new_unchecked(ptr) }
      }

      fn destroy(ptr: Self::Ptr, _storage: &mut MaybeUninit<Self::Storage>) {
        unsafe { vSemaphoreDelete(ptr.as_ptr()) }
      }
    }

    impl$(<const $max: $max_ty, const $initial: $initial_ty>)* LazyInit for Semaphore<$semaphore$(<$max, $initial>)*, Static> {
      type Handle = SemaphoreHandle_t;
      type Storage = StaticSemaphore_t;

      fn init(storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
        unsafe {
          let storage = &mut *storage.get();
          let ptr = $create_static($($max, $initial,)* storage.as_mut_ptr());
          assert!(!ptr.is_null());
          Self::Ptr::new_unchecked(ptr)
        }
      }

      fn cancel_init_supported() -> bool {
        false
      }

      fn destroy(ptr: Self::Ptr, storage: &mut MaybeUninit<Self::Storage>) {
        unsafe {
          vSemaphoreDelete(ptr.as_ptr());
          storage.assume_init_drop();
        }
      }
    }
  };
}

impl_semaphore!(
  Binary,
  xSemaphoreCreateBinary,
  xSemaphoreCreateBinaryStatic,
  new_binary,
  new_binary_static,
  binary,
);

impl_semaphore!(
  Counting<const MAX: u32, const INITIAL: u32>,
  xSemaphoreCreateCounting,
  xSemaphoreCreateCountingStatic,
  new_counting,
  new_counting_static,
  counting,
);

unsafe impl<T: Send, A: Send> Send for Semaphore<T, A>
where
  Self: LazyInit,
{}
unsafe impl<T: Send, A> Sync for Semaphore<T, A>
where
  Self: LazyInit,
{}

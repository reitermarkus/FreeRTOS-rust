use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::Deref;

use crate::alloc::Dynamic;
#[cfg(freertos_feature = "static_allocation")]
use crate::alloc::Static;
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
pub struct Counting<const MAX: usize, const INITIAL: usize> {}

/// A binary or counting semaphore.
pub struct Semaphore<T, A = Dynamic>
where
  Self: LazyInit,
{
  alloc_type: PhantomData<A>,
  handle: LazyPtr<Self>,
}

macro_rules! impl_semaphore {
  (
    $semaphore:ident $(<const $max:ident: $max_ty:ident = $max_val:literal, const $initial:ident: $initial_ty:ident = $initial_val:literal>)?,
    $create:ident,
    $create_static:ident,
    $new_fn:ident,
    $new_fn_static:ident,
    $variant_name:ident,
  ) => {
    impl<$(const $max: $max_ty, const $initial: $initial_ty,)*> Semaphore<$semaphore$(<$max, $initial>)*, Dynamic>
    where
      Self: LazyInit<Data = ()>,
    {
      #[doc = concat!("Create a new dynamic ", stringify!($variant_name), " semaphore.")]
      pub const fn $new_fn() -> Self {
        $(assert!($initial <= $max);)*

        Self { alloc_type: PhantomData, handle: LazyPtr::new(()) }
      }
    }

    #[cfg(freertos_feature = "static_allocation")]
    impl<$(const $max: $max_ty, const $initial: $initial_ty,)*> Semaphore<$semaphore$(<$max, $initial>)*, Static>
    where
      Self: LazyInit<Data = ()>,
    {
      #[doc = concat!("Create a new static ", stringify!($variant_name), " semaphore.")]
      ///
      /// # Safety
      ///
      /// The returned semaphore must have a `'static` lifetime.
      ///
      /// # Examples
      ///
      /// ```
      #[doc = concat!("use freertos_rust::{alloc::Static, sync::{Semaphore, ", stringify!($semaphore), "}};")]
      ///
      /// // SAFETY: Assignment to a `static` ensures a `'static` lifetime.
      #[doc = concat!("static SEMAPHORE: Semaphore<", stringify!($semaphore), $("<", stringify!($max_val), ", ", stringify!($initial_val), ">",)* ", Static> = unsafe {")]
      #[doc = concat!("  Semaphore::", stringify!($new_fn_static), "()")]
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
      type Storage = ();
      type Handle = SemaphoreHandle_t;
      type Data = ();

      fn init(_data: &UnsafeCell<Self::Data>, _storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
        let ptr = unsafe { $create($($max as _, $initial as _)*) };
        assert!(!ptr.is_null());

        unsafe { Self::Ptr::new_unchecked(ptr) }
      }

      fn destroy(ptr: Self::Ptr, _storage: &mut MaybeUninit<Self::Storage>) {
        unsafe { vSemaphoreDelete(ptr.as_ptr()) }
      }
    }

    #[cfg(freertos_feature = "static_allocation")]
    impl$(<const $max: $max_ty, const $initial: $initial_ty>)* LazyInit for Semaphore<$semaphore$(<$max, $initial>)*, Static> {
      type Storage = StaticSemaphore_t;
      type Handle = SemaphoreHandle_t;
      type Data = ();

      fn init(_data: &UnsafeCell<Self::Data>, storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
        unsafe {
          let storage = &mut *storage.get();
          let ptr = $create_static($($max as _, $initial as _,)* storage.as_mut_ptr());
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
  Counting<const MAX: usize = 4, const INITIAL: usize = 0>,
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

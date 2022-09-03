use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::Deref;

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
  Self: LazyInit<SemaphoreHandle_t>,
{
    handle: LazyPtr<Self, SemaphoreHandle_t>,
    _alloc_type: PhantomData<A>,
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
      Self: LazyInit<SemaphoreHandle_t>,
    {
      #[doc = concat!("Create a new dynamic ", stringify!($variant_name), " semaphore.")]
      pub const fn $new_fn() -> Self {
        $(assert!($initial <= $max);)*

        Self { handle: LazyPtr::new(), _alloc_type: PhantomData }
      }
    }

    impl<$(const $max: $max_ty, const $initial: $initial_ty,)*> Semaphore<$semaphore$(<$max, $initial>)*, Static>
    where
      Self: LazyInit<SemaphoreHandle_t>,
    {
      #[doc = concat!("Create a new static ", stringify!($variant_name), " semaphore.")]
      ///
      /// # Safety
      ///
      /// The returned semaphore must be pinned before using it.
      ///
      /// # Examples
      ///
      /// ```
      /// use freertos_rust::pin_static;
      ///
      #[doc = concat!("pin_static!(pub static SEMAPHORE = Semaphore::<", stringify!($semaphore), ">::", stringify!($new_fn_static), "());")]
      /// ```
      pub const unsafe fn $new_fn_static() -> Self {
        $(assert!($initial <= $max);)*

        Self { handle: LazyPtr::new(), _alloc_type: PhantomData }
      }
    }

    impl<$(const $max: $max_ty, const $initial: $initial_ty,)* A> Deref for Semaphore<$semaphore$(<$max, $initial>)*, A>
    where
      Self: LazyInit<SemaphoreHandle_t>,
    {
      type Target = SemaphoreHandle;

      fn deref(&self) -> &Self::Target {
        unsafe { SemaphoreHandle::from_ptr(self.handle.as_ptr()) }
      }
    }

    impl$(<const $max: $max_ty, const $initial: $initial_ty>)* LazyInit<SemaphoreHandle_t> for Semaphore<$semaphore$(<$max, $initial>)*, Dynamic> {
      fn init(_data: &UnsafeCell<MaybeUninit<()>>) -> Self::Ptr {
        let ptr = unsafe { $create($($max, $initial)*) };
        assert!(!ptr.is_null());
        unsafe { Self::Ptr::new_unchecked(ptr) }
      }

      fn destroy(ptr: Self::Ptr) {
        unsafe { vSemaphoreDelete(ptr.as_ptr()) }
      }
    }

    impl$(<const $max: $max_ty, const $initial: $initial_ty>)* LazyInit<SemaphoreHandle_t> for Semaphore<$semaphore$(<$max, $initial>)*, Static> {
      type Data = StaticSemaphore_t;

      fn init(data: &UnsafeCell<MaybeUninit<Self::Data>>) -> Self::Ptr {
        unsafe {
          let data = &mut *data.get();
          let ptr = $create_static($($max, $initial,)* data.as_mut_ptr());
          assert!(!ptr.is_null());
          Self::Ptr::new_unchecked(ptr)
        }
      }

      fn cancel_init_supported() -> bool {
        false
      }

      fn destroy(ptr: Self::Ptr) {
        drop(ptr)
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
  Self: LazyInit<SemaphoreHandle_t>,
{}
unsafe impl<T: Send, A> Sync for Semaphore<T, A>
where
  Self: LazyInit<SemaphoreHandle_t>,
{}

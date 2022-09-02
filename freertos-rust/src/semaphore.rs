use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::pin::Pin;

use crate::alloc::{Dynamic, Static};
use crate::ffi::SemaphoreHandle;
use crate::lazy_init::{LazyPtr, LazyInit};
use crate::{error::*, InterruptContext};
use crate::shim::*;
use crate::ticks::*;

/// Marker type for a binary semaphore.
#[non_exhaustive]
pub struct Binary {}

/// Marker type for a counting semaphore.
#[non_exhaustive]
pub struct Counting<const MAX: u32, const INITIAL: u32> {}

/// A counting or binary semaphore.
pub struct Semaphore<T, S = Dynamic>
where
  (T, S): LazyInit<SemaphoreHandle_t>,
{
    handle: LazyPtr<(T, S), SemaphoreHandle_t>,
    _alloc_type: PhantomData<S>,
}

macro_rules! impl_inner {
    ($self_ty:ty $(, $get_ref:ident)?) => {
      /// Get the raw semaphore handle.
      pub fn as_ptr(&self) -> SemaphoreHandle_t {
        self.handle.as_ptr()
      }

      unsafe fn handle(self: $self_ty) -> &'_ SemaphoreHandle {
        SemaphoreHandle::from_ptr(self$(.$get_ref())*.as_ptr())
      }

      /// Increment the semaphore.
      #[inline]
      pub fn give(self: $self_ty) -> Result<(), FreeRtosError> {
        unsafe { self.handle().give() }
      }

      /// Increment the semaphore from within an interrupt service routine.
      #[inline]
      pub fn give_from_isr(self: $self_ty, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
        unsafe { self.handle().give_from_isr(ic) }
      }

      /// Decrement the semaphore.
      #[inline]
      pub fn take(self: $self_ty, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
        unsafe { self.handle().take(timeout) }
      }

      /// Decrement the semaphore from within an interrupt service routine.
      #[inline]
      pub fn take_from_isr(self: $self_ty, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
        unsafe { self.handle().take_from_isr(ic) }
      }

      /// Lock this semaphore in RAII fashion.
      pub fn lock(self: $self_ty, timeout: impl Into<Ticks>) -> Result<SemaphoreGuard<'_>, FreeRtosError> {
        let handle = unsafe { self.handle() };
        unsafe { handle.take(timeout)? };
        Ok(SemaphoreGuard { handle })
      }
    };
}

macro_rules! impl_semaphore {
    (
      $semaphore:ident $(<const $max:ident: $max_ty:ident, const $initial:ident: $initial_ty:ident>)?,
      $create:ident,
      $create_static:ident,
      $new_fn:ident,
      $variant_name:ident,
    ) => {
      impl<$(const $max: $max_ty, const $initial: $initial_ty,)* A> Semaphore<$semaphore$(<$max, $initial>)*, A>
      where
        ($semaphore$(<$max, $initial>)*, A): LazyInit<SemaphoreHandle_t>,
      {
        #[doc = concat!("Create a new ", stringify!($variant_name), " semaphore.")]
        pub const fn $new_fn() -> Self {
          $(assert!($initial <= $max);)*

          Self { handle: LazyPtr::new(), _alloc_type: PhantomData }
        }
      }

      impl$(<const $max: $max_ty, const $initial: $initial_ty>)* Semaphore<$semaphore$(<$max, $initial>)*, Dynamic> {
        impl_inner!(&Self);
      }

      impl$(<const $max: $max_ty, const $initial: $initial_ty>)* Semaphore<$semaphore$(<$max, $initial>)*, Static> {
        impl_inner!(Pin<&Self>, get_ref);
      }

      impl$(<const $max: $max_ty, const $initial: $initial_ty>)* LazyInit<SemaphoreHandle_t> for ($semaphore$(<$max, $initial>)*, Dynamic) {
        fn init(_data: &UnsafeCell<MaybeUninit<()>>) -> Self::Ptr {
          let ptr = unsafe { $create($($max, $initial)*) };
          assert!(!ptr.is_null());
          unsafe { Self::Ptr::new_unchecked(ptr) }
        }

        fn destroy(ptr: Self::Ptr) {
          unsafe { vSemaphoreDelete(ptr.as_ptr()) }
        }
      }

      impl$(<const $max: $max_ty, const $initial: $initial_ty>)* LazyInit<SemaphoreHandle_t> for ($semaphore$(<$max, $initial>)*, Static) {
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
  binary,
);

impl_semaphore!(
  Counting<const MAX: u32, const INITIAL: u32>,
  xSemaphoreCreateCounting,
  xSemaphoreCreateCountingStatic,
  new_counting,
  counting,
);

unsafe impl<T: Send, A: Send> Send for Semaphore<T, A>
where
  (T, A): LazyInit<SemaphoreHandle_t>,
{}
unsafe impl<T: Send, A> Sync for Semaphore<T, A>
where
  (T, A): LazyInit<SemaphoreHandle_t>,
{}

/// An RAII implementation of a “scoped decrement” of a semaphore.
///
/// When this structure is dropped (falls out of scope), the semaphore is incremented again.
#[must_use = concat!("if unused the `Semaphore` will increment again immediately")]
// #[must_not_suspend = "holding a `Semaphore` across suspend points can cause deadlocks, delays, \
//                       and cause Futures to not implement `Send`"]
#[derive(Debug)]
#[must_use = ""]
pub struct SemaphoreGuard<'s> {
  handle: &'s SemaphoreHandle,
}

impl Drop for SemaphoreGuard<'_> {
  fn drop(&mut self) {
    let _ = unsafe { self.handle.give() };
  }
}

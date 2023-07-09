use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr;
use core::ops::Deref;

mod guard;
pub use guard::SemaphoreGuard;
mod handle;
pub use handle::SemaphoreHandle;

use crate::{
  ffi::SemaphoreHandle_t,
  shim::{
    StaticSemaphore_t,
    vSemaphoreDelete,
    xSemaphoreCreateBinary,
    xSemaphoreCreateCountingStatic,
    xSemaphoreCreateCounting,
    xSemaphoreCreateBinaryStatic,
  },
};

/// Marker type for a binary semaphore.
#[non_exhaustive]
pub struct Binary {}

/// Marker type for a counting semaphore.
#[non_exhaustive]
pub struct Counting<const INITIAL: usize, const MAX: usize> {}

/// A binary or counting semaphore.
///
/// # Example
///
/// ```
/// use freertos_rust::sync::{Semaphore, Counting};
///
/// let binary_semaphore = Semaphore::new_binary();
/// binary_semaphore.give().unwrap();
///
/// let counting_semaphore = Semaphore::<Counting<3, 8>>::new_counting();
/// for _ in 0..3 {
///   counting_semaphore.take().unwrap();
/// }
/// for _ in 0..8 {
///   counting_semaphore.give().unwrap();
/// }
/// ```
pub struct Semaphore<T> {
  handle: SemaphoreHandle_t,
  mode: PhantomData<T>,
}

macro_rules! impl_semaphore {
  (
    $semaphore:ident $(<const $initial:ident: $initial_ty:ident = $initial_val:literal, const $max:ident: $max_ty:ident = $max_val:literal>)?,
    $create:ident,
    $new_fn:ident,
    $variant_name:ident,
  ) => {
    impl<$(const $initial: $initial_ty, const $max: $max_ty)*> Semaphore<$semaphore$(<$initial, $max>)*> {
      #[doc = concat!("Create a new dynamic ", stringify!($variant_name), " semaphore.")]
      pub fn $new_fn() -> Self {
        $(assert!($initial <= $max);)*

        let ptr = unsafe { $create($($max as _, $initial as _)*) };
        assert!(!ptr.is_null());

        Self { handle: ptr, mode: PhantomData }
      }
    }
  };
}



impl_semaphore!(
  Binary,
  xSemaphoreCreateBinary,
  new_binary,
  binary,
);

impl_semaphore!(
  Counting<const INITIAL: usize = 0, const MAX: usize = 4>,
  xSemaphoreCreateCounting,
  new_counting,
  counting,
);

impl<T> Deref for Semaphore<T> {
  type Target = SemaphoreHandle;

  fn deref(&self) -> &Self::Target {
    unsafe { SemaphoreHandle::from_ptr(self.handle) }
  }
}

impl<T> Drop for Semaphore<T> {
  fn drop(&mut self) {
    unsafe { vSemaphoreDelete(self.as_ptr()) }
  }
}

unsafe impl<T: Send> Send for Semaphore<T> {}
unsafe impl<T: Sync> Sync for Semaphore<T> {}

/// A statically allocated binary or counting semaphore.
///
/// # Example
///
/// ```
/// use core::mem::MaybeUninit;
///
/// use freertos_rust::sync::{StaticSemaphore, Counting};
///
///
/// let binary_semaphore = StaticSemaphore::new_binary(unsafe {
///   static mut BINARY_SEMAPHORE: MaybeUninit<StaticSemaphore> = MaybeUninit::uninit();
///   &mut BINARY_SEMAPHORE
/// });
/// binary_semaphore.give().unwrap();
///
/// let counting_semaphore = Semaphore::<Counting<3, 8>>::new_counting(unsafe {
///   static mut COUNTING_SEMAPHORE: MaybeUninit<StaticSemaphore> = MaybeUninit::uninit();
///   &mut COUNTING_SEMAPHORE
/// });
/// for _ in 0..3 {
///   counting_semaphore.take().unwrap();
/// }
/// for _ in 0..8 {
///   counting_semaphore.give().unwrap();
/// }
/// ```
pub struct StaticSemaphore<T> {
  data: StaticSemaphore_t,
  mode: PhantomData<T>,
}

unsafe impl<T: Send> Send for StaticSemaphore<T> {}
unsafe impl<T: Sync> Sync for StaticSemaphore<T> {}

macro_rules! impl_static_semaphore {
  (
    $semaphore:ident $(<const $initial:ident: $initial_ty:ident = $initial_val:literal, const $max:ident: $max_ty:ident = $max_val:literal>)?,
    $create:ident,
    $new_fn:ident,
    $variant_name:ident,
  ) => {
    impl<$(const $initial: $initial_ty, const $max: $max_ty)*> StaticSemaphore<$semaphore$(<$max, $initial>)*> {
      #[doc = concat!("Create a new static ", stringify!($variant_name), " semaphore.")]
      pub fn $new_fn(semaphore: &'static mut MaybeUninit<StaticSemaphore<$semaphore$(<$initial, $max>)*>>) -> &'static StaticSemaphore<$semaphore$(<$initial, $max>)*> {
        $(assert!($initial <= $max);)*

        let semaphore_ptr = semaphore.as_mut_ptr();

        unsafe {
          let ptr = $create($($max as _, $initial as _,)* ptr::addr_of_mut!((*semaphore_ptr).data));
          debug_assert!(!ptr.is_null());
          debug_assert_eq!(ptr, ptr::addr_of_mut!((*semaphore_ptr).data).cast());
          semaphore.assume_init_ref()
        }
      }
    }
  };
}

impl_static_semaphore!(
  Binary,
  xSemaphoreCreateBinaryStatic,
  new_binary,
  binary,
);

impl_static_semaphore!(
  Counting<const INITIAL: usize = 0, const MAX: usize = 4>,
  xSemaphoreCreateCountingStatic,
  new_counting,
  counting,
);

impl<T> Deref for StaticSemaphore<T> {
  type Target = SemaphoreHandle;

  fn deref(&self) -> &Self::Target {
    unsafe { SemaphoreHandle::from_ptr(ptr::addr_of!(self.data) as SemaphoreHandle_t) }
  }
}

impl<T> Drop for StaticSemaphore<T> {
  fn drop(&mut self) {
    unsafe { vSemaphoreDelete(self.as_ptr()) }
  }
}

use core::mem::MaybeUninit;
use core::ops::Deref;

use crate::lazy_init::{LazyPtr, LazyInit, PtrType};
use crate::{error::*, InterruptContext};
use crate::shim::*;
use crate::ticks::*;

pub type StaticSemaphore = StaticSemaphore_t;

#[macro_export]
macro_rules! static_semaphore {
    () => {{
      static mut STATIC_SEMAPHORE: ::core::mem::MaybeUninit<$crate::StaticSemaphore> = ::core::mem::MaybeUninit::uninit();
      static mut SEMAPHORE: ::core::mem::MaybeUninit::<Semaphore<Binary>> = ::core::mem::MaybeUninit::uninit();
      unsafe {
        $crate::Semaphore::new_binary_static(&mut STATIC_SEMAPHORE, &mut SEMAPHORE)
      }
    }};
    ($max:expr, $initial:expr) => {
      static mut STATIC_SEMAPHORE: ::core::mem::MaybeUninit<$crate::StaticSemaphore> = ::core::mem::MaybeUninit::uninit();
      static mut SEMAPHORE: ::core::mem::MaybeUninit::<Semaphore<Counting>> = ::core::mem::MaybeUninit::uninit();
      unsafe {
        $crate::Semaphore::Counting<$max, $initial>::new_counting_static(&mut STATIC_SEMAPHORE, &mut SEMAPHORE)
      }
    };
}

#[non_exhaustive]
pub struct Binary {}

#[non_exhaustive]
pub enum Counting<const MAX: u32, const INITIAL: u32> {}

#[derive(Debug)]
#[repr(transparent)]
pub struct SemaphoreHandle(<SemaphoreHandle_t as PtrType>::Type);

impl SemaphoreHandle {
  /// # Safety
  ///
  /// - `ptr` must point to a valid semaphore.
  /// - The semaphore must not be deleted for the lifetime `'a` of the returned `SemaphoreHandle`.
  pub unsafe fn from_ptr<'a>(ptr: SemaphoreHandle_t) -> &'a Self {
    &*ptr.cast::<Self>()
  }

  pub const fn as_ptr(&self) -> SemaphoreHandle_t {
    self as *const _ as SemaphoreHandle_t
  }
}

impl<T, S> Deref for Semaphore<T, S>
where
  (T, S): SemaphoreImpl,
{
  type Target = SemaphoreHandle;

  fn deref(&self) -> &Self::Target {
    // SAFETY: Self is an active semaphore which has not been deleted.
    unsafe { SemaphoreHandle::from_ptr(self.handle.as_ptr()) }
  }
}

/// A counting or binary semaphore
pub struct Semaphore<T, S = Dynamic>
where
  (T, S): SemaphoreImpl,
{
    handle: LazyPtr<(T, S), SemaphoreHandle_t>,
    storage: MaybeUninit<<(T, S) as SemaphoreImpl>::Storage>,
}

impl LazyInit<SemaphoreHandle_t> for (Binary, Dynamic) {
  fn init() -> Self::Ptr {
    let ptr = unsafe { xSemaphoreCreateBinary() };
    assert!(!ptr.is_null());
    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn destroy(ptr: Self::Ptr) {
    unsafe { vSemaphoreDelete(ptr.as_ptr()) }
  }
}

impl LazyInit<SemaphoreHandle_t> for (Binary, Static) {
  fn init() -> Self::Ptr {
    let static_semaphore: &'static mut MaybeUninit<StaticSemaphore> = todo!();

    let ptr = unsafe { xSemaphoreCreateBinaryStatic(static_semaphore.as_mut_ptr()) };
    assert!(!ptr.is_null());
    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn destroy(ptr: Self::Ptr) {
    unsafe { vSemaphoreDelete(ptr.as_ptr()) }
  }
}

impl<const MAX: u32, const INITIAL: u32> LazyInit<SemaphoreHandle_t> for (Counting<MAX, INITIAL>, Dynamic) {
  fn init() -> Self::Ptr {
    let ptr = unsafe { xSemaphoreCreateCounting(MAX, INITIAL) };
    assert!(!ptr.is_null());
    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn destroy(ptr: Self::Ptr) {
    unsafe { vSemaphoreDelete(ptr.as_ptr()) }
  }
}

impl<const MAX: u32, const INITIAL: u32> LazyInit<SemaphoreHandle_t> for (Counting<MAX, INITIAL>, Static) {
  fn init() -> Self::Ptr {
    todo!()
  }

  fn destroy(ptr: Self::Ptr) {
    unsafe { vSemaphoreDelete(ptr.as_ptr()) }
  }
}

unsafe impl<T: Send, S: Send> Send for Semaphore<T, S>
where
  (T, S): SemaphoreImpl,
{}
unsafe impl<T: Send, S> Sync for Semaphore<T, S>
where
  (T, S): SemaphoreImpl,
{}

pub enum Dynamic {}
pub enum Static {}

impl<S> Semaphore<Binary, S>
where
  (Binary, S): SemaphoreImpl,
{
  /// Create a new binary semaphore
  pub const fn new_binary() -> Self {
    Self { handle: LazyPtr::new(), storage: MaybeUninit::uninit() }
  }
}

impl<const MAX: u32, const INITIAL: u32> Semaphore<Counting<MAX, INITIAL>> {
  pub fn new_counting_static(
    static_semaphore: &'static mut MaybeUninit<StaticSemaphore>,
    semaphore: &'static mut MaybeUninit<Self>,
  ) -> &'static mut Self {
    assert!(INITIAL <= MAX);

    unsafe {
      let handle = xSemaphoreCreateCountingStatic(
        MAX, INITIAL,
        static_semaphore.as_mut_ptr(),
      );

      semaphore.write(Semaphore::from_raw_handle(handle))
    }
  }

  /// Create a new counting semaphore
  pub const fn new_counting() -> Self {
    assert!(INITIAL <= MAX);

    Self { handle: LazyPtr::new(), storage: MaybeUninit::uninit() }
  }
}

pub trait SemaphoreImpl: LazyInit<SemaphoreHandle_t> {
  type Storage = ();
}

impl SemaphoreImpl for (Binary, Dynamic) {}

impl SemaphoreImpl for (Binary, Static) {
  type Storage = StaticSemaphore_t;
}


impl<const MAX: u32, const INITIAL: u32> SemaphoreImpl for (Counting<MAX, INITIAL>, Dynamic) {}

impl<T> Semaphore<T, Dynamic>
where
  (T, Dynamic): SemaphoreImpl
{
  #[inline]
  pub unsafe fn from_raw_handle(handle: SemaphoreHandle_t) -> Self {
    Self { handle: LazyPtr::new_unchecked(handle), storage: MaybeUninit::uninit() }
  }

  #[inline]
  pub fn as_raw_handle(&self) -> SemaphoreHandle_t {
    self.handle.as_ptr()
  }
}

impl SemaphoreHandle {
  #[inline]
  pub fn give(&self) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreGive(self.as_ptr()) } {
      pdTRUE => Ok(()),
      errQUEUE_FULL => Err(FreeRtosError::QueueFull),
      _ => unreachable!(),
    }
  }

  #[inline]
  pub fn give_from_isr(&self, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreGiveFromISR(self.as_ptr(), ic.as_ptr()) } {
      pdTRUE => Ok(()),
      errQUEUE_FULL => Err(FreeRtosError::QueueFull),
      _ => unreachable!(),
    }
  }

  #[inline]
  pub fn take(&self, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreTake(self.as_ptr(), timeout.into().as_ticks()) } {
      pdTRUE => Ok(()),
      pdFALSE => Err(FreeRtosError::Timeout),
      _ => unreachable!(),
    }
  }

  pub fn take_from_isr(&self, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreTakeFromISR(self.as_ptr(), ic.as_ptr()) } {
      pdTRUE => Ok(()),
      pdFALSE => Err(FreeRtosError::Unavailable),
      _ => unreachable!(),
    }
  }

  /// Lock this semaphore in RAII fashion.
  pub fn lock(&self, timeout: impl Into<Ticks>) -> Result<SemaphoreGuard<'_>, FreeRtosError> {
      self.take(timeout)?;

      Ok(SemaphoreGuard { handle: self })
  }
}

/// Holds the lock to the semaphore until we are dropped
#[derive(Debug)]
pub struct SemaphoreGuard<'s> {
  handle: &'s SemaphoreHandle,
}

impl Drop for SemaphoreGuard<'_> {
  fn drop(&mut self) {
    let _ = self.handle.give();
  }
}

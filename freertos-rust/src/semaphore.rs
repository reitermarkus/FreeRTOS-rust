use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::pin::Pin;

use crate::alloc::{Dynamic, Static};
use crate::lazy_init::{LazyPtr, LazyInit, PtrType};
use crate::{error::*, InterruptContext};
use crate::shim::*;
use crate::ticks::*;

pub type StaticSemaphore = StaticSemaphore_t;

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
    _alloc_type: PhantomData<S>,
}

impl LazyInit<SemaphoreHandle_t> for (Binary, Dynamic) {
  fn init(_data: &UnsafeCell<MaybeUninit<()>>) -> Self::Ptr {
    let ptr = unsafe { xSemaphoreCreateBinary() };
    assert!(!ptr.is_null());
    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn destroy(ptr: Self::Ptr) {
    unsafe { vSemaphoreDelete(ptr.as_ptr()) }
  }
}

impl LazyInit<SemaphoreHandle_t> for (Binary, Static) {
  type Data = StaticSemaphore;

  fn init(data: &UnsafeCell<MaybeUninit<Self::Data>>) -> Self::Ptr {
    unsafe {
      let data = &mut *data.get();
      let ptr = xSemaphoreCreateBinaryStatic(data.as_mut_ptr());
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

impl<const MAX: u32, const INITIAL: u32> LazyInit<SemaphoreHandle_t> for (Counting<MAX, INITIAL>, Dynamic) {
  fn init(_data: &UnsafeCell<MaybeUninit<()>>) -> Self::Ptr {
    let ptr = unsafe { xSemaphoreCreateCounting(MAX, INITIAL) };
    assert!(!ptr.is_null());
    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn destroy(ptr: Self::Ptr) {
    unsafe { vSemaphoreDelete(ptr.as_ptr()) }
  }
}

impl<const MAX: u32, const INITIAL: u32> LazyInit<SemaphoreHandle_t> for (Counting<MAX, INITIAL>, Static) {
  type Data = StaticSemaphore;

  fn init(data: &UnsafeCell<MaybeUninit<Self::Data>>) -> Self::Ptr {
    unsafe {
      let data = &mut *data.get();
      let ptr = xSemaphoreCreateCountingStatic(MAX, INITIAL, data.as_mut_ptr());
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

unsafe impl<T: Send, A: Send> Send for Semaphore<T, A>
where
  (T, A): SemaphoreImpl,
{}
unsafe impl<T: Send, A> Sync for Semaphore<T, A>
where
  (T, A): SemaphoreImpl,
{}

impl<A> Semaphore<Binary, A>
where
  (Binary, A): SemaphoreImpl,
{
  /// Create a new binary semaphore.
  pub const fn new_binary() -> Self {
    Self { handle: LazyPtr::new(), _alloc_type: PhantomData }
  }
}

impl<const MAX: u32, const INITIAL: u32, A> Semaphore<Counting<MAX, INITIAL>, A>
where
  (Counting<MAX, INITIAL>, A): SemaphoreImpl,
{
  /// Create a new counting semaphore.
  pub const fn new_counting() -> Self {
    assert!(INITIAL <= MAX);

    Self { handle: LazyPtr::new(), _alloc_type: PhantomData }
  }
}

pub trait SemaphoreImpl: LazyInit<SemaphoreHandle_t> {}

impl<T, A> SemaphoreImpl for (T, A)
where
  (T, A): LazyInit<SemaphoreHandle_t>
{}
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
#[must_use = ""]
pub struct SemaphoreGuard<'s> {
  handle: &'s SemaphoreHandle,
}

impl Drop for SemaphoreGuard<'_> {
  fn drop(&mut self) {
    let _ = self.handle.give();
  }
}

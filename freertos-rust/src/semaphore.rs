use core::marker::PhantomData;
use core::mem::MaybeUninit;

use crate::lazy_init::{LazyPtr, LazyInit};
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

pub enum Binary {}

pub struct Counting<const MAX: u32, const INITIAL: u32> {
  _0: ()
}

/// A counting or binary semaphore
pub struct Semaphore<T: SemaphoreImpl> {
    handle: LazyPtr<T, SemaphoreHandle_t>,
}

impl LazyInit<SemaphoreHandle_t> for Binary {
  fn init() -> Self::Ptr {
    let ptr = unsafe { xSemaphoreCreateBinary() };
    assert!(!ptr.is_null());
    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn destroy(ptr: Self::Ptr) {
    unsafe { vSemaphoreDelete(ptr.as_ptr()) }
  }
}

impl<const MAX: u32, const INITIAL: u32> LazyInit<SemaphoreHandle_t> for Counting<MAX, INITIAL> {
  fn init() -> Self::Ptr {
    let ptr = unsafe { xSemaphoreCreateCounting(MAX, INITIAL) };
    assert!(!ptr.is_null());
    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn destroy(ptr: Self::Ptr) {
    unsafe { vSemaphoreDelete(ptr.as_ptr()) }
  }
}

unsafe impl<T: Send + SemaphoreImpl> Send for Semaphore<T> {}
unsafe impl<T: Send + SemaphoreImpl> Sync for Semaphore<T> {}

impl Semaphore<Binary> {
  /// Create a new binary semaphore
  pub const fn new_binary() -> Self {
    Self { handle: LazyPtr::new() }
  }

  pub fn new_binary_static(
    static_semaphore: &'static mut MaybeUninit<StaticSemaphore>,
    semaphore: &'static mut MaybeUninit<Self>,
  ) -> &'static mut Self {
    unsafe {
      let handle = xSemaphoreCreateBinaryStatic(
        static_semaphore.as_mut_ptr(),
      );

      semaphore.write(Semaphore::from_raw_handle(handle))
    }
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

    Self { handle: LazyPtr::new() }
  }
}

pub trait SemaphoreImpl: LazyInit<SemaphoreHandle_t> {}
impl SemaphoreImpl for Binary {}
impl<const MAX: u32, const INITIAL: u32> SemaphoreImpl for Counting<MAX, INITIAL> {}

impl<T: SemaphoreImpl> Semaphore<T> {
  #[inline]
  pub unsafe fn from_raw_handle(handle: SemaphoreHandle_t) -> Self {
    Self { handle: LazyPtr::new_unchecked(handle) }
  }

  #[inline]
  pub fn as_raw_handle(&self) -> SemaphoreHandle_t {
      self.handle.as_ptr()
  }

  #[inline]
  pub fn give(&self) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreGive(self.handle.as_ptr()) } {
      pdTRUE => Ok(()),
      errQUEUE_FULL => Err(FreeRtosError::QueueFull),
      _ => unreachable!(),
    }
  }

  #[inline]
  pub fn give_from_isr(&self, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreGiveFromISR(self.handle.as_ptr(), ic.as_ptr()) } {
      pdTRUE => Ok(()),
      errQUEUE_FULL => Err(FreeRtosError::QueueFull),
      _ => unreachable!(),
    }
  }

  #[inline]
  pub fn take(&self, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreTake(self.handle.as_ptr(), timeout.into().as_ticks()) } {
      pdTRUE => Ok(()),
      pdFALSE => Err(FreeRtosError::Timeout),
      _ => unreachable!(),
    }
  }

  pub fn take_from_isr(&self, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
    match unsafe { xSemaphoreTakeFromISR(self.handle.as_ptr(), ic.as_ptr()) } {
      pdTRUE => Ok(()),
      pdFALSE => Err(FreeRtosError::Unavailable),
      _ => unreachable!(),
    }
  }

  /// Lock this semaphore in RAII fashion.
  pub fn lock(&self, timeout: impl Into<Ticks>) -> Result<SemaphoreGuard<'_>, FreeRtosError> {
      self.take(timeout)?;

      Ok(SemaphoreGuard { handle: self.handle.as_ptr(), _lifetime: PhantomData })
  }
}

/// Holds the lock to the semaphore until we are dropped
#[derive(Debug)]
pub struct SemaphoreGuard<'s> {
    handle: SemaphoreHandle_t,
    _lifetime: PhantomData<&'s ()>
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        unsafe { xSemaphoreGive(self.handle) };
    }
}

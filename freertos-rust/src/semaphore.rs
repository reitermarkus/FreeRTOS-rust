use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use crate::lazy_init::{LazyPtr, LazyInit};
use crate::{base::*, InterruptContext};
use crate::shim::*;
use crate::units::*;

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
    handle: LazyPtr<T>,
}

impl LazyInit for Binary {
  fn init() -> NonNull<CVoid> {
    let ptr = unsafe { freertos_rs_semaphore_create_binary() };
    assert!(!ptr.is_null());
    unsafe { NonNull::new_unchecked(ptr) }
  }

  fn destroy(ptr: NonNull<CVoid>) {
    unsafe { freertos_rs_semaphore_delete(ptr.as_ptr()) }
  }
}

impl<const MAX: u32, const INITIAL: u32> LazyInit for Counting<MAX, INITIAL> {
  fn init() -> NonNull<CVoid> {
    let ptr = unsafe { freertos_rs_semaphore_create_counting(MAX, INITIAL) };
    assert!(!ptr.is_null());
    unsafe { NonNull::new_unchecked(ptr) }
  }

  fn destroy(ptr: NonNull<CVoid>) {
    unsafe { freertos_rs_semaphore_delete(ptr.as_ptr()) }
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
      let handle = freertos_rs_semaphore_create_binary_static(
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
      let handle = freertos_rs_semaphore_create_counting_static(
        MAX, INITIAL,
        static_semaphore.as_mut_ptr(),
      );

      semaphore.write(Semaphore::from_raw_handle(handle))
    }
  }

  /// Create a new counting semaphore
  pub const fn new_counting(max: u32, initial: u32) -> Self {
    assert!(INITIAL <= MAX);

    Self { handle: LazyPtr::new() }
  }
}

pub trait SemaphoreImpl: LazyInit {}
impl SemaphoreImpl for Binary {}
impl<const MAX: u32, const INITIAL: u32> SemaphoreImpl for Counting<MAX, INITIAL> {}

impl<T: SemaphoreImpl> Semaphore<T> {
  #[inline]
  pub unsafe fn from_raw_handle(handle: FreeRtosSemaphoreHandle) -> Self {
    Self { handle: LazyPtr::new_unchecked(handle) }
  }

  #[inline]
  pub fn as_raw_handle(&self) -> FreeRtosSemaphoreHandle {
      self.handle.as_ptr()
  }

  #[inline]
  pub fn give(&self) -> Result<(), FreeRtosError> {
    unsafe { semaphore_give(self.handle.as_ptr()) }
  }

  #[inline]
  pub fn give_from_isr(&self, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
    unsafe { semaphore_give_from_isr(self.handle.as_ptr(), ic) }
  }

  #[inline]
  pub fn take<D: DurationTicks>(&self, max_wait: D) -> Result<(), FreeRtosError> {
    unsafe { semaphore_take(self.handle.as_ptr(), max_wait) }
  }

  /// Lock this semaphore in a RAII fashion
  pub fn lock<D: DurationTicks>(&self, max_wait: D) -> Result<SemaphoreGuard<'_>, FreeRtosError> {
      self.take(max_wait)?;

      Ok(SemaphoreGuard { handle: self.handle.as_ptr(), _lifetime: PhantomData })
  }
}

/// Holds the lock to the semaphore until we are dropped
#[derive(Debug)]
pub struct SemaphoreGuard<'s> {
    handle: *mut CVoid,
    _lifetime: PhantomData<&'s ()>
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        let _ = unsafe { semaphore_give(self.handle) };
    }
}

unsafe fn semaphore_give(handle: *mut CVoid) -> Result<(), FreeRtosError> {
    let res = freertos_rs_semaphore_give(handle);

    if res != 0 {
      return Err(FreeRtosError::QueueFull);
    }

    Ok(())
}

unsafe fn semaphore_give_from_isr(handle: *mut CVoid, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
    let res = freertos_rs_semaphore_give_from_isr(handle, ic.x_higher_priority_task_woken());

    if res != 0 {
      return Err(FreeRtosError::QueueFull);
    }

    Ok(())
}

unsafe fn semaphore_take<D: DurationTicks>(handle: *mut CVoid, max_wait: D) -> Result<(), FreeRtosError> {
    let res = freertos_rs_semaphore_take(handle, max_wait.to_ticks());

    if res != 0 {
        return Err(FreeRtosError::Timeout);
    }

    Ok(())
}

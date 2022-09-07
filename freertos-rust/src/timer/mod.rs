//! FreeRTOS timer primitives.

use core::cell::UnsafeCell;
use core::ffi::CStr;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::ops::Deref;
use core::pin::Pin;
use core::ptr;

use alloc2::boxed::Box;

use crate::alloc::{Dynamic, Static};
use crate::lazy_init::{LazyInit, LazyPtr};
use crate::shim::*;
use crate::ticks::Ticks;
use crate::task::TaskHandle;

mod builder;
pub use builder::TimerBuilder;
mod handle;
pub use handle::TimerHandle;

/// A software timer.
///
/// Note that all operations on a timer are processed by a FreeRTOS internal task
/// that receives messages in a queue. Every operation has an associated waiting time
/// for that queue to get unblocked.
#[must_use = "timer will be deleted immediately if unused"]
pub struct Timer<'f, A = Dynamic>
where
  Self: LazyInit,
{
  alloc_type: PhantomData<A>,
  handle: LazyPtr<Self>,
}

unsafe impl<'f, A> Send for Timer<'f, A>
where
  Self: LazyInit,
{}
unsafe impl<'f, A> Sync for Timer<'f, A>
where
  Self: LazyInit,
{}

impl Timer<'_> {
  /// Stack size of the timer daemon task.
  pub const STACK_SIZE: u16 = configTIMER_TASK_STACK_DEPTH;

  /// Get the handle for the timer daemon task.
  #[inline]
  pub fn daemon_task() -> &'static TaskHandle {
    unsafe { TaskHandle::from_ptr(xTimerGetTimerDaemonTaskHandle()) }
  }

  /// Create a new timer builder.
  pub const fn new() -> TimerBuilder<'static> {
    TimerBuilder {
      name: None,
      period: Ticks::new(0),
      auto_reload: true,
    }
  }
}

impl<A> Timer<'static, A>
where
  Self: LazyInit,
  <Self as LazyInit>::Data: Sized,
{
  /// Detach this timer from Rust's memory management. The timer will still be active and
  /// will consume the memory.
  ///
  /// Can be used for timers that will never be changed and don't need to stay in scope.
  ///
  /// This is the same as calling [`mem::forget`], but self-documenting.
  pub unsafe fn detach(self) {
    mem::forget(self);
  }
}

impl<'f, A> Deref for Timer<'f, A>
where
  Self: LazyInit<Handle = TimerHandle_t>,
{
  type Target = TimerHandle;

  fn deref(&self) -> &Self::Target {
    // Ensure timer is initialized.
    let handle = self.handle.as_ptr();
    unsafe { TimerHandle::from_ptr(handle) }
  }
}

#[doc(hidden)]
pub struct TimerMeta<'f, F> {
  name: Option<&'f CStr>,
  period: TickType_t,
  auto_reload: bool,
  callback: F,
}

impl<'f> LazyInit for Timer<'f, Dynamic> {
  type Storage = ();
  type Handle = TimerHandle_t;
  type Data = TimerMeta<'f, Pin<Box<Box<dyn Fn(&TimerHandle) + Send + 'f>>>>;

  fn init(data: &UnsafeCell<Self::Data>, _storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
    let data = unsafe { &mut *data.get() };
    let TimerMeta { name, period, auto_reload, callback } = data;

    let callback_ptr: *mut Box<dyn Fn(&TimerHandle) + Send + 'f> = &mut **callback;

    extern "C" fn timer_callback(ptr: TimerHandle_t) -> () {
      unsafe {
        let handle = TimerHandle::from_ptr(ptr);

        let callback_ptr = pvTimerGetTimerID(ptr);
        let callback: &mut Box<dyn Fn(&TimerHandle)> = &mut *callback_ptr.cast();
        callback(handle);
      }
    }

    let ptr = unsafe {
      xTimerCreate(
        name.as_deref().map(|n| n.as_ptr()).unwrap_or(ptr::null()),
        *period,
        if *auto_reload { pdTRUE } else { pdFALSE } as _,
        callback_ptr.cast(),
        Some(timer_callback),
      )
    };
    assert!(!ptr.is_null());

    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn destroy(ptr: Self::Ptr, storage: &mut MaybeUninit<Self::Storage>) {
      unsafe { xTimerDelete(ptr.as_ptr(), portMAX_DELAY) };
      unsafe { storage.assume_init_drop() };
  }
}

impl LazyInit for Timer<'static, Static> {
  type Storage = StaticTimer_t;
  type Handle = TimerHandle_t;
  type Data = TimerMeta<'static, fn(&TimerHandle)>;

  fn init(data: &UnsafeCell<Self::Data>, storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
    let data = unsafe { &mut *data.get() };
    let TimerMeta { name, period, auto_reload, callback } = data;

    let callback: fn(&TimerHandle) = *callback;
    let callback_ptr = callback as *mut _;

    extern "C" fn timer_callback(ptr: TimerHandle_t) -> () {
      unsafe {
        let handle = TimerHandle::from_ptr(ptr);

        let callback_ptr = pvTimerGetTimerID(ptr);
        let callback: fn(&TimerHandle) = mem::transmute(callback_ptr);
        callback(handle);
      }
    }

    let ptr = unsafe {
      let storage = &mut *storage.get();

      xTimerCreateStatic(
        name.as_deref().map(|n| n.as_ptr()).unwrap_or(ptr::null()),
        *period,
        if *auto_reload { pdTRUE } else { pdFALSE } as _,
        callback_ptr,
        Some(timer_callback),
        storage.as_mut_ptr(),
      )
    };
    assert!(!ptr.is_null());

    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn cancel_init_supported() -> bool {
    false
  }

  fn destroy(ptr: Self::Ptr, storage: &mut MaybeUninit<Self::Storage>) {
    unsafe { xTimerDelete(ptr.as_ptr(), portMAX_DELAY) };
    unsafe { storage.assume_init_drop() };
  }
}

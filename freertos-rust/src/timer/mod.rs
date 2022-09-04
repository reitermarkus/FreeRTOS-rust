use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem;
use core::ops::Deref;
use core::pin::Pin;
use core::ptr;
use core::ptr::NonNull;

use alloc2::{
  ffi::CString,
  boxed::Box,
};

use crate::alloc::Dynamic;
use crate::error::FreeRtosError;
use crate::lazy_init::LazyInit;
use crate::lazy_init::LazyPtr;
use crate::shim::*;
use crate::ticks::*;
use crate::Task;
use crate::lazy_init::PtrType;

mod builder;
pub use builder::TimerBuilder;
mod handle;
pub use handle::TimerHandle;

/// A FreeRTOS software timer.
///
/// Note that all operations on a timer are processed by a FreeRTOS internal task
/// that receives messages in a queue. Every operation has an associated waiting time
/// for that queue to get unblocked.
pub struct Timer<'f, A = Dynamic>
where
  Self: LazyInit,
{
  alloc_type: PhantomData<A>,
  handle: LazyPtr<Self, ()>,
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
  /// Stack size of the timer task.
  pub const STACK_SIZE: u16 = configTIMER_TASK_STACK_DEPTH;

  pub fn daemon_task() -> Task {
    unsafe { Task::from_raw_handle(xTimerGetTimerDaemonTaskHandle()) }
  }

  /// Create a new timer builder.
  pub const fn new(period: impl Into<Ticks>) -> TimerBuilder<'static> {
    TimerBuilder {
      name: None,
      period: period.into(),
      auto_reload: true,
    }
  }
}

impl<'f> Timer<'f, Dynamic> {
  fn spawn<F>(
      name: Option<&str>,
      period_tick: Ticks,
      auto_reload: bool,
      callback: F,
  ) -> Result<Self, FreeRtosError>
  where
      F: Fn(&TimerHandle) + Send + 'f,
  {
    let name = match name.map(CString::new) {
      None => None,
      Some(Ok(name)) => Some(Pin::new(name)),
      Some(_) => return Err(FreeRtosError::StringConversionError)
    };

    let meta = TimerMeta {
      name,
      period: period_tick.as_ticks(),
      auto_reload,
      callback: Box::pin(Box::new(callback)),
    };

    Ok(Timer {
      alloc_type: PhantomData,
      handle: LazyPtr::new_with_storage((), meta),
    })
  }
}

impl<A> Timer<'static, A>
where
  Self: LazyInit,
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

extern "C" fn timer_callback(ptr: TimerHandle_t) -> () {
  unsafe {
    let handle = TimerHandle::from_ptr(ptr);

    let callback_ptr = pvTimerGetTimerID(ptr);
    let callback = &mut *callback_ptr.cast::<Box<dyn Fn(&TimerHandle)>>();
    callback(handle);
  }
}

pub struct TimerMeta<'f> {
  name: Option<Pin<CString>>,
  period: TickType_t,
  auto_reload: bool,
  callback: Pin<Box<Box<dyn Fn(&TimerHandle) + Send + 'f>>>,
}

impl<'f> LazyInit for Timer<'f, Dynamic> {
  type Storage = TimerMeta<'f>;
  type Handle = TimerHandle_t;

  fn init(storage: &UnsafeCell<mem::MaybeUninit<Self::Storage>>) -> NonNull<<Self::Handle as PtrType>::Type> {
    let storage = unsafe { &mut *storage.get() };
    let TimerMeta { name, period, auto_reload, callback } = unsafe { storage.assume_init_mut() };

    let callback_ptr: *mut Box<dyn Fn(&TimerHandle) + Send + 'f> = &mut **callback;

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

  fn destroy(ptr: NonNull<<Self::Handle as PtrType>::Type>, storage: &mut mem::MaybeUninit<Self::Storage>) {
      unsafe { xTimerDelete(ptr.as_ptr(), portMAX_DELAY) };
      unsafe { storage.assume_init_drop() };
  }
}

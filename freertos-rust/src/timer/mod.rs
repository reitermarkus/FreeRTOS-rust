//! FreeRTOS timer primitives.

use core::{ffi::CStr, ops::Deref};

#[cfg(freertos_feature = "dynamic_allocation")]
use alloc2::boxed::Box;

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
pub struct Timer<'n> {
  handle: TimerHandle_t,
  #[cfg(freertos_feature = "dynamic_allocation")]
  #[allow(unused)]
  callback: Option<Box<Box<dyn Fn(&TimerHandle)>>>,
  #[allow(unused)]
  name: Option<&'n CStr>,
}

unsafe impl<'n> Send for Timer<'n> {}
unsafe impl<'n> Sync for Timer<'n> {}

impl<'n> Timer<'n> {
  /// Stack size of the timer daemon task.
  pub const STACK_SIZE: u16 = configTIMER_TASK_STACK_DEPTH;

  /// Get the handle for the timer daemon task.
  #[inline]
  pub fn daemon_task() -> &'static TaskHandle {
    unsafe {
      let ptr = xTimerGetTimerDaemonTaskHandle();
      assert!(!ptr.is_null());
      TaskHandle::from_ptr(ptr)
    }
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

impl<'n> Deref for Timer<'n> {
  type Target = TimerHandle;

  fn deref(&self) -> &Self::Target {
    unsafe { TimerHandle::from_ptr(self.handle) }
  }
}

impl<'n> Drop for Timer<'n> {
  fn drop(&mut self) {
    unsafe { xTimerDelete(self.as_ptr(), portMAX_DELAY) };
  }
}

/// A statically allocated software timer.
#[must_use = "timer will be deleted immediately if unused"]
pub struct StaticTimer {
  data: StaticTimer_t,
}

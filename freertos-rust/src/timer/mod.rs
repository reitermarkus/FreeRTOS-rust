//! FreeRTOS timer primitives.

use core::{ops::Deref, ptr};

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
pub struct Timer {
  handle: TimerHandle_t,
}

unsafe impl Send for Timer {}
unsafe impl Sync for Timer {}

impl Timer {
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

impl Deref for Timer {
  type Target = TimerHandle;

  fn deref(&self) -> &Self::Target {
    unsafe { TimerHandle::from_ptr(self.handle) }
  }
}

impl Drop for Timer {
  fn drop(&mut self) {
    unsafe {
      let callback = pvTimerGetTimerID(self.as_ptr());
      if !callback.is_null() {
        let callback = Box::from_raw(callback as *mut Box<dyn FnOnce(&TimerHandle)>);
        drop(callback);

        vTimerSetTimerID(self.as_ptr(), ptr::null_mut());
      }

      xTimerDelete(self.as_ptr(), portMAX_DELAY);
    }
  }
}

/// A statically allocated software timer.
#[must_use = "timer will be deleted immediately if unused"]
pub struct StaticTimer {
  data: StaticTimer_t,
}

unsafe impl Send for StaticTimer {}
unsafe impl Sync for StaticTimer {}

impl Deref for StaticTimer {
  type Target = TimerHandle;

  fn deref(&self) -> &Self::Target {
    unsafe { TimerHandle::from_ptr(ptr::addr_of!(self.data) as TimerHandle_t) }
  }
}

impl Drop for StaticTimer {
  fn drop(&mut self) {
    unsafe { xTimerDelete(self.as_ptr(), portMAX_DELAY) };
  }
}

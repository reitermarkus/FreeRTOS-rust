use embedded_hal::blocking::delay::DelayMs;

use crate::ffi::TickType_t;
use crate::shim::{vTaskDelay, vTaskDelayUntil, portTICK_PERIOD_MS};
use crate::task::Scheduler;
use crate::ticks::Ticks;

/// Delay the current task by the given duration, minus the
/// time that was spent processing the last wakeup loop.
pub struct CurrentTask {
    last_wake_time: TickType_t,
}

impl CurrentTask {
  /// Create a new helper, marking the current time as the start of the
  /// next measurement.
  pub fn new() -> CurrentTask {
    CurrentTask {
      last_wake_time: Scheduler::tick_count().as_ticks(),
    }
  }

  /// Delay the execution of the current task.
  pub fn delay(&mut self, delay: impl Into<Ticks>) {
    unsafe { vTaskDelay(delay.into().as_ticks()) }
  }

  /// Delay the execution of the current task by the given duration,
  /// minus the time spent in this task since the last delay.
  pub fn delay_until(&mut self, delay: impl Into<Ticks>) {
    unsafe {
      vTaskDelayUntil(&mut self.last_wake_time, delay.into().as_ticks())
    }
  }
}

impl DelayMs<u32> for CurrentTask {
  fn delay_ms(&mut self, ms: u32) {
    // Round up so the delay is at least the given amount.
    self.delay((ms + portTICK_PERIOD_MS - 1) / portTICK_PERIOD_MS)
  }
}

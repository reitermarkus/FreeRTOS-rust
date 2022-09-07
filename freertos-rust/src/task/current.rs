use embedded_hal::blocking::delay::DelayMs;

use crate::{TaskHandle, FreeRtosError};
use crate::ffi::TickType_t;
use crate::shim::{
  vTaskDelay,
  vTaskDelayUntil,
  portTICK_PERIOD_MS,
  pdTRUE,
  pdFALSE,
  ulTaskNotifyTake, xTaskNotifyWait, xTaskGetCurrentTaskHandle, pdPASS,
};
use crate::task::Scheduler;
use crate::ticks::Ticks;

/// Delay the current task by the given duration, minus the
/// time that was spent processing the last wakeup loop.
pub struct CurrentTask {
  handle: &'static TaskHandle,
  last_wake_time: TickType_t,
}

impl CurrentTask {
  /// Create a new helper, marking the current time as the start of the
  /// next measurement.
  ///
  /// # Safety
  ///
  /// - Must be called from inside a task function.
  /// - Only one mutable `CurrentTask` instance may exist for a single task.
  pub unsafe fn new_unchecked() -> CurrentTask {
    CurrentTask {
      handle: TaskHandle::from_ptr(xTaskGetCurrentTaskHandle()),
      last_wake_time: Scheduler::tick_count().into(),
    }
  }

  /// Take a notification and either clear the notification value or decrement it by one.
  pub fn take_notification(&mut self, clear: bool, timeout: impl Into<Ticks>) -> u32 {
    unsafe {
      ulTaskNotifyTake(if clear { pdTRUE } else { pdFALSE }, timeout.into().into())
    }
  }

  /// Wait for a notification.
  ///
  /// Clears the bits set in `clear_on_entry` after entering and clears the
  /// bits set in `clear_on_exit` before returning from the function.
  pub fn wait_for_notification(
    &mut self,
    clear_on_entry: u32,
    clear_on_exit: u32,
    timeout: impl Into<Ticks>,
  ) -> Result<u32, FreeRtosError> {
    let mut val = 0;
    match unsafe {
      xTaskNotifyWait(
        clear_on_entry,
        clear_on_exit,
        &mut val as *mut _,
        timeout.into().into(),
      )
    } {
      pdPASS => Ok(val),
      _ => Err(FreeRtosError::Timeout),
    }
  }

  /// Clear pending notifications for this task.
  ///
  /// Returns whether a pending notification was cleared.
  pub fn clear_notification(&mut self) -> bool {
    self.handle.clear_notification()
  }

  /// Delay the execution of the current task.
  pub fn delay(&mut self, delay: impl Into<Ticks>) {
    unsafe { vTaskDelay(delay.into().into()) }
  }

  /// Delay the execution of the current task by the given duration,
  /// minus the time spent in this task since the last delay.
  pub fn delay_until(&mut self, delay: impl Into<Ticks>) {
    unsafe {
      vTaskDelayUntil(&mut self.last_wake_time, delay.into().into())
    }
  }
}

impl DelayMs<u32> for CurrentTask {
  fn delay_ms(&mut self, ms: u32) {
    // Round up so the delay is at least the given amount.
    self.delay((ms + portTICK_PERIOD_MS - 1) / portTICK_PERIOD_MS)
  }
}

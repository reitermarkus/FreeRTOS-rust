use core::{ffi::CStr, str, fmt, ptr};

use crate::FreeRtosError;
use crate::InterruptContext;
use crate::shim::{pdTRUE, pcTaskGetName, configTASK_NOTIFICATION_ARRAY_ENTRIES};
use crate::shim::uxTaskGetStackHighWaterMark;
use crate::shim::vTaskResume;
use crate::shim::vTaskSuspend;
use crate::shim::xTaskNotifyFromISR;
use crate::shim::xTaskNotifyStateClear;
use crate::task::TaskNotification;
use crate::lazy_init::PtrType;
use crate::ffi::TaskHandle_t;
use crate::shim::freertos_rs_task_notify_indexed;
use crate::shim::freertos_rs_task_notify_indexed_from_isr;
use crate::shim::pdPASS;
use crate::shim::xTaskNotify;

/// A handle for managing a task.
///
/// See [`Task`](crate::task::Task) for the preferred owned version.
///
/// `TaskHandle` is compatible with a raw FreeRTOS task.
pub struct TaskHandle(<TaskHandle_t as PtrType>::Type);

impl fmt::Debug for TaskHandle {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.as_ptr().fmt(f)
  }
}

impl TaskHandle {
  /// Create a a `TaskHandle` from a raw handle.
  ///
  /// # Safety
  ///
  /// - `ptr` must point to a valid task.
  /// - The task must not be deleted for the lifetime `'a` of the returned `TaskHandle`.
  pub const unsafe fn from_ptr<'a>(ptr: TaskHandle_t) -> &'a Self {
    &*ptr.cast::<Self>()
  }

  /// Get the raw task handle.
  pub const fn as_ptr(&self) -> TaskHandle_t {
    ptr::addr_of!(self.0).cast_mut()
  }

  /// Get the name of this task.
  pub fn name(&self) -> &str {
    unsafe {
      let task_name = pcTaskGetName(self.as_ptr());
      task_name.as_ref()
      .map(|n| CStr::from_ptr(n))
      .map(|n| match n.to_str() {
        Ok(n) => n,
        Err(err) => str::from_utf8_unchecked(&n.to_bytes()[..err.valid_up_to()]),
      })
      .unwrap_or_default()
    }
  }

  /// Suspend execution of the task.
  pub fn suspend(&self) {
    unsafe { vTaskSuspend(self.as_ptr()) }
  }

  /// Resume execution of the task.
  pub fn resume(&self) {
    unsafe { vTaskResume(self.as_ptr()) }
  }

  /// Forcibly set the notification value for this task.
  ///
  /// This is the same as sending `TaskNotification::OverwriteValue(val)` using `notify`,
  /// which cannot fail.
  pub fn set_notification_value(&self, val: u32) {
    // NOTE: Overwriting never fails.
    let _ = self.notify(TaskNotification::OverwriteValue(val));
  }

  /// Send a notification to this task.
  ///
  /// # Errors
  ///
  /// This can only fail when sending [`TaskNotification::SetValue`] and
  /// the task already has pending notifications.
  pub fn notify(&self, notification: TaskNotification) -> Result<(), FreeRtosError> {
    let (value, action) = notification.to_freertos();

    match unsafe { xTaskNotify(self.as_ptr(), value, action) } {
      pdPASS => Ok(()),
      _ => Err(FreeRtosError::QueueFull),
    }
  }

  /// Notify this task with the given index.
  ///
  /// # Errors
  ///
  /// This can only fail when sending [`TaskNotification::SetValue`] and
  /// the task already has pending notifications.
  ///
  /// # Panics
  ///
  /// This panics if `index` is not within \[0, `configTASK_NOTIFICATION_ARRAY_ENTRIES`\).
  pub fn notify_indexed(&self, index: u32, notification: TaskNotification) -> Result<(), FreeRtosError> {
    assert!(index < configTASK_NOTIFICATION_ARRAY_ENTRIES);

    let (value, action) = notification.to_freertos();

    match unsafe { freertos_rs_task_notify_indexed(self.as_ptr(), index, value, action) } {
      pdPASS => Ok(()),
      _ => Err(FreeRtosError::QueueFull),
    }
  }

    /// Notify this task from an interrupt.
    pub fn notify_from_isr(
        &self,
        notification: TaskNotification,
        ic: &mut InterruptContext,
    ) -> Result<(), FreeRtosError> {
      let (value, action) = notification.to_freertos();

      match unsafe {
        xTaskNotifyFromISR(
          self.as_ptr(),
          value,
          action,
          ic.as_ptr(),
        )
      } {
        pdPASS => Ok(()),
        _ => Err(FreeRtosError::QueueFull),
      }
    }

    /// Notify this task from an interrupt with the given index.
    ///
    /// # Errors
    ///
    /// This can only fail when sending [`TaskNotification::SetValue`] and
    /// the task already has pending notifications.
    ///
    /// # Panics
    ///
    /// This panics if `index` is not within \[0, `configTASK_NOTIFICATION_ARRAY_ENTRIES`\).
    pub fn notify_indexed_from_isr(
      &self,
      index: u32,
      notification: TaskNotification,
      ic: &mut InterruptContext,
    ) -> Result<(), FreeRtosError> {
      assert!(index < configTASK_NOTIFICATION_ARRAY_ENTRIES);

      let (value, action) = notification.to_freertos();

      match unsafe {
        freertos_rs_task_notify_indexed_from_isr(
          self.as_ptr(),
          index,
          value,
          action,
          ic.as_ptr(),
        )
      } {
        pdPASS => Ok(()),
        _ => Err(FreeRtosError::QueueFull),
      }
    }

  /// Get the minimum amount of stack that was ever left on this task.
  pub fn stack_high_water_mark(&self) -> u32 {
    unsafe { uxTaskGetStackHighWaterMark(self.as_ptr()) }
  }

  /// Clear pending notifications for this task.
  ///
  /// Returns whether a pending notification was cleared.
  pub fn clear_notification(&self) -> bool {
    unsafe { xTaskNotifyStateClear(self.as_ptr()) == pdTRUE }
  }
}

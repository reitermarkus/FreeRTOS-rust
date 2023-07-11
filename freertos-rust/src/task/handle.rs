use core::{ffi::CStr, str, fmt, ptr};

use crate::FreeRtosError;
use crate::InterruptContext;
use crate::ffi::Pointee;
use crate::shim::{pdTRUE, pcTaskGetName, configTASK_NOTIFICATION_ARRAY_ENTRIES};
use crate::shim::uxTaskGetStackHighWaterMark;
use crate::shim::vTaskResume;
use crate::shim::vTaskSuspend;
use crate::shim::xTaskNotifyFromISR;
use crate::shim::xTaskNotifyStateClear;
use crate::task::TaskNotification;
use crate::ffi::TaskHandle_t;
use crate::shim::freertos_rs_task_notify_indexed;
use crate::shim::freertos_rs_task_notify_indexed_from_isr;
use crate::shim::pdPASS;
use crate::shim::xTaskNotify;
use crate::shim::{uxTaskGetTaskNumber, vTaskSetTaskNumber};

/// A handle for managing a task.
///
/// See [`Task`](crate::task::Task) for the preferred owned version.
///
/// This type is compatible with a raw FreeRTOS [`TaskHandle_t`].
#[repr(transparent)]
pub struct TaskHandle(Pointee<TaskHandle_t>);

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
  #[inline]
  pub const unsafe fn from_ptr<'a>(ptr: TaskHandle_t) -> &'a Self {
    debug_assert!(!ptr.is_null());
    &*ptr.cast()
  }

  /// Get the raw task handle.
  #[inline]
  pub const fn as_ptr(&self) -> TaskHandle_t {
    ptr::addr_of!(self.0).cast_mut()
  }

  /// Get the number of this task.
  #[inline]
  pub fn number(&self) -> usize {
    unsafe { uxTaskGetTaskNumber(self.as_ptr()) as usize }
  }

  /// Set the number of this task.
  #[inline]
  pub fn set_number(&self, n: usize) {
    unsafe { vTaskSetTaskNumber(self.as_ptr(), n as _) }
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
  #[inline]
  pub fn suspend(&self) {
    unsafe { vTaskSuspend(self.as_ptr()) }
  }

  /// Resume execution of the task.
  #[inline]
  pub fn resume(&self) {
    unsafe { vTaskResume(self.as_ptr()) }
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
  pub fn notify_indexed(&self, index: usize, notification: TaskNotification) -> Result<(), FreeRtosError> {
    assert!(index < configTASK_NOTIFICATION_ARRAY_ENTRIES as _);

    let (value, action) = notification.to_freertos();

    match unsafe { freertos_rs_task_notify_indexed(self.as_ptr(), index as _, value, action) } {
      pdPASS => Ok(()),
      _ => Err(FreeRtosError::QueueFull),
    }
  }

  /// Notify this task from an interrupt.
  pub fn notify_from_isr(
    &self,
    notification: TaskNotification,
    ic: &InterruptContext,
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
    index: usize,
    notification: TaskNotification,
    ic: &InterruptContext,
  ) -> Result<(), FreeRtosError> {
    assert!(index < configTASK_NOTIFICATION_ARRAY_ENTRIES as _);

    let (value, action) = notification.to_freertos();

    match unsafe {
      freertos_rs_task_notify_indexed_from_isr(
        self.as_ptr(),
        index as _,
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
  #[inline]
  pub fn stack_high_water_mark(&self) -> usize {
    unsafe { uxTaskGetStackHighWaterMark(self.as_ptr()) as _ }
  }

  /// Clear pending notifications for this task.
  ///
  /// Returns whether a pending notification was cleared.
  #[inline]
  pub fn clear_notification(&self) -> bool {
    unsafe { xTaskNotifyStateClear(self.as_ptr()) == pdTRUE }
  }
}

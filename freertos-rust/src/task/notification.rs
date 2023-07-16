use crate::shim::{
  eNotifyAction,
  eNotifyAction_eNoAction,
  eNotifyAction_eSetBits,
  eNotifyAction_eIncrement,
  eNotifyAction_eSetValueWithOverwrite,
  eNotifyAction_eSetValueWithoutOverwrite,
};

/// Notification to be sent to a task.
#[derive(Debug, Clone, Copy)]
pub enum TaskNotification {
  /// Send the event and unblock the task without changing the notification value.
  NoAction,
  /// Perform a logical or with the task's notification value.
  SetBits(u32),
  /// Increment the notification value by one.
  Increment,
  /// Unconditionally set the notification value.
  OverwriteValue(u32),
  /// Try setting the notification value to this value.
  ///
  /// # Errors
  ///
  /// This will fail if the task already has pending notifications.
  SetValue(u32),
}

impl TaskNotification {
  pub(crate) fn to_freertos(self) -> (u32, eNotifyAction) {
    match *self {
      TaskNotification::NoAction => (0, eNotifyAction_eNoAction),
      TaskNotification::SetBits(v) => (v, eNotifyAction_eSetBits),
      TaskNotification::Increment => (0, eNotifyAction_eIncrement),
      TaskNotification::OverwriteValue(v) => (v, eNotifyAction_eSetValueWithOverwrite),
      TaskNotification::SetValue(v) => (v, eNotifyAction_eSetValueWithoutOverwrite),
    }
  }
}

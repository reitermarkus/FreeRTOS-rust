use core::fmt;

use crate::shim::UBaseType_t;
use crate::shim::configMAX_PRIORITIES;

/// Task execution priority.
///
/// Low priority numbers denote low priority tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskPriority {
  priority: u8,
}

impl TaskPriority {
  /// Create a new `TaskPriority`.
  ///
  /// Returns `None` if `priority` is greater or equal to `configMAX_PRIORITIES`.
  pub const fn new(priority: u8) -> Option<Self> {
    if priority >= configMAX_PRIORITIES {
      return None
    }

    Some(Self { priority })
  }

  /// Create a new `TaskPriority` without checking whether it is valid.
  pub const unsafe fn new_unchecked(priority: u8) -> Self {
    Self { priority }
  }

  pub(crate) fn to_freertos(&self) -> UBaseType_t {
    self.priority.into()
  }
}

impl fmt::Display for TaskPriority {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.priority.fmt(f)
  }
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct PriorityOverflow;

impl TryFrom<u8> for TaskPriority {
  type Error = PriorityOverflow;

  fn try_from(priority: u8) -> Result<Self, Self::Error> {
    if let Some(priority) = Self::new(priority) {
      return Ok(priority)
    }

    Err(PriorityOverflow)
  }
}

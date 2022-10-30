use core::fmt;

/// Basic error type for the library.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FreeRtosError {
  /// Memory allocation failed.
  OutOfMemory,
  /// Timeout during a blocking operation.
  Timeout,
  /// Not available.
  Unavailable,
  /// No more space in queue.
  QueueFull,
  /// Task does not exist.
  TaskNotFound,
}

impl fmt::Display for FreeRtosError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::OutOfMemory => "out of memory",
      Self::Timeout => "timed out",
      Self::Unavailable => "unavailable",
      Self::QueueFull => "queue full",
      Self::TaskNotFound => "task not found",
    }.fmt(f)
  }
}

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

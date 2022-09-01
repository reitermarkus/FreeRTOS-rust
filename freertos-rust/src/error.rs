/// Basic error type for the library.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FreeRtosError {
  /// Memory allocation failed.
  OutOfMemory,
  /// Timeout during a blocking operation.
  Timeout,
  /// No more space in queue.
  QueueFull,
  StringConversionError,
  TaskNotFound,
  InvalidQueueSize,
  ProcessorHasShutDown,
}

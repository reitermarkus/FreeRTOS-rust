/// Status of a [`Task`](crate::Task).
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum TaskState {
  /// The task is querying the state of itself, so must be running.
  Running = 0,
  /// The task is in a read or pending ready list.
  Ready = 1,
  /// The task is blocked.
  Blocked = 2,
  /// The task is suspended or blocked with an infinite time out.
  Suspended = 3,
  /// The task has been deleted, but its TCB has not yet been freed.
  Deleted = 4,
  /// The task state is invalid.
  Invalid = 5,
}

impl From<u32> for TaskState {
  fn from(s: u32) -> Self {
    match s {
      0 => Self::Running,
      1 => Self::Ready,
      2 => Self::Blocked,
      3 => Self::Suspended,
      4 => Self::Deleted,
      _ => Self::Invalid,
    }
  }
}

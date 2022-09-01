use crate::shim::{
  BaseType_t,
  taskSCHEDULER_SUSPENDED,
  taskSCHEDULER_NOT_STARTED,
  taskSCHEDULER_RUNNING,
};

/// State of the FreeRTOS task scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerState {
  /// Execution of all tasks is suspended.
  Suspended,
  /// Scheduler was not yet started.
  NotStarted,
  /// Scheduler is running.
  Running,
}

impl SchedulerState {
  pub(crate) const fn from_freertos(state: BaseType_t) -> Self{
    match state {
      taskSCHEDULER_SUSPENDED => SchedulerState::Suspended,
      taskSCHEDULER_NOT_STARTED => SchedulerState::NotStarted,
      taskSCHEDULER_RUNNING => SchedulerState::Running,
      _ => unreachable!(),
    }
  }
}

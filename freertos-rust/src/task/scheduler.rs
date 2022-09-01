use crate::shim::{
  BaseType_t,
  taskSCHEDULER_SUSPENDED,
  taskSCHEDULER_NOT_STARTED,
  taskSCHEDULER_RUNNING,
  vTaskStartScheduler,
  xTaskGetSchedulerState,
  vTaskSuspendAll,
  xTaskResumeAll,
  pdTRUE, xTaskGetTickCount, TickType_t,
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

/// The FreeRTOS task scheduler.
#[non_exhaustive]
pub struct Scheduler;

impl Scheduler {
  /// Start scheduling tasks.
  #[inline(always)]
  pub fn start() -> ! {
    unsafe { vTaskStartScheduler() };
    unreachable!()
  }

  /// Get the current scheduler state.
  #[inline]
  pub fn state() -> SchedulerState {
    SchedulerState::from_freertos(unsafe {
      xTaskGetSchedulerState()
    })
  }

  /// Suspend the scheduler without disabling interrupts.
  #[inline(always)]
  pub fn suspend() {
    unsafe { vTaskSuspendAll() }
  }

  /// Resume the scheduler.
  ///
  /// Returns `true` if resuming the scheduler caused a context switch.
  #[inline]
  pub fn resume() -> bool {
    unsafe { xTaskResumeAll() == pdTRUE }
  }

  /// Number of ticks since the scheduler was started.
  #[inline(always)]
  pub fn tick_count() -> TickType_t {
    unsafe { xTaskGetTickCount() }
  }
}

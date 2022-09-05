use core::mem::MaybeUninit;

use alloc2::vec::Vec;

use crate::shim::{
  BaseType_t,
  UBaseType_t,
  taskSCHEDULER_SUSPENDED,
  taskSCHEDULER_NOT_STARTED,
  taskSCHEDULER_RUNNING,
  vTaskStartScheduler,
  xTaskGetSchedulerState,
  vTaskSuspendAll,
  xTaskResumeAll,
  xTaskGetTickCount,
  uxTaskGetSystemState,
  uxTaskGetNumberOfTasks,
  pdTRUE,
  TaskStatus_t,
};

use crate::ticks::Ticks;

use super::{
  TaskName,
  TaskHandle,
  TaskStatus,
  SystemState,
  TaskPriority,
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
  pub fn tick_count() -> Ticks {
    unsafe { xTaskGetTickCount().into() }
  }

  /// Get the number of existing tasks.
  pub fn task_count() -> usize {
    unsafe { uxTaskGetNumberOfTasks() as usize }
  }

  /// Get the complete system state.
  pub fn system_state() -> SystemState {
    let tasks_len = Self::task_count();
    let mut tasks = Vec::<TaskStatus_t>::with_capacity(tasks_len as usize);
    let mut total_run_time = 0;

    unsafe {
        let filled = uxTaskGetSystemState(
          MaybeUninit::slice_as_mut_ptr(tasks.spare_capacity_mut()),
          tasks_len as UBaseType_t,
          &mut total_run_time,
        );
        tasks.set_len(filled as usize);
    }

    let tasks = tasks
      .into_iter()
      .map(|t| {
        TaskStatus {
          handle: unsafe { TaskHandle::from_ptr(t.xHandle) },
          name: unsafe { TaskName::from_ptr(t.pcTaskName) },
          number: t.xTaskNumber,
          state: t.eCurrentState.into(),
          current_priority: unsafe { TaskPriority::new_unchecked(t.uxCurrentPriority as u8) },
          base_priority: unsafe { TaskPriority::new_unchecked(t.uxBasePriority as u8) },
          run_time_counter: t.ulRunTimeCounter,
          stack_high_water_mark: t.usStackHighWaterMark,
        }
      })
      .collect();

    SystemState { tasks, total_run_time }
  }
}

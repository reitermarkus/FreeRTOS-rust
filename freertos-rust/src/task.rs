use core::{
  ffi::{CStr, c_void},
  mem::MaybeUninit,
  ptr::{self, NonNull},
};

#[cfg(feature = "alloc")]
use alloc::{
  boxed::Box,
  vec::Vec,
};

use crate::error::*;
use crate::isr::*;
use crate::shim::*;
use crate::ticks::*;

mod builder;
pub use builder::TaskBuilder;
mod current;
pub use current::CurrentTask;
mod name;
use name::TaskName;
mod notification;
pub use notification::TaskNotification;
mod priority;
pub use priority::TaskPriority;
mod scheduler;
pub use scheduler::{SchedulerState, Scheduler};
mod stack_overflow_hook;
pub use stack_overflow_hook::set_stack_overflow_hook;
mod state;
pub use state::TaskState;
mod system_state;
pub use system_state::{SystemState, TaskStatus};

/// Handle for a FreeRTOS task
#[derive(Debug, Clone)]
pub struct Task {
    handle: NonNull<c_void>,
}

unsafe impl Send for Task {}

impl Task {
    /// Minimal task stack size.
    pub const MINIMAL_STACK_SIZE: u16 = configMINIMAL_STACK_SIZE;

    /// Prepare a builder object for the new task.
    pub const fn new() -> TaskBuilder<'static> {
      TaskBuilder::new()
    }

    pub unsafe fn from_raw_handle(handle: TaskHandle_t) -> Self {
      Self { handle: NonNull::new_unchecked(handle) }
    }

    pub fn as_raw_handle(&self) -> TaskHandle_t {
      self.handle.as_ptr()
    }

    /// Suspend execution of the task.
    pub fn suspend(&self) {
        unsafe { vTaskSuspend(self.handle.as_ptr()) }
    }

    /// Resume execution of the task.
    pub fn resume(&self) {
        unsafe { vTaskResume(self.handle.as_ptr()) }
    }

    pub(crate) fn spawn<F>(
      name: &str,
      stack_size: u16,
      priority: TaskPriority,
      f: F,
    ) -> Result<Task, FreeRtosError>
    where
        F: FnOnce(&mut CurrentTask) + Send + 'static,
    {
        unsafe {
            Task::spawn_inner(Box::new(f), name, stack_size, priority)
        }
    }

    unsafe fn spawn_inner<'a>(
        f: Box<dyn FnOnce(&mut CurrentTask)>,
        name: &str,
        stack_size: u16,
        priority: TaskPriority,
    ) -> Result<Task, FreeRtosError> {
        extern "C" fn task_function(param: *mut c_void) {
            unsafe {
                // NOTE: New scope so that everything is dropped before the task is deleted.
                {
                    let mut current_task = CurrentTask;
                    let b = Box::from_raw(param as *mut Box<dyn FnOnce(&mut CurrentTask)>);
                    b(&mut current_task);
                }

                vTaskDelete(ptr::null_mut());
                unreachable!();
            }
        }

        let task_name = TaskName::<{ configMAX_TASK_NAME_LEN as usize }>::new(name);

        let param = Box::into_raw(Box::new(f));

        let mut task_handle = ptr::null_mut();

        let ret = unsafe {
          xTaskCreate(
            Some(task_function),
            task_name.as_ptr().cast(),
            stack_size,
            param.cast(),
            priority.to_freertos(),
            &mut task_handle,
          )
        };

        match ret {
          pdPASS if !task_handle.is_null() => {
            Ok(Task::from_raw_handle(task_handle))
          },
          errCOULD_NOT_ALLOCATE_REQUIRED_MEMORY => {
            drop(Box::from_raw(param));

            Err(FreeRtosError::OutOfMemory)
          },
          _ => unreachable!(),
        }
    }

  /// Get the name of the current task.
  pub fn name(&self) -> &str {
    unsafe {
      let task_name = pcTaskGetName(self.handle.as_ptr());
      CStr::from_ptr(task_name).to_str().unwrap()
    }
  }

    /// Try to find the task of the current execution context.
    pub fn current() -> Result<Task, FreeRtosError> {
        unsafe {
            match NonNull::new(xTaskGetCurrentTaskHandle()) {
              Some(handle) => Ok(Task { handle }),
              None => Err(FreeRtosError::TaskNotFound),
            }
        }
    }

    /// Forcibly set the notification value for this task.
    pub fn set_notification_value(&self, val: u32) {
        let _ = self.notify(TaskNotification::OverwriteValue(val));
    }

    /// Take the notification and either clear the notification value or decrement it by one.
    pub fn take_notification(clear: bool, timeout: impl Into<Ticks>) -> u32 {
      unsafe {
        ulTaskNotifyTake(if clear { pdTRUE } else { pdFALSE }, timeout.into().as_ticks())
      }
    }

    /// Notify this task.
    pub fn notify(&self, notification: TaskNotification) -> Result<(), FreeRtosError> {
      unsafe {
          let n = notification.to_freertos();
          if xTaskNotify(self.handle.as_ptr(), n.0, n.1) == pdPASS {
            return Ok(())
          }
      }

      Err(FreeRtosError::QueueFull)
    }

    /// Notify this task with the given index.
    pub fn notify_indexed(&self, index: u32, notification: TaskNotification) -> Result<(), FreeRtosError> {
      unsafe {
          let n = notification.to_freertos();
          if freertos_rs_task_notify_indexed(self.handle.as_ptr(), index, n.0, n.1) == pdPASS {
            return Ok(())
          }
      }

      Err(FreeRtosError::QueueFull)
    }

    /// Notify this task from an interrupt.
    pub fn notify_from_isr(
        &self,
        notification: TaskNotification,
        ic: &mut InterruptContext,
    ) -> Result<(), FreeRtosError> {
        unsafe {
            let n = notification.to_freertos();
            let t = xTaskNotifyFromISR(
                self.handle.as_ptr(),
                n.0,
                n.1,
                ic.as_ptr(),
            );
            if t == pdPASS {
                return Ok(())
            }
        }

        Err(FreeRtosError::QueueFull)
    }

    /// Notify this task from an interrupt with the given index.
    pub fn notify_indexed_from_isr(
      &self,
      index: u32,
      notification: TaskNotification,
      ic: &mut InterruptContext,
    ) -> Result<(), FreeRtosError> {
        unsafe {
            let n = notification.to_freertos();
            let t = freertos_rs_task_notify_indexed_from_isr(
                self.handle.as_ptr(),
                index,
                n.0,
                n.1,
                ic.as_ptr(),
            );
            if t == pdPASS {
              return Ok(())
          }
      }

      Err(FreeRtosError::QueueFull)
    }

    /// Wait for a notification to be posted.
    pub fn timeout_notification(
        &self,
        clear_bits_enter: u32,
        clear_bits_exit: u32,
        timeout: impl Into<Ticks>,
    ) -> Result<u32, FreeRtosError> {
        let mut val = 0;
        let r = unsafe {
            xTaskNotifyWait(
                clear_bits_enter,
                clear_bits_exit,
                &mut val as *mut _,
                timeout.into().as_ticks(),
            )
        };

        if r == pdPASS {
            return Ok(val)
          }
      Err(FreeRtosError::Timeout)
    }

  /// Get the minimum amount of stack that was ever left on this task.
  pub fn stack_high_water_mark(&self) -> u32 {
    unsafe { uxTaskGetStackHighWaterMark(self.handle.as_ptr()) }
  }

  /// Get the number of existing tasks.
  pub fn count() -> usize {
    unsafe { uxTaskGetNumberOfTasks() as usize }
  }

  /// Get the complete system state.
  pub fn system_state(tasks_len: Option<usize>) -> SystemState {
    let tasks_len = tasks_len.unwrap_or(Task::count());
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
          handle: unsafe { Task::from_raw_handle(t.xHandle) },
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

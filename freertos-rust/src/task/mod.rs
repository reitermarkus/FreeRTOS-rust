use core::{
  ffi::c_void,
  ptr::{self, NonNull}, ops::Deref,
};

#[cfg(feature = "alloc")]
use alloc2::{
  boxed::Box,
};

use crate::error::*;
use crate::shim::*;
use crate::lazy_init::PtrType;

mod builder;
pub use builder::TaskBuilder;
mod current;
pub use current::CurrentTask;
mod handle;
pub use handle::TaskHandle;
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
    handle: NonNull<<TaskHandle_t as PtrType>::Type>,
}

unsafe impl Send for Task {}

impl Task {
  /// Minimal task stack size.
  pub const MINIMAL_STACK_SIZE: u16 = configMINIMAL_STACK_SIZE;

  /// Prepare a builder object for the new task.
  pub const fn new() -> TaskBuilder<'static> {
    TaskBuilder::new()
  }

  pub fn idle_task() -> &'static TaskHandle {
    unsafe { TaskHandle::from_ptr(xTaskGetIdleTaskHandle()) }
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
          let mut current_task = CurrentTask::new_unchecked();
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

      match (ret, NonNull::new(task_handle)) {
        (pdPASS, Some(handle)) if !task_handle.is_null() => {
          Ok(Task { handle })
        },
        (errCOULD_NOT_ALLOCATE_REQUIRED_MEMORY, None) => {
          drop(Box::from_raw(param));
          Err(FreeRtosError::OutOfMemory)
        },
        _ => unreachable!(),
      }
  }
}

impl Deref for Task {
  type Target = TaskHandle;

  fn deref(&self) -> &Self::Target {
    let handle = self.handle.as_ptr();
    unsafe { TaskHandle::from_ptr(handle) }
  }
}

use core::{ptr, mem, ffi::c_void};

use alloc2::{boxed::Box};

use crate::{
  CurrentTask,
  shim::{configMAX_TASK_NAME_LEN, pdPASS, pdFAIL, xTaskCreate, vTaskDelete},
  FreeRtosError,
};

use super::{Task, TaskPriority, TaskName, MINIMAL_STACK_SIZE};

/// Helper for spawning a new task, created with [`Task::new`].
pub struct TaskBuilder<'n> {
  name: &'n str,
  stack_size: u16,
  priority: TaskPriority,
}

impl TaskBuilder<'_> {
  pub(crate) const fn new() -> TaskBuilder<'static> {
    TaskBuilder {
      name: "",
      stack_size: MINIMAL_STACK_SIZE,
      priority: TaskPriority::new(1).unwrap(),
    }
  }
}

impl TaskBuilder<'_> {
  /// Set the task name.
  pub const fn name<'n>(self, name: &'n str) -> TaskBuilder<'n> {
    TaskBuilder {
      name,
      stack_size: self.stack_size,
      priority: self.priority,
    }
  }

  /// Set the stack size in words.
  pub const fn stack_size(mut self, stack_size: u16) -> Self {
    self.stack_size = stack_size;
    self
  }

  /// Set the task priority.
  pub const fn priority(mut self, priority: TaskPriority) -> Self {
    self.priority = priority;
    self
  }

  /// Create and start the [`Task`].
  pub fn start<'f, F>(&self, f: F) -> Result<Task, FreeRtosError>
  where
    F: FnOnce(&mut CurrentTask) + Send + 'static,
  {
    extern "C" fn task_function(param: *mut c_void) {
      unsafe {
        // NOTE: New scope so that everything is dropped before the task is deleted.
        {
          let mut current_task = CurrentTask::new_unchecked();
          let function = Box::from_raw(param as *mut Box<dyn FnOnce(&mut CurrentTask)>);
          function(&mut current_task);
        }

        vTaskDelete(ptr::null_mut());
        unreachable!();
      }
    }

    let name = TaskName::<{ configMAX_TASK_NAME_LEN as usize }>::new(self.name);

    let f: Box<dyn FnOnce(&mut CurrentTask)> = Box::new(f);
    let mut function = Box::new(f);
    let function_ptr: *mut Box<dyn FnOnce(&mut CurrentTask)> = &mut *function;

    let mut ptr = ptr::null_mut();
    let res = unsafe {
      xTaskCreate(
        Some(task_function),
        name.as_ptr(),
        self.stack_size,
        function_ptr.cast(),
        self.priority.to_freertos(),
        &mut ptr,
      )
    };

    match (res, ptr) {
      (pdPASS, ptr) if !ptr.is_null() => {
        mem::forget(function);
        Ok(Task { ptr })
      },
      (pdFAIL, ptr) if ptr.is_null() => {
        Err(FreeRtosError::OutOfMemory)
      },
      _ => unreachable!(),
    }
  }
}

use core::{mem::{MaybeUninit, self}, ptr, ffi::c_void};

use alloc2::{boxed::Box};

use crate::{
  CurrentTask,
  shim::{xTaskCreate, vTaskDelete, pdPASS},
};
#[cfg(freertos_feature = "static_allocation")]
use crate::StaticTask;

use super::{Task, TaskPriority, TaskName, MINIMAL_STACK_SIZE};

/// Helper for creating a new task returned by [`Task::new`].
pub struct TaskBuilder<'n> {
  name: &'n str,
  stack_size: usize,
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
  pub const fn stack_size(mut self, stack_size: usize) -> Self {
    self.stack_size = stack_size;
    self
  }

  /// Set the task priority.
  pub const fn priority(mut self, priority: TaskPriority) -> Self {
    self.priority = priority;
    self
  }

  /// Create the [`Task`].
  pub fn create<'f, F>(&self, f: F) -> Task
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

    let name = TaskName::new(self.name);

    let function: Box<dyn FnOnce(&mut CurrentTask)> = Box::new(f);
    let function_ptr: *mut Box<dyn FnOnce(&mut CurrentTask)> = Box::into_raw(Box::new(function));
    let mut ptr = ptr::null_mut();

    unsafe {
      let res = xTaskCreate(
        Some(task_function),
        name.as_ptr(),
        self.stack_size.try_into().unwrap_or(!0),
        function_ptr.cast(),
        self.priority.to_freertos(),
        &mut ptr,
      );

      if res == pdPASS {
        debug_assert!(!ptr.is_null());

        Task { handle: ptr }
      } else {
        drop(Box::from_raw(function_ptr));
        assert_eq!(res, pdPASS);
        unreachable!();
      }
    }
  }

  /// Create the static [`Task`].
  ///
  /// The returned task needs to be started.
  ///
  /// # Safety
  ///
  /// The returned task must have a `'static` lifetime.
  ///
  /// # Examples
  ///
  /// ```
  /// use core::mem::MaybeUninit;
  ///
  /// use freertos_rust::{alloc::Static, task::{Task, CurrentTask}};
  ///
  /// fn my_task(task: &mut CurrentTask) {
  ///   // ...
  /// }
  ///
  /// static mut TASK: MaybeUninit<StaticTask<STACK_SIZE>> = MaybeUninit::uninit();
  /// // SAFETY: Only used once to create `my_task` below.
  /// let task = unsafe { &mut TASK };
  ///
  /// Task::new().name("my_task").create_static(task, my_task)
  /// ```
  #[cfg(freertos_feature = "static_allocation")]
  pub fn create_static<const STACK_SIZE: usize>(self, task: &'static mut MaybeUninit<StaticTask<STACK_SIZE>>, f: fn(&mut CurrentTask)) -> &'static StaticTask<STACK_SIZE> {
    use crate::shim::xTaskCreateStatic;

    assert!(STACK_SIZE <= self.stack_size);

    extern "C" fn task_function(param: *mut c_void) {
      unsafe {
        // NOTE: New scope so that everything is dropped before the task is deleted.
        {
          let mut current_task = CurrentTask::new_unchecked();
          let function: fn(&mut CurrentTask) = mem::transmute(param);
          function(&mut current_task);
        }

        vTaskDelete(ptr::null_mut());
        unreachable!();
      }
    }

    let name = TaskName::new(self.name);

    let function_ptr = f as *mut c_void;

    let task_ptr = task.as_mut_ptr();

    unsafe {
      let stack_buffer = ptr::addr_of_mut!((*task_ptr).stack).cast();
      let task_buffer = ptr::addr_of_mut!((*task_ptr).data);

      let ptr = xTaskCreateStatic(
        Some(task_function),
        name.as_ptr(),
        self.stack_size as _,
        function_ptr,
        self.priority.to_freertos(),
        stack_buffer,
        task_buffer,
      );

      debug_assert!(!ptr.is_null());
      debug_assert_eq!(ptr, task_buffer.cast());

      task.assume_init_ref()
    }
  }
}

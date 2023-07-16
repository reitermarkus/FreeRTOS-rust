use core::{mem::{MaybeUninit, self}, ptr, ffi::c_void};

#[cfg(freertos_feature = "dynamic_allocation")]
use alloc2::boxed::Box;

use crate::{
  CurrentTask,
  shim::{vTaskDelete, pdPASS},
};
#[cfg(freertos_feature = "dynamic_allocation")]
use crate::shim::xTaskCreate;
#[cfg(freertos_feature = "static_allocation")]
use crate::{StaticTask, shim::xTaskCreateStatic};

use super::{Task, TaskPriority, TaskName, MINIMAL_STACK_SIZE};

#[cfg(freertos_feature = "dynamic_allocation")]
type BoxTaskFn = Box<dyn FnOnce(&mut CurrentTask)>;

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
  pub const fn name(self, name: &str) -> TaskBuilder<'_> {
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
  #[cfg(freertos_feature = "dynamic_allocation")]
  pub fn create<F>(&self, f: F) -> Task
  where
    F: FnOnce(&mut CurrentTask) + Send + 'static,
  {
    extern "C" fn task_function(param: *mut c_void) {
      unsafe {
        // NOTE: New scope so that everything is dropped before the task is deleted.
        {
          let mut current_task = CurrentTask::new_unchecked();
          let function: &mut Option<BoxTaskFn> = &mut *param.cast();
          let function = function.take().unwrap_unchecked();
          function(&mut current_task);
        }

        vTaskDelete(ptr::null_mut());
        unreachable!();
      }
    }

    let name = TaskName::new(self.name);

    let function: BoxTaskFn = Box::new(f);
    let function_ptr: *mut Option<BoxTaskFn> = Box::into_raw(Box::new(Some(function)));
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

        Task {
          handle: ptr,
          function: Some(Box::from_raw(function_ptr)),
        }
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
  /// use freertos_rust::{Task, StaticTask, CurrentTask};
  ///
  /// fn my_task(task: &mut CurrentTask) {
  ///   // ...
  /// }
  ///
  /// static mut TASK: MaybeUninit<StaticTask<128>> = MaybeUninit::uninit();  ///
  /// let _task = Task::new().name("my_task").create_static(unsafe { &mut TASK }, my_task);
  /// ```
  #[cfg(freertos_feature = "static_allocation")]
  pub fn create_static<const STACK_SIZE: usize>(self, task: &'static mut MaybeUninit<StaticTask<STACK_SIZE>>, f: fn(&mut CurrentTask)) -> Task {
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

      Task {
        handle: ptr,
        #[cfg(freertos_feature = "dynamic_allocation")]
        function: None,
      }
    }
  }
}

//! FreeRTOS task primitives.
//!
//! # Examples
//!
//! ```no_run
//! use core::time::Duration;
//! use freertos_rust::task::{Task, Scheduler};
//!
//! let _task = Task::new().name("hello").stack_size(128).create(|task| {
//!   loop {
//!     println!("Hello, world!");
//!     task.delay(Duration::MAX);
//!   }
//! });
//!
//! Scheduler::start();
//! ```

use core::{
  ops::Deref,
  mem::MaybeUninit,
};

#[cfg(freertos_feature = "dynamic_allocation")]
use alloc2::boxed::Box;

use crate::{
  shim::{configMINIMAL_STACK_SIZE, xTaskGetIdleTaskHandle, StaticTask_t, StackType_t, vTaskDelete},
  ffi::TaskHandle_t,
};

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

/// Minimal task stack size.
pub const MINIMAL_STACK_SIZE: usize = configMINIMAL_STACK_SIZE as usize;

/// A task.
#[must_use = "task will be deleted immediately if unused"]
pub struct Task {
  handle: TaskHandle_t,
  #[cfg(freertos_feature = "dynamic_allocation")]
  #[allow(unused)]
  function: Option<Box<Option<Box<dyn FnOnce(&mut CurrentTask)>>>>,
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
  /// Prepare a builder object for the new task.
  pub const fn new() -> TaskBuilder<'static> {
    TaskBuilder::new()
  }

  /// Get the handle for the idle task.
  pub fn idle_task() -> &'static TaskHandle {
    unsafe { TaskHandle::from_ptr(xTaskGetIdleTaskHandle()) }
  }
}

impl Deref for Task {
  type Target = TaskHandle;

  #[inline]
  fn deref(&self) -> &Self::Target {
    unsafe { TaskHandle::from_ptr(self.handle) }
  }
}

impl Drop for Task {
  fn drop(&mut self) {
      unsafe { vTaskDelete(self.as_ptr()) }
  }
}

/// A statically allocated task.
pub struct StaticTask<const STACK_SIZE: usize = MINIMAL_STACK_SIZE> {
  data: StaticTask_t,
  stack: [MaybeUninit<StackType_t>; STACK_SIZE],
}

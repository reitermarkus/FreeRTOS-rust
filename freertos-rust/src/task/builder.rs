use crate::{FreeRtosError, CurrentTask};

use super::{Task, TaskPriority};

/// Helper for spawning a new task, created with [`Task::new`].
pub struct TaskBuilder<'n> {
  task_name: &'n str,
  task_stack_size: u16,
  task_priority: TaskPriority,
}

impl TaskBuilder<'_> {
  pub(crate) const fn new() -> TaskBuilder<'static> {
    TaskBuilder {
      task_name: "rust_task",
      task_stack_size: Task::MINIMAL_STACK_SIZE,
      task_priority: TaskPriority::new(1).unwrap(),
    }
  }

  /// Set the task name.
  pub const fn name<'n>(self, name: &'n str) -> TaskBuilder<'n> {
    TaskBuilder {
      task_name: name,
      task_stack_size: self.task_stack_size,
      task_priority: self.task_priority,
    }
  }

  /// Set the stack size in words.
  pub const fn stack_size(mut self, stack_size: u16) -> Self {
    self.task_stack_size = stack_size;
    self
  }

  /// Set the task priority.
  pub const fn priority(mut self, priority: TaskPriority) -> Self {
    self.task_priority = priority;
    self
  }

  /// Start a new task.
  pub fn start<F>(&self, func: F) -> Result<Task, FreeRtosError>
  where
    F: FnOnce(&mut CurrentTask) -> () + Send + 'static,
  {
    Task::spawn(
      self.task_name,
      self.task_stack_size,
      self.task_priority,
      func,
    )
  }
}

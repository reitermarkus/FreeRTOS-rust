use core::marker::PhantomData;

use alloc2::{boxed::Box};

use crate::{
  alloc::{Dynamic, Static},
  CurrentTask,
  lazy_init::{LazyPtr, LazyInit},
};

use super::{Task, TaskPriority, TaskName, TaskMeta, MINIMAL_STACK_SIZE};

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

  /// Create the dynamic [`Task`].
  ///
  /// The returned task needs to be started.
  #[must_use = "task must be started"]
  pub fn create<'f, F>(&self, f: F) -> Task<Dynamic>
  where
    F: FnOnce(&mut CurrentTask) + Send + 'static,
  {
    let meta: <Task<Dynamic> as LazyInit>::Data = TaskMeta {
      name: TaskName::new(self.name),
      stack_size: self.stack_size,
      priority: self.priority,
      f: Some(Box::new(f)),
    };

    Task {
      alloc_type: PhantomData,
      handle: LazyPtr::new(meta),
    }
  }
}

impl TaskBuilder<'static> {
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
  /// use freertos_rust::{alloc::Static, task::{Task, CurrentTask}};
  ///
  /// fn my_task(task: &mut CurrentTask) {
  ///   // ...
  /// }
  ///
  /// // SAFETY: Assignment to a `static` ensures a `'static` lifetime.
  /// static TASK: Task<Static, 1337> = unsafe {
  ///   Task::new().create_static(my_task)
  /// };
  ///
  /// TASK.start();
  /// ```
  #[must_use = "task must be started"]
  pub const unsafe fn create_static<const STACK_SIZE: usize>(self, f: fn(&mut CurrentTask)) -> Task<Static, STACK_SIZE> {
    let meta: <Task<Static> as LazyInit>::Data = TaskMeta {
      name: self.name,
      stack_size: (),
      priority: self.priority,
      f,
    };

    Task {
      alloc_type: PhantomData,
      handle: LazyPtr::new(meta),
    }
  }
}

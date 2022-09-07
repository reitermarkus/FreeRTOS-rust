use core::{
  ffi::c_void,
  ptr,
  ops::Deref, cell::UnsafeCell,
  mem::{self, MaybeUninit}, sync::atomic::{AtomicPtr, Ordering},
};

use crate::shim::*;

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
pub const MINIMAL_STACK_SIZE: u16 = configMINIMAL_STACK_SIZE;


pub struct StaticTaskBuilder {
  name: &'static str,
  priority: TaskPriority,
}

impl StaticTaskBuilder {
  /// Set the task name.
  pub const fn name(mut self, name: &'static str) -> Self {
    self.name = name;
    self
  }

  /// Set the task priority.
  pub const fn priority(mut self, priority: TaskPriority) -> Self {
    self.priority = priority;
    self
  }

  /// Create the [`StaticTask`].
  ///
  /// The returned `StaticTask` needs to be assigned to a `static` variable in order to be started.
  pub const fn create<const STACK_SIZE: usize>(self, f: fn(&mut CurrentTask)) -> StaticTask<STACK_SIZE> {
    StaticTask {
      name: self.name,
      priority: self.priority,
      task: UnsafeCell::new(MaybeUninit::uninit()),
      stack: UnsafeCell::new(MaybeUninit::uninit_array()),
      f: AtomicPtr::new(f as *mut _),
    }
  }
}

/// A statically allocated task.
pub struct StaticTask<const STACK_SIZE: usize = 0> {
  name: &'static str,
  priority: TaskPriority,
  task: UnsafeCell<MaybeUninit<StaticTask_t>>,
  stack: UnsafeCell<[MaybeUninit<StackType_t>; STACK_SIZE]>,
  // Invariant: If `f` is null, the task is started and `task` is initialized.
  f: AtomicPtr<c_void>,
}

unsafe impl<const STACK_SIZE: usize> Send for StaticTask<STACK_SIZE> {}
unsafe impl<const STACK_SIZE: usize> Sync for StaticTask<STACK_SIZE> {}

impl StaticTask {
  pub const fn new() -> StaticTaskBuilder {
    StaticTaskBuilder {
      name: "",
      priority: unsafe { TaskPriority::new_unchecked(1) },
    }
  }
}

impl<const STACK_SIZE: usize> StaticTask<STACK_SIZE> {
  /// Start the task.
  pub fn start(&'static self) -> &'static TaskHandle {
    let function_ptr = self.f.load(Ordering::Acquire);
    if function_ptr.is_null() {
      return unsafe { TaskHandle::from_ptr((&mut *self.task.get()).as_mut_ptr().cast()) }
    }

    self.start_inner()
  }

  #[cold]
  fn start_inner(&'static self) -> &'static TaskHandle {
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

    unsafe {
      freertos_rs_enter_critical();

      let mut function_ptr = &mut *ptr::addr_of!(self.f).cast_mut();
      let function_ptr = function_ptr.get_mut();

      if function_ptr.is_null() {
        freertos_rs_exit_critical();
        return TaskHandle::from_ptr((&mut *self.task.get()).as_mut_ptr().cast())
      }

      let name = TaskName::<{ configMAX_TASK_NAME_LEN as usize }>::new(self.name);
      let task = &mut *self.task.get();
      let stack = &mut *self.stack.get();

      let ptr = xTaskCreateStatic(
        Some(task_function),
        name.as_ptr(),
        STACK_SIZE as _,
        *function_ptr,
        self.priority.to_freertos(),
        MaybeUninit::slice_as_mut_ptr(stack),
        task.as_mut_ptr(),
      );
      debug_assert!(!ptr.is_null());

      *function_ptr = ptr::null_mut();

      freertos_rs_exit_critical();

      TaskHandle::from_ptr(ptr)
    }
  }
}

/// A task.
pub struct Task {
  ptr: TaskHandle_t,
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

  fn deref(&self) -> &Self::Target {
    unsafe { TaskHandle::from_ptr(self.ptr) }
  }
}

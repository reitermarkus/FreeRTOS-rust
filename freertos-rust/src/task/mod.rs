//! FreeRTOS task primitives.
//!
//! # Examples
//!
//! ```
//! use core::time::Duration;
//! use freertos_rust::task::{Task, Scheduler};
//!
//! Task::new().name("hello").stack_size(128).create(|task| {
//!   loop {
//!     println!("Hello, world!");
//!     task.delay(Duration::MAX);
//!   }
//! }).start();
//!
//! Scheduler::start();
//! ```

use core::{
  ffi::c_void,
  ptr,
  marker::PhantomData,
  ops::Deref, cell::UnsafeCell,
  mem::{self, MaybeUninit},
};

use alloc2::boxed::Box;

use crate::alloc::Dynamic;
#[cfg(freertos_feature = "static_allocation")]
use crate::alloc::Static;
use crate::shim::*;
use crate::lazy_init::LazyInit;
use crate::lazy_init::LazyPtr;

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

/// A task.
pub struct Task<A = Dynamic, const STACK_SIZE: usize = 0>
where
  Self: LazyInit,
{
  alloc_type: PhantomData<A>,
  handle: LazyPtr<Self>,
}

unsafe impl<A, const STACK_SIZE: usize> Send for Task<A, STACK_SIZE>
where
  Self: LazyInit
{}
unsafe impl<A, const STACK_SIZE: usize> Sync for Task<A, STACK_SIZE>
where
  Self: LazyInit
{}

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

impl<A, const STACK_SIZE: usize> Task<A, STACK_SIZE>
where
  Self: LazyInit<Handle = TaskHandle_t>,
{
  /// Start the task.
  pub fn start(&self) -> &TaskHandle {
    unsafe { TaskHandle::from_ptr(self.handle.as_ptr()) }
  }
}

impl<A, const STACK_SIZE: usize> Deref for Task<A, STACK_SIZE>
where
  Self: LazyInit<Handle = TaskHandle_t>,
{
  type Target = TaskHandle;

  #[inline]
  fn deref(&self) -> &Self::Target {
    self.start()
  }
}

#[doc(hidden)]
pub struct TaskMeta<N, S, F> {
  name: N,
  stack_size: S,
  priority: TaskPriority,
  f: F,
}

impl LazyInit for Task<Dynamic> {
  type Storage = ();
  type Handle = TaskHandle_t;
  type Data = TaskMeta<TaskName<{ configMAX_TASK_NAME_LEN as usize }>, u16, Option<Box<dyn FnOnce(&mut CurrentTask)>>>;

  fn init(data: &UnsafeCell<Self::Data>, _storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
    let data = unsafe { &mut *data.get() };

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

    let mut function = Box::new(data.f.take().unwrap());
    let function_ptr: *mut Box<dyn FnOnce(&mut CurrentTask)> = &mut *function;

    let mut ptr = ptr::null_mut();
    let res = unsafe {
      xTaskCreate(
        Some(task_function),
        data.name.as_ptr(),
        data.stack_size,
        function_ptr.cast(),
        data.priority.to_freertos(),
        &mut ptr,
      )
    };
    assert_eq!(res, pdPASS);
    assert!(!ptr.is_null());

    mem::forget(function);
    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn cancel_init_supported() -> bool {
    false
  }

  fn destroy(_ptr: Self::Ptr, _storage: &mut MaybeUninit<Self::Storage>) {
    // Task deletes itself.
  }
}

#[cfg(freertos_feature = "static_allocation")]
impl<const STACK_SIZE: usize> LazyInit for Task<Static, STACK_SIZE> {
  type Storage = (MaybeUninit<StaticTask_t>, [MaybeUninit<StackType_t>; STACK_SIZE]);
  type Handle = TaskHandle_t;
  type Data = TaskMeta<&'static str, (), fn(&mut CurrentTask)>;

  fn init(data: &UnsafeCell<Self::Data>, storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
    let data = unsafe { &mut *data.get() };
    let (task, stack) = unsafe { (&mut *storage.get()).assume_init_mut() };

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

    let name = TaskName::<{ configMAX_TASK_NAME_LEN as usize }>::new(data.name);

    let function: fn(&mut CurrentTask) = data.f;
    let function_ptr = function as *mut c_void;

    let ptr = unsafe {
      xTaskCreateStatic(
        Some(task_function),
        name.as_ptr(),
        STACK_SIZE as _,
        function_ptr,
        data.priority.to_freertos(),
        MaybeUninit::slice_as_mut_ptr(stack),
        task.as_mut_ptr(),
      )
    };
    debug_assert!(!ptr.is_null());


    unsafe { Self::Ptr::new_unchecked(ptr) }
  }

  fn cancel_init_supported() -> bool {
    false
  }

  fn destroy(_ptr: Self::Ptr, _storage: &mut MaybeUninit<Self::Storage>) {
    // Task deletes itself.
  }
}

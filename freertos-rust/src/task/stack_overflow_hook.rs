use core::{ffi::{c_char, CStr}, sync::atomic::{AtomicPtr, Ordering}, mem};

use crate::shim::TaskHandle_t;

use super::Task;

type StackOverflowHookFunction = fn(&Task, &str);

static STACK_OVERFLOW_HOOK_FUNCTION: AtomicPtr<StackOverflowHookFunction> = AtomicPtr::new(default_stack_overflow_hook as *mut _);

fn default_stack_overflow_hook(task: &Task, task_name: &str) {
  panic!("task '{}' ({:?}) has overflowed its stack", task_name, task.as_raw_handle());
}

/// Set a custom stack overflow hook.
///
/// ```
/// fn my_stack_overflow_hook(task: &Task, task_name: &str) {
///   panic!("Stack overflow detected in task '{}' at {:?}.", task_name, task.as_raw_handle());
/// }
///
/// freertos_rust::set_stack_overflow_hook(my_stack_overflow_hook);
/// ```
pub fn set_stack_overflow_hook(f: fn(&Task, &str)) {
  STACK_OVERFLOW_HOOK_FUNCTION.store(f as *mut _, Ordering::Release);
}

#[export_name = "vApplicationStackOverflowHook"]
extern "C" fn stack_overflow_hook(task_handle: TaskHandle_t, task_name: *const c_char) {
  unsafe {
    let task = Task::from_raw_handle(task_handle);
    let task_name = CStr::from_ptr(task_name).to_str().unwrap();

    let f: StackOverflowHookFunction = mem::transmute(STACK_OVERFLOW_HOOK_FUNCTION.load(Ordering::Acquire));
    f(&task, task_name);
  }
}

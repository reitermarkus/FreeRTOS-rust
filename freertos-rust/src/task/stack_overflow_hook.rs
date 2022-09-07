use core::{
  ffi::{c_char, CStr},
  sync::atomic::{AtomicPtr, Ordering},
  str,
  mem,
};

use crate::ffi::TaskHandle_t;

use super::TaskHandle;

type StackOverflowHookFunction = fn(&TaskHandle, &str);

static STACK_OVERFLOW_HOOK_FUNCTION: AtomicPtr<StackOverflowHookFunction> = AtomicPtr::new(default_stack_overflow_hook as *mut _);

fn default_stack_overflow_hook(task: &TaskHandle, task_name: &str) {
  if task_name.is_empty() {
    panic!("task {:?} has overflowed its stack", task.as_ptr());
  } else {
    panic!("task '{}' has overflowed its stack", task_name);
  }
}

/// Set a custom stack overflow hook.
///
/// ```
/// use freertos_rust::task::{self, TaskHandle};
///
/// fn my_stack_overflow_hook(task: &TaskHandle, task_name: &str) {
///   panic!("Stack overflow detected in task '{}' at {:?}.", task_name, task.as_raw_handle());
/// }
///
/// task::set_stack_overflow_hook(my_stack_overflow_hook);
/// ```
pub fn set_stack_overflow_hook(f: fn(&TaskHandle, &str)) {
  STACK_OVERFLOW_HOOK_FUNCTION.store(f as *mut _, Ordering::Release);
}

#[export_name = "vApplicationStackOverflowHook"]
extern "C" fn stack_overflow_hook(task_handle: TaskHandle_t, task_name: *const c_char) {
  unsafe {
    let task = TaskHandle::from_ptr(task_handle);
    let task_name = task_name.as_ref()
      .map(|n| CStr::from_ptr(n))
      .map(|n| match n.to_str() {
        Ok(n) => n,
        Err(err) => str::from_utf8_unchecked(&n.to_bytes()[..err.valid_up_to()]),
      })
      .unwrap_or_default();

    let f: StackOverflowHookFunction = mem::transmute(STACK_OVERFLOW_HOOK_FUNCTION.load(Ordering::Acquire));
    f(&task, task_name);
  }
}

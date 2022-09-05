use core::{
  ffi::{c_char, CStr},
  sync::atomic::{AtomicPtr, Ordering},
  mem,
};

use crate::ffi::TaskHandle_t;

use super::TaskHandle;

type StackOverflowHookFunction = fn(&TaskHandle, Option<&CStr>);

static STACK_OVERFLOW_HOOK_FUNCTION: AtomicPtr<StackOverflowHookFunction> = AtomicPtr::new(default_stack_overflow_hook as *mut _);

fn default_stack_overflow_hook(task: &TaskHandle, task_name: Option<&CStr>) {
  if let Some(name) = task_name.and_then(|n| n.to_str().ok()) {
    panic!("task '{}' has overflowed its stack", name);
  } else {
    panic!("task {:?} has overflowed its stack", task.as_ptr());
  }
}

/// Set a custom stack overflow hook.
///
/// ```
/// use core::ffi::CStr;
/// use freertos_rust::task::{self, TaskHandle};
///
/// fn my_stack_overflow_hook(task: &TaskHandle, task_name: Option<&CStr>) {
///   panic!("Stack overflow detected in task {:?} at {:?}.", task_name.unwrap_or_default(), task.as_raw_handle());
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
    let task_name = if task_name.is_null() {
      None
    } else {
      Some(CStr::from_ptr(task_name))
    };

    let f: StackOverflowHookFunction = mem::transmute(STACK_OVERFLOW_HOOK_FUNCTION.load(Ordering::Acquire));
    f(&task, task_name);
  }
}

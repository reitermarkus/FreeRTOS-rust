use core::sync::atomic::{AtomicPtr, Ordering};
use core::mem;

static TICK_FUNCTION: AtomicPtr<()> = AtomicPtr::new(default_tick_hook as *mut _);

/// Set a custom tick hook.
///
/// # Examples
///
/// ```
/// static mut CURRENT_TICK: usize = 0;
///
/// fn my_tick_hook() {
///   unsafe { CURRENT_TICK = CURRENT_TICK.wrapping_add(1) };
/// }
///
/// freertos_rust::set_tick_hook(my_tick_hook);
/// ```
pub fn set_tick_hook(f: fn()) {
  TICK_FUNCTION.store(f as *mut _, Ordering::Release);
}

fn default_tick_hook() {}

#[export_name = "vApplicationTickHook"]
extern "C" fn application_tick_hook() {
  let f: fn() = unsafe { mem::transmute(TICK_FUNCTION.load(Ordering::Acquire)) };
  f();
}

static IDLE_FUNCTION: AtomicPtr<()> = AtomicPtr::new(default_idle_hook as *mut _);

/// Set a custom idle hook.
///
/// # Examples
///
/// ```
/// fn my_idle_hook() {
///   // ...
/// }
///
/// freertos_rust::set_idle_hook(my_idle_hook);
/// ```
pub fn set_idle_hook(f: fn()) {
  IDLE_FUNCTION.store(f as *mut _, Ordering::Release);
}

fn default_idle_hook() {}

#[export_name = "vApplicationIdleHook"]
extern "C" fn application_idle_hook() {
  let f: fn() = unsafe { mem::transmute(IDLE_FUNCTION.load(Ordering::Acquire)) };
  f();
}

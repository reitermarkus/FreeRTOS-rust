#![allow(non_snake_case)]

#[cfg(freertos_feature = "static_allocation")]
use core::mem::MaybeUninit;

mod bindings {
  #![allow(unused)]
  #![allow(missing_docs)]

  include!(concat!(env!("OUT_DIR"), "/shim.rs"));
}

pub use bindings::*;

#[cfg(freertos_feature = "static_allocation")]
#[no_mangle]
unsafe extern "C" fn vApplicationGetIdleTaskMemory(
  tcb_buffer: *mut *mut StaticTask_t,
  stack_buffer: *mut *mut StackType_t,
  stack_size: *mut u32,
) {
  static mut IDLE_TASK_TCB: MaybeUninit<StaticTask_t> = MaybeUninit::uninit();
  static mut IDLE_TASK_STACK: [MaybeUninit<StackType_t>; configMINIMAL_STACK_SIZE as usize] = MaybeUninit::uninit_array();

  // Pass out a pointer to the `StaticTask_t` structure in which the Idle task's state will be stored.
  *tcb_buffer = IDLE_TASK_TCB.as_mut_ptr();

  // Pass out the array that will be used as the Idle task's stack.
  *stack_buffer = MaybeUninit::slice_as_mut_ptr(&mut IDLE_TASK_STACK);

  // Pass out the size of the array pointed to by `stack_buffer`. Note that the array is necessarily
  // of type `StackType_t`, i.e. `configMINIMAL_STACK_SIZE` is specified in words, not bytes.
  *stack_size = configMINIMAL_STACK_SIZE.into();
}

#[cfg(freertos_feature = "static_allocation")]
#[no_mangle]
unsafe extern "C" fn vApplicationGetTimerTaskMemory(
  tcb_buffer: *mut *mut StaticTask_t,
  stack_buffer: *mut *mut StackType_t,
  stack_size: *mut u32,
) {
  static mut TIMER_TASK_TCB: MaybeUninit<StaticTask_t> = MaybeUninit::uninit();
  static mut TIMER_TASK_STACK: [MaybeUninit<StackType_t>; configTIMER_TASK_STACK_DEPTH as usize] = MaybeUninit::uninit_array();

  // Pass out a pointer to the `StaticTask_t` structure in which the Timer task's state will be stored.
  *tcb_buffer = TIMER_TASK_TCB.as_mut_ptr();

  // Pass out the array that will be used as the Timer task's stack.
  *stack_buffer = MaybeUninit::slice_as_mut_ptr(&mut TIMER_TASK_STACK);

  // Pass out the size of the array pointed to by `stack_buffer`. Note that the array is necessarily
  // of type `StackType_t`, i.e. `configTIMER_TASK_STACK_DEPTH` is specified in words, not bytes.
  *stack_size = configTIMER_TASK_STACK_DEPTH.into();
}

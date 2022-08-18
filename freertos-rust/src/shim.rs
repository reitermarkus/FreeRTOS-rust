#![allow(non_snake_case)]

use core::{mem::{self, size_of}, ptr};

use const_zero::const_zero;

include!(concat!(env!("OUT_DIR"), "/shim.rs"));

#[no_mangle]
unsafe extern "C" fn vApplicationGetIdleTaskMemory(
  tcb_buffer: *mut *mut StaticTask_t,
  stack_buffer: *mut *mut StackType_t,
  stack_size: *mut u32,
) {
  static mut IDLE_TASK_TCB: StaticTask_t = unsafe { const_zero!(StaticTask_t) };
  static mut IDLE_TASK_STACK: [StackType_t; TASK_MINIMAL_STACK_SIZE as usize] = [0; TASK_MINIMAL_STACK_SIZE as usize];

  // Pass out a pointer to the `StaticTask_t` structure in which the Idle task's state will be stored.
  *tcb_buffer = ptr::addr_of_mut!(IDLE_TASK_TCB);

  // Pass out the array that will be used as the Idle task's stack.
  *stack_buffer = IDLE_TASK_STACK.as_mut_ptr();

  // Pass out the size of the array pointed to by `stack_buffer`. Note that the array is necessarily
  // of type `StackType_t`, i.e. `TASK_MINIMAL_STACK_SIZE` is specified in words, not bytes.
  *stack_size = u32::from(TASK_MINIMAL_STACK_SIZE);
}

#[no_mangle]
unsafe extern "C" fn vApplicationGetTimerTaskMemory(
  tcb_buffer: *mut *mut StaticTask_t,
  stack_buffer: *mut *mut StackType_t,
  stack_size: *mut u32,
) {
  static mut TIMER_TASK_TCB: StaticTask_t = unsafe { const_zero!(StaticTask_t) };
  static mut TIMER_TASK_STACK: [StackType_t; TIMER_TASK_STACK_SIZE as usize] = [0; TIMER_TASK_STACK_SIZE as usize];

  // Pass out a pointer to the `StaticTask_t` structure in which the Timer task's state will be stored.
  *tcb_buffer = ptr::addr_of_mut!(TIMER_TASK_TCB);

  // Pass out the array that will be used as the Timer task's stack.
  *stack_buffer = TIMER_TASK_STACK.as_mut_ptr();

  // Pass out the size of the array pointed to by `stack_buffer`. Note that the array is necessarily
  // of type `StackType_t`, i.e. `TIMER_TASK_STACK_SIZE` is specified in words, not bytes.
  *stack_size = u32::from(TIMER_TASK_STACK_SIZE);
}

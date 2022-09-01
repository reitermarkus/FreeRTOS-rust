use crate::shim::*;

/// Representation of an interrupt context.
///
/// The existence of this struct means that the current function is inside an interrupt service
/// routine. FreeRTOS needs to keep track of whether or not to yield the execution to a different
/// task after returning from the interrupt routine, so this struct needs to be passed to all
/// `*_from_isr` functions.
///
/// A single `InterruptContext` should be created at the start of an interrupt routine and dropped
/// as the last thing inside the same interrupt routine as dropping it calls `portYIELD_FROM_ISR`.
#[repr(transparent)]
#[must_use]
pub struct InterruptContext {
  x_higher_priority_task_woken: BaseType_t,
}

impl InterruptContext {
  /// Instantiate a new context.
  ///
  /// This must be called from within an interrupt service routine.
  pub fn new() -> InterruptContext {
    InterruptContext { x_higher_priority_task_woken: 0 }
  }

  /// Create an `InterruptContext` from a raw pointer.
  ///
  /// # Safety
  ///
  /// `x_higher_priority_task_woken` must point to a `BaseType_t` which will be
  /// passed to `portYIELD_FROM_ISR` at the end of an interrupt.
  pub unsafe fn from_ptr<'a>(ptr: *mut BaseType_t) -> &'a mut Self {
    // SAFETY: `InterruptContext` is `repr(transparent)`, so its layout is
    //         equivalent to that of `BaseType_t`.
    unsafe { &mut *ptr.cast() }
  }

  /// Get the pointer to the contained `BaseType_t` for passing it to a FreeRTOS API function.
  pub fn as_ptr(&mut self) -> *mut BaseType_t {
    &mut self.x_higher_priority_task_woken
  }
}

impl Drop for InterruptContext {
  fn drop(&mut self) {
    unsafe { freertos_rs_yield_from_isr(self.x_higher_priority_task_woken) }
  }
}

use crate::base::*;
use crate::shim::*;

/// Keep track of whether we need to yield the execution to a different
/// task at the end of the interrupt.
///
/// Should be dropped as the last thing inside an interrupt.
#[repr(transparent)]
#[must_use]
pub struct InterruptContext {
    x_higher_priority_task_woken: FreeRtosBaseType,
}

impl InterruptContext {
    /// Instantiate a new context.
    pub fn new() -> InterruptContext {
        InterruptContext {
            x_higher_priority_task_woken: 0,
        }
    }

    pub fn x_higher_priority_task_woken(&mut self) -> *mut FreeRtosBaseType {
        &mut self.x_higher_priority_task_woken
    }
}

impl Drop for InterruptContext {
    fn drop(&mut self) {
      unsafe {
        freertos_rs_yield_from_isr(self.x_higher_priority_task_woken);
      }
    }
}

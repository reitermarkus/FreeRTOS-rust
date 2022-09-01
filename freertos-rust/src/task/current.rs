use crate::shim::vTaskDelay;
use crate::Ticks;

/// A task that is currently executing.
pub struct CurrentTask;

impl CurrentTask {
  /// Delay the execution of the current task.
  pub fn delay<T: Into<Ticks>>(delay: T) {
    unsafe { vTaskDelay(delay.into().as_ticks()) }
  }
}

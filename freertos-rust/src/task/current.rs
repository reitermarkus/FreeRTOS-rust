use crate::DurationTicks;
use crate::shim::vTaskDelay;

/// A task that is currently executing.
pub struct CurrentTask;

impl CurrentTask {
  /// Delay the execution of the current task.
  pub fn delay<D: DurationTicks>(delay: D) {
    unsafe { vTaskDelay(delay.to_ticks()) }
  }
}

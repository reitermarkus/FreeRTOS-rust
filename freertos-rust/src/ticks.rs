use core::time::Duration;

use crate::shim::{portMAX_DELAY, portTICK_PERIOD_MS};
use crate::ffi::TickType_t;

/// Duration in FreeRTOS ticks.
///
/// This type represents a duration in ticks. The duration of a single tick
/// depends on `portTICK_PERIOD_MS`.
///
/// All blocking API functions support any type which can be converted to
/// `Ticks`. In particular, you can pass a [`Duration`] seamlessly with the
/// following behaviour:
///
/// - `Duration::ZERO` makes an API call non-blocking and it will return immediately.
/// - `Duration::MAX` blocks an API call until it completes. This is true for any
///   `Duration` which exceeds `portMAX_DELAY` ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Ticks {
  pub(crate) ticks: TickType_t,
}

impl Ticks {
  /// Create `Ticks` from raw ticks.
  pub const fn new(ticks: TickType_t) -> Self {
    Self { ticks }
  }

  /// Create `Ticks` from milliseconds.
  pub const fn from_millis(ms: u32) -> Self {
    let ticks = ms / portTICK_PERIOD_MS as u32;
    Self { ticks }
  }
}

impl From<Ticks> for TickType_t {
  fn from(ticks: Ticks) -> Self {
    ticks.ticks
  }
}

impl From<TickType_t> for Ticks {
  fn from(ticks: TickType_t) -> Self {
    Self::new(ticks)
  }
}

impl From<Duration> for Ticks {
  /// Convert a `Duration` to `Ticks`.
  fn from(duration: Duration) -> Self {
    let ticks = duration.as_millis() / portTICK_PERIOD_MS as u128;
    Self::new(ticks.try_into().unwrap_or(portMAX_DELAY))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn duration_max_gte_port_max_delay() {
    assert_eq!(Ticks::from(Duration::MAX), Ticks::new(portMAX_DELAY));
  }
}

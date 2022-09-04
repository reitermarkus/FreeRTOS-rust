use crate::FreeRtosError;
use crate::Ticks;
use crate::alloc::Dynamic;

use super::{Timer, TimerHandle};

/// Helper struct for creating a new [`Timer`].
pub struct TimerBuilder<'a> {
  pub(super) name: Option<&'a str>,
  pub(super) period: Ticks,
  pub(super) auto_reload: bool,
}

impl TimerBuilder<'_> {
  /// Set the name of the timer.
  pub const fn name<'a>(self, name: &'a str) -> TimerBuilder<'a> {
    TimerBuilder {
      name: Some(name),
      period: self.period,
      auto_reload: self.auto_reload,
    }
  }

  /// Set the period of the timer.
  pub const fn period(mut self, period: impl Into<Ticks>) -> Self {
    self.period = period.into();
    self
  }

  /// Should the timer be automatically reloaded?
  pub const fn auto_reload(mut self, auto_reload: bool) -> Self {
    self.auto_reload = auto_reload;
    self
  }

  /// Create the [`Timer`].
  ///
  /// Note that the newly created timer must be started.
  pub fn create<'f, F>(self, callback: F) -> Result<Timer<'f, Dynamic>, FreeRtosError>
  where
    F: Fn(&TimerHandle) + Send + 'f,
  {
    Timer::spawn(
      self.name,
      self.period,
      self.auto_reload,
      callback,
    )
  }
}

use core::ffi::CStr;
use core::marker::PhantomData;

use alloc2::{boxed::Box};

use crate::Ticks;
use crate::alloc::Dynamic;
use crate::alloc::Static;
use crate::lazy_init::LazyInit;
use crate::lazy_init::LazyPtr;

use super::{Timer, TimerMeta, TimerHandle};

/// Helper struct for creating a new timer returned by [`Timer::new`].
pub struct TimerBuilder<'n> {
  pub(super) name: Option<&'n CStr>,
  pub(super) period: Ticks,
  pub(super) auto_reload: bool,
}

impl<'n> TimerBuilder<'n> {
  /// Set the name of the timer.
  pub const fn name<'a>(self, name: &'a CStr) -> TimerBuilder<'a> {
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

  /// Create the dynamic [`Timer`].
  ///
  /// Note that the newly created timer must be started.
  #[must_use]
  pub fn create<'f, F>(self, callback: F) -> Timer<'f, Dynamic>
  where
    F: Fn(&TimerHandle) + Send + 'f,
    'n: 'f,
  {
    let meta: <Timer<'f, Dynamic> as LazyInit>::Data = TimerMeta {
      name: self.name,
      period: self.period.into(),
      auto_reload: self.auto_reload,
      callback: Box::pin(Box::new(callback)),
    };

    Timer {
      alloc_type: PhantomData,
      handle: LazyPtr::new(meta),
    }
  }
}

impl TimerBuilder<'static> {
  /// Create the static [`Timer`].
  ///
  /// Note that the newly created timer must be started.
  ///
  /// # Safety
  ///
  /// The returned timer must have a `'static` lifetime.
  ///
  /// # Examples
  ///
  /// ```
  /// use freertos_rust::{alloc::Static, timer::Timer};
  ///
  /// fn my_timer_callback(timer: &TimerHandle) {
  ///   // ...
  /// }
  ///
  /// // SAFETY: Assignment to a `static` ensures a `'static` lifetime.
  /// static TIMER: Timer<Static> = unsafe {
  ///   Timer::new().period(200).create_static(my_timer_callback)
  /// };
  /// ```
  #[must_use]
  pub const unsafe fn create_static(self, callback: fn(&TimerHandle)) -> Timer<'static, Static> {
    let meta = TimerMeta {
      name: self.name,
      period: self.period.into(),
      auto_reload: self.auto_reload,
      callback,
    };

    Timer {
      alloc_type: PhantomData,
      handle: LazyPtr::new(meta),
    }
  }
}

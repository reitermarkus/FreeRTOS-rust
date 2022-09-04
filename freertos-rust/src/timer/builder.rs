use core::ffi::CStr;
use core::marker::PhantomData;
use core::pin::Pin;

use alloc2::boxed::Box;

use crate::Ticks;
use crate::alloc::Dynamic;
use crate::alloc::Static;
use crate::lazy_init::LazyPtr;

use super::{Timer, TimerMeta, TimerHandle};

/// Helper struct for creating a new [`Timer`].
pub struct TimerBuilder<'a> {
  pub(super) name: Option<&'a CStr>,
  pub(super) period: Ticks,
  pub(super) auto_reload: bool,
}

impl<'a> TimerBuilder<'a> {
  /// Set the name of the timer.
  pub const fn name<'b>(self, name: &'b CStr) -> TimerBuilder<'b> {
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
  pub fn create<'f, F>(self, callback: F) -> Timer<'f, Dynamic>
  where
    F: Fn(&TimerHandle) + Send + 'f,
    'a: 'f,
  {
    let meta: TimerMeta<Pin<Box<Box<dyn Fn(&TimerHandle) + Send + 'f>>>> = TimerMeta {
      name: self.name,
      period: self.period.as_ticks(),
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
  /// The returned timer must be [pinned](core::pin) before using it.
  ///
  /// # Examples
  ///
  /// ```
  /// use core::pin::Pin;
  /// use freertos_rust::timer::Timer;
  ///
  /// fn my_timer_callback(timer: &TimerHandle) {
  ///   // ...
  /// }
  ///
  /// // SAFETY: Assignment to a `static` ensures the timer will never move.
  /// pub static TIMER: Pin<Timer<Static>> = unsafe {
  ///   Pin::new_unchecked(Timer::new(Ticks::new(200).create_static(my_timer_callback)))
  /// };
  /// ```
  pub const unsafe fn create_static(self, callback: fn(&TimerHandle)) -> Timer<'static, Static> {
    let meta = TimerMeta {
      name: self.name,
      period: self.period.as_ticks(),
      auto_reload: self.auto_reload,
      callback,
    };

    Timer {
      alloc_type: PhantomData,
      handle: LazyPtr::new(meta),
    }
  }
}

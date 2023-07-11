use core::{ptr, ffi::CStr};
#[cfg(freertos_feature = "static_allocation")]
use core::mem::{self, MaybeUninit};

#[cfg(freertos_feature = "dynamic_allocation")]
use alloc2::boxed::Box;

use crate::{
  ffi::TimerHandle_t,
  shim::{pdFALSE, pdTRUE, pvTimerGetTimerID},
  Ticks,
};
#[cfg(freertos_feature = "dynamic_allocation")]
use crate::shim::xTimerCreate;
#[cfg(freertos_feature = "static_allocation")]
use crate::shim::xTimerCreateStatic;

use super::{Timer, StaticTimer, TimerHandle};

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
  pub const fn period(mut self, period: Ticks) -> Self {
    self.period = period;
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
  #[cfg(freertos_feature = "dynamic_allocation")]
  pub fn create<F>(self, callback: F) -> Timer
  where
    F: Fn(&TimerHandle) + Send + 'static,
  {
    extern "C" fn timer_callback(ptr: TimerHandle_t) -> () {
      unsafe {
        let handle = TimerHandle::from_ptr(ptr);

        let callback_ptr = pvTimerGetTimerID(ptr);
        let callback: &mut Box<dyn Fn(&TimerHandle)> = &mut *callback_ptr.cast();
        callback(handle);
      }
    }

    let name = if let Some(name) = self.name {
      name.as_ptr()
    } else {
      ptr::null()
    };

    let callback = Box::new(callback);
    let callback_ptr: *mut Box<dyn Fn(&TimerHandle)> = Box::into_raw(Box::new(callback));

    let ptr = unsafe {
      xTimerCreate(
        name,
        self.period.ticks,
        if self.auto_reload { pdTRUE } else { pdFALSE } as _,
        callback_ptr.cast(),
        Some(timer_callback),
      )
    };
    assert!(!ptr.is_null());

    Timer { handle: ptr }
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
  /// use core::time::Duration;
  /// use freertos_rust::{alloc::Static, timer::{Timer, TimerHandle}};
  ///
  /// fn my_timer_callback(timer: &TimerHandle) {
  ///   // ...
  /// }
  ///
  /// // SAFETY: Assignment to a `static` ensures a `'static` lifetime.
  /// static TIMER: Timer<Static> = unsafe {
  ///   Timer::new().period(200).create_static(my_timer_callback)
  /// };
  ///
  /// TIMER.start(Duration::MAX);
  /// ```
  #[must_use]
  #[cfg(freertos_feature = "static_allocation")]
  pub fn create_static(self, timer: &'static mut MaybeUninit<StaticTimer>, callback: fn(timer: &TimerHandle)) -> &'static StaticTimer {
    extern "C" fn timer_callback(ptr: TimerHandle_t) -> () {
      unsafe {
        let handle = TimerHandle::from_ptr(ptr);

        let callback_ptr = pvTimerGetTimerID(ptr);
        let callback: fn(&TimerHandle) = mem::transmute(callback_ptr);
        callback(handle);
      }
    }

    let timer_ptr = timer.as_mut_ptr();

    let name = if let Some(name) = self.name {
      name.as_ptr()
    } else {
      ptr::null()
    };

    let callback_ptr = callback as *mut _;

    unsafe {
      let ptr = xTimerCreateStatic(
        name,
        self.period.ticks,
        if self.auto_reload { pdTRUE } else { pdFALSE } as _,
        callback_ptr,
        Some(timer_callback),
        ptr::addr_of_mut!((*timer_ptr).data),
      );

      debug_assert!(!ptr.is_null());
      debug_assert_eq!(ptr, ptr::addr_of_mut!((*timer_ptr).data) as TimerHandle_t);

      timer.assume_init_ref()
    }
  }
}

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
  pub const fn name(self, name: &CStr) -> TimerBuilder<'_> {
    TimerBuilder {
      name: Some(name),
      period: self.period,
      auto_reload: self.auto_reload,
    }
  }

  /// Set the period of the timer.
  pub fn period<T: Into<Ticks>>(mut self, period: T) -> Self {
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
  #[cfg(freertos_feature = "dynamic_allocation")]
  pub fn create<F>(self, callback: F) -> Timer<'n>
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

    unsafe {
      let ptr = xTimerCreate(
        name,
        self.period.ticks,
        if self.auto_reload { pdTRUE } else { pdFALSE } as _,
        callback_ptr.cast(),
        Some(timer_callback),
      );
      assert!(!ptr.is_null());

      Timer { handle: ptr, callback: Some(Box::from_raw(callback_ptr)), name: self.name }
    }
  }

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
  /// use core::{mem::MaybeUninit, time::Duration};
  /// use freertos_rust::{StaticTimer, Timer, TimerHandle};
  ///
  /// fn my_timer_callback(timer: &TimerHandle) {
  ///   // ...
  /// }
  ///
  /// static mut TIMER: MaybeUninit<StaticTimer> = MaybeUninit::uninit();
  /// let timer = Timer::new().period(Duration::from_millis(200)).create_static(unsafe { &mut TIMER }, my_timer_callback);
  ///
  /// timer.start(Duration::MAX);
  /// ```
  #[cfg(freertos_feature = "static_allocation")]
  pub fn create_static(self, timer: &'static mut MaybeUninit<StaticTimer>, callback: fn(timer: &TimerHandle)) -> Timer<'n> {
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

      Timer {
        handle: ptr,
        #[cfg(freertos_feature = "dynamic_allocation")]
        callback: None,
        name: self.name
      }
    }
  }
}

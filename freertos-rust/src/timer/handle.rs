use core::ffi::CStr;
use core::ptr;

use crate::FreeRtosError;
use crate::InterruptContext;
use crate::Ticks;
use crate::ffi::Pointee;
use crate::ffi::TimerHandle_t;
use crate::shim::pcTimerGetName;
use crate::shim::pdFALSE;
use crate::shim::pdPASS;
use crate::shim::xTimerChangePeriod;
use crate::shim::xTimerChangePeriodFromISR;
use crate::shim::xTimerIsTimerActive;
use crate::shim::xTimerStart;
use crate::shim::xTimerStartFromISR;
use crate::shim::xTimerStop;
use crate::shim::xTimerStopFromISR;

/// A handle for managing a timer.
///
/// See [`Timer`](crate::timer::Timer) for the preferred owned version.
///
/// This type is compatible with a raw FreeRTOS [`TimerHandle_t`].
#[repr(transparent)]
pub struct TimerHandle(Pointee<TimerHandle_t>);

impl TimerHandle {
  /// Create a `TimerHandle` from a raw handle.
  ///
  /// # Safety
  ///
  /// - `ptr` must point to a valid timer.
  /// - The timer must not be deleted for the lifetime `'a` of the returned `TimerHandle`.
  #[inline]
  pub const unsafe fn from_ptr<'a>(ptr: TimerHandle_t) -> &'a Self {
    debug_assert!(!ptr.is_null());
    &*ptr.cast()
  }

  /// Get the raw timer handle.
  #[inline]
  pub const fn as_ptr(&self) -> TimerHandle_t {
    ptr::addr_of!(self.0).cast_mut()
  }

  /// Get the timer's name if it has one.
  #[inline]
  pub fn name(&self) -> Option<&CStr> {
    unsafe {
      let timer_name = pcTimerGetName(self.as_ptr());
      if timer_name.is_null() {
        None
      } else {
        Some(CStr::from_ptr(timer_name))
      }
    }
  }

  /// Check if the timer is active.
  #[inline]
  pub fn is_active(&self) -> bool {
    unsafe { xTimerIsTimerActive(self.as_ptr()) != pdFALSE }
  }

  /// Start the timer.
  #[inline]
  pub fn start(&self, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
    match unsafe { xTimerStart(self.as_ptr(), timeout.into().into()) } {
      pdPASS => Ok(()),
      _ => Err(FreeRtosError::Timeout),
    }
  }

  /// Start the timer from an interrupt service routine.
  #[inline]
  pub fn start_from_isr(&self, ic: &InterruptContext) -> Result<(), FreeRtosError> {
    match unsafe { xTimerStartFromISR(self.as_ptr(), ic.as_ptr()) } {
      pdPASS => Ok(()),
      _ => Err(FreeRtosError::Timeout),
    }
  }

  /// Stop the timer.
  #[inline]
  pub fn stop(&self, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
    match unsafe { xTimerStop(self.as_ptr(), timeout.into().into()) } {
      pdPASS => Ok(()),
      _ => Err(FreeRtosError::Timeout),
    }
  }

  /// Stop the timer from an interrupt service routine.
  #[inline]
  pub fn stop_from_isr(&self, ic: &InterruptContext) -> Result<(), FreeRtosError> {
    match unsafe { xTimerStopFromISR(self.as_ptr(), ic.as_ptr()) } {
      pdPASS => Ok(()),
      _ => Err(FreeRtosError::Timeout),
    }
  }

  /// Get the timer's period.
  #[cfg(freertos_feature = "timer_get_period")]
  pub fn period(&self) -> Ticks {
    use crate::shim::xTimerGetPeriod;
    Ticks::new(unsafe { xTimerGetPeriod(self.as_ptr()) })
  }

  /// Change the timer's period.
  #[inline]
  pub fn change_period(
    &self,
    new_period: impl Into<Ticks>,
    timeout: impl Into<Ticks>,
  ) -> Result<(), FreeRtosError> {
    match unsafe {
      xTimerChangePeriod(
        self.as_ptr(),
        new_period.into().into(),
        timeout.into().into(),
      )
    } {
      pdPASS => Ok(()),
      _ => Err(FreeRtosError::Timeout),
    }
  }

  /// Change the timer's period from an interrupt service routine.
  #[inline]
  pub fn change_period_from_isr(
    &self,
    new_period: impl Into<Ticks>,
    id: &InterruptContext,
  ) -> Result<(), FreeRtosError> {
    match unsafe {
      xTimerChangePeriodFromISR(
        self.as_ptr(),
        new_period.into().into(),
        id.as_ptr(),
      )
    } {
      pdPASS => Ok(()),
      _ => Err(FreeRtosError::Timeout),
    }
  }
}

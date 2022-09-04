use core::mem;
use core::ptr;
use core::ptr::NonNull;

use alloc2::{
  ffi::CString,
  boxed::Box,
};

use crate::error::FreeRtosError;
use crate::InterruptContext;
use crate::shim::*;
use crate::ticks::*;
use crate::Task;
use crate::lazy_init::PtrType;

unsafe impl Send for Timer {}
unsafe impl Sync for Timer {}

/// A FreeRTOS software timer.
///
/// Note that all operations on a timer are processed by a FreeRTOS internal task
/// that receives messages in a queue. Every operation has an associated waiting time
/// for that queue to get unblocked.
pub struct Timer {
    handle: NonNull<<TimerHandle_t as PtrType>::Type>,
    detached: bool,
}

/// Helper struct for creating a new [`Timer`].
pub struct TimerBuilder<'a> {
    name: Option<&'a str>,
    period: Ticks,
    auto_reload: bool,
}

impl TimerBuilder<'_> {
  /// Set the name of the timer.
  pub const fn name<'a>(mut self, name: &'a str) -> TimerBuilder<'a> {
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
  pub fn create<F>(&self, callback: F) -> Result<Timer, FreeRtosError>
  where
    F: Fn(Timer) -> (),
    F: Send + 'static,
  {
    Timer::spawn(
      self.name,
      self.period,
      self.auto_reload,
      callback,
    )
  }
}

impl Timer {
    /// Stack size of the timer task.
    pub const STACK_SIZE: u16 = configTIMER_TASK_STACK_DEPTH;

    /// Create a new timer builder.
    pub const fn new(period: impl Into<Ticks>) -> TimerBuilder<'static> {
      TimerBuilder {
        name: None,
        period: period.into(),
        auto_reload: true,
      }
    }

    /// Create a timer from a raw handle.
    pub unsafe fn from_raw_handle(handle: TimerHandle_t) -> Self {
        Self { handle: NonNull::new_unchecked(handle), detached: false }
    }

    pub fn as_raw_handle(&self) -> TimerHandle_t {
      self.handle.as_ptr()
    }

    pub fn daemon_task() -> Task {
        unsafe { Task::from_raw_handle(xTimerGetTimerDaemonTaskHandle()) }
    }

    unsafe fn spawn_inner<'a>(
        name: Option<&str>,
        period_ticks: TickType_t,
        auto_reload: bool,
        callback: Box<dyn Fn(Timer) + Send + 'a>,
    ) -> Result<Timer, FreeRtosError> {
        let name = match name.map(CString::new) {
            None => None,
            Some(Ok(name)) => Some(name.into_boxed_c_str()),
            Some(_) => return Err(FreeRtosError::StringConversionError)
        };

        let f = Box::new(callback);
        let param_ptr = &*f as *const _ as *mut _;

        extern "C" fn timer_callback(handle: TimerHandle_t) -> () {
            unsafe {
                {
                    let timer = Timer {
                        handle: NonNull::new_unchecked(handle),
                        detached: true,
                    };
                    let callback_ptr = pvTimerGetTimerID(handle);
                    let b = Box::from_raw(callback_ptr as *mut Box<dyn Fn(Timer)>);
                    b(timer);
                    Box::into_raw(b);
                }
            }
        }

        let timer_handle = unsafe {
          xTimerCreate(
            name.as_ref().map(|n| n.as_ptr()).unwrap_or(ptr::null()),
            period_ticks,
            if auto_reload { pdTRUE } else { pdFALSE } as _,
            param_ptr,
            Some(timer_callback),
          )
        };

        match NonNull::new(timer_handle) {
          Some(handle) => {
            mem::forget(f);
            mem::forget(name);

            Ok(Timer { handle, detached: false })
          },
          None => Err(FreeRtosError::OutOfMemory)
        }
    }

    fn spawn<F>(
        name: Option<&str>,
        period_tick: Ticks,
        auto_reload: bool,
        callback: F,
    ) -> Result<Timer, FreeRtosError>
    where
        F: Fn(Timer) -> (),
        F: Send + 'static,
    {
        unsafe { Timer::spawn_inner(name, period_tick.as_ticks(), auto_reload, Box::new(callback)) }
    }

    /// Start the timer.
    pub fn start(&self, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
        let res = unsafe {
          xTimerStart(self.handle.as_ptr(), timeout.into().as_ticks())
        };

        match res {
          pdPASS => Ok(()),
          _ => Err(FreeRtosError::Timeout),
        }
    }

    /// Start the timer from an interrupt.
    pub fn start_from_isr(&self, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
        let res = unsafe {
          xTimerStartFromISR(self.handle.as_ptr(), ic.as_ptr())
        };

        match res {
          pdPASS => Ok(()),
          _ => Err(FreeRtosError::Timeout),
        }
    }

    /// Stop the timer.
    pub fn stop(&self, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
        let res = unsafe {
          xTimerStop(self.handle.as_ptr(), timeout.into().as_ticks())
        };

        match res {
          pdPASS => Ok(()),
          _ => Err(FreeRtosError::Timeout),
        }
  }

    pub fn is_active(&self) -> bool {
        unsafe { xTimerIsTimerActive(self.handle.as_ptr()) != 0 }
    }

    /// Change the period of the timer.
    pub fn change_period(
        &self,
        new_period: impl Into<Ticks>,
        timeout: impl Into<Ticks>,
    ) -> Result<(), FreeRtosError> {
        unsafe {
            if xTimerChangePeriod(
                self.handle.as_ptr(),
                new_period.into().as_ticks(),
                timeout.into().as_ticks(),
            ) == pdTRUE {
                return Ok(())
            }

            Err(FreeRtosError::Timeout)
        }
    }

    /// Detach this timer from Rust's memory management. The timer will still be active and
    /// will consume the memory.
    ///
    /// Can be used for timers that will never be changed and don't need to stay in scope.
    pub unsafe fn detach(mut self) {
        self.detached = true;
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if self.detached == true {
            return;
        }

        unsafe {
            let task_name_ptr = pcTimerGetName(self.handle.as_ptr());
            let task_name = CString::from_raw(task_name_ptr.cast_mut());

            let callback_ptr = pvTimerGetTimerID(self.handle.as_ptr());
            let callback = Box::from_raw(callback_ptr as *mut Box<dyn Fn(Timer)>);

            xTimerDelete(self.handle.as_ptr(), portMAX_DELAY);

            drop(task_name);
            drop(callback);
        }
    }
}

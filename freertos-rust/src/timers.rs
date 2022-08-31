use core::ptr::NonNull;

use alloc::ffi::CString;

use crate::InterruptContext;
use crate::base::*;
use crate::prelude::v1::*;
use crate::shim::*;
use crate::units::*;
use crate::Task;

unsafe impl Send for Timer {}
unsafe impl Sync for Timer {}

/// A FreeRTOS software timer.
///
/// Note that all operations on a timer are processed by a FreeRTOS internal task
/// that receives messages in a queue. Every operation has an associated waiting time
/// for that queue to get unblocked.
pub struct Timer {
    handle: NonNull<CVoid>,
    detached: bool,
}

/// Helper builder for a new software timer.
pub struct TimerBuilder<D: DurationTicks> {
    name: String,
    period: D,
    auto_reload: bool,
}

impl<D: DurationTicks> TimerBuilder<D> {
    /// Set the name of the timer.
    pub fn set_name(&mut self, name: &str) -> &mut Self {
        self.name = name.into();
        self
    }

    /// Set the period of the timer.
    pub fn set_period(&mut self, period: D) -> &mut Self {
        self.period = period;
        self
    }

    /// Should the timer be automatically reloaded?
    pub fn set_auto_reload(&mut self, auto_reload: bool) -> &mut Self {
        self.auto_reload = auto_reload;
        self
    }

    /// Try to create the new timer.
    ///
    /// Note that the newly created timer must be started.
    pub fn create<F>(&self, callback: F) -> Result<Timer, FreeRtosError>
    where
        F: Fn(Timer) -> (),
        F: Send + 'static,
    {
        Timer::spawn(
            self.name.as_str(),
            self.period.to_ticks(),
            self.auto_reload,
            callback,
        )
    }
}

impl Timer {
    pub const STACK_SIZE: u16 = configTIMER_TASK_STACK_DEPTH;

    /// Create a new timer builder.
    pub fn new<D: DurationTicks>(period: D) -> TimerBuilder<D> {
        TimerBuilder {
            name: "timer".into(),
            period: period,
            auto_reload: true,
        }
    }

    /// Create a timer from a raw handle.
    pub unsafe fn from_raw_handle(handle: FreeRtosTimerHandle) -> Self {
        Self { handle: NonNull::new_unchecked(handle), detached: false }
    }

    pub fn as_raw_handle(&self) -> FreeRtosTimerHandle {
      self.handle.as_ptr()
    }

    pub fn daemon_task() -> Task {
        unsafe { Task::from_raw_handle(xTimerGetTimerDaemonTaskHandle()) }
    }

    unsafe fn spawn_inner<'a>(
        name: &str,
        period_ticks: FreeRtosTickType,
        auto_reload: bool,
        callback: Box<dyn Fn(Timer) + Send + 'a>,
    ) -> Result<Timer, FreeRtosError> {
        let name = if let Ok(name) = CString::new(name) {
          name.into_boxed_c_str()
        } else {
          return Err(FreeRtosError::StringConversionError)
        };

        let f = Box::new(callback);
        let param_ptr = &*f as *const _ as *mut _;

        extern "C" fn timer_callback(handle: FreeRtosTimerHandle) -> () {
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
            name.as_ptr(),
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
        name: &str,
        period_tick: FreeRtosTickType,
        auto_reload: bool,
        callback: F,
    ) -> Result<Timer, FreeRtosError>
    where
        F: Fn(Timer) -> (),
        F: Send + 'static,
    {
        unsafe { Timer::spawn_inner(name, period_tick, auto_reload, Box::new(callback)) }
    }

    /// Start the timer.
    pub fn start<D: DurationTicks>(&self, block_time: D) -> Result<(), FreeRtosError> {
        let res = unsafe {
          xTimerStart(self.handle.as_ptr(), block_time.to_ticks())
        };

        match res {
          pdPASS => Ok(()),
          _ => Err(FreeRtosError::Timeout),
        }
    }

    /// Start the timer from an interrupt.
    pub fn start_from_isr(&self, context: &mut InterruptContext) -> Result<(), FreeRtosError> {
        let res = unsafe {
          xTimerStartFromISR(self.handle.as_ptr(), context.x_higher_priority_task_woken())
        };

        match res {
          pdPASS => Ok(()),
          _ => Err(FreeRtosError::Timeout),
        }
    }

    /// Stop the timer.
    pub fn stop<D: DurationTicks>(&self, block_time: D) -> Result<(), FreeRtosError> {
        let res = unsafe {
          xTimerStop(self.handle.as_ptr(), block_time.to_ticks())
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
    pub fn change_period<D: DurationTicks>(
        &self,
        block_time: D,
        new_period: D,
    ) -> Result<(), FreeRtosError> {
        unsafe {
            if xTimerChangePeriod(
                self.handle.as_ptr(),
                block_time.to_ticks(),
                new_period.to_ticks(),
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

            xTimerDelete(self.handle.as_ptr(), Duration::infinite().to_ticks());

            drop(task_name);
            drop(callback);
        }
    }
}

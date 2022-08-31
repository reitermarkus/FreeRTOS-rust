use core::ffi::CStr;
use core::fmt;
use core::mem;
use core::mem::MaybeUninit;
use core::ptr;
use core::ptr::NonNull;

#[cfg(feature = "alloc")]
use alloc::{
  boxed::Box,
  ffi::CString,
  string::{String, ToString},
  vec::Vec,
};

use crate::base::*;
use crate::isr::*;
use crate::shim::*;
use crate::units::*;

/// Handle for a FreeRTOS task
#[derive(Debug, Clone)]
pub struct Task {
    handle: NonNull<CVoid>,
}

unsafe impl Send for Task {}

/// Task's execution priority. Low priority numbers denote low priority tasks.
#[derive(Debug, Copy, Clone)]
pub struct TaskPriority(pub u8);

/// Notification to be sent to a task.
#[derive(Debug, Copy, Clone)]
pub enum TaskNotification {
    /// Send the event, unblock the task, the task's notification value isn't changed.
    NoAction,
    /// Perform a logical or with the task's notification value.
    SetBits(u32),
    /// Increment the task's notification value by one.
    Increment,
    /// Set the task's notification value to this value.
    OverwriteValue(u32),
    /// Try to set the task's notification value to this value. Succeeds
    /// only if the task has no pending notifications. Otherwise, the
    /// notification call will fail.
    SetValue(u32),
}

impl TaskNotification {
    fn to_freertos(&self) -> (UBaseType_t, eNotifyAction) {
        match *self {
            TaskNotification::NoAction => (0, eNotifyAction_eNoAction),
            TaskNotification::SetBits(v) => (v, eNotifyAction_eSetBits),
            TaskNotification::Increment => (0, eNotifyAction_eIncrement),
            TaskNotification::OverwriteValue(v) => (v, eNotifyAction_eSetValueWithOverwrite),
            TaskNotification::SetValue(v) => (v, eNotifyAction_eSetValueWithoutOverwrite),
        }
    }
}

impl TaskPriority {
    fn to_freertos(&self) -> UBaseType_t {
        self.0.into()
    }
}

/// Helper for spawning a new task. Instantiate with [`Task::new()`].
///
/// [`Task::new()`]: struct.Task.html#method.new
pub struct TaskBuilder {
    task_name: String,
    task_stack_size: u16,
    task_priority: TaskPriority,
}

impl TaskBuilder {
    /// Set the task's name.
    pub fn name(&mut self, name: &str) -> &mut Self {
        self.task_name = name.into();
        self
    }

    /// Set the stack size, in words.
    pub fn stack_size(&mut self, stack_size: u16) -> &mut Self {
        self.task_stack_size = stack_size;
        self
    }

    /// Set the task's priority.
    pub fn priority(&mut self, priority: TaskPriority) -> &mut Self {
        self.task_priority = priority;
        self
    }

    /// Start a new task that can't return a value.
    pub fn start<F>(&self, func: F) -> Result<Task, FreeRtosError>
    where
        F: FnOnce(Task) -> (),
        F: Send + 'static,
    {
        Task::spawn(
            &self.task_name,
            self.task_stack_size,
            self.task_priority,
            func,
        )
    }
}

impl Task {
    /// Minimal task stack size.
    pub const MINIMAL_STACK_SIZE: u16 = configMINIMAL_STACK_SIZE;

    /// Prepare a builder object for the new task.
    pub fn new() -> TaskBuilder {
        TaskBuilder {
            task_name: "rust_task".into(),
            task_stack_size: 1024,
            task_priority: TaskPriority(1),
        }
    }

    pub unsafe fn from_raw_handle(handle: FreeRtosTaskHandle) -> Self {
      Self { handle: NonNull::new_unchecked(handle) }
    }

    pub fn as_raw_handle(&self) -> FreeRtosTaskHandle {
      self.handle.as_ptr()
    }

    pub fn suspend(&self) {
        unsafe { vTaskSuspend(self.handle.as_ptr()) }
    }

    pub fn suspend_all() {
      unsafe { vTaskSuspendAll() }
    }

    pub fn resume_all() -> bool {
        unsafe { xTaskResumeAll() == pdTRUE }
    }

    unsafe fn spawn_inner<'a>(
        f: Box<dyn FnOnce(Task)>,
        name: &str,
        stack_size: u16,
        priority: TaskPriority,
    ) -> Result<Task, FreeRtosError> {
        let name = name.as_bytes();
        let mut c_name = [0; configMAX_TASK_NAME_LEN as usize];
        let name_len = name.len().min(c_name.len() - 1);
        c_name[..name_len].copy_from_slice(&name[..name_len]);

        let f = Box::new(f);
        let param_ptr = &*f as *const _ as *mut _;

        let mut task_handle = ptr::null_mut();

        extern "C" fn thread_start(main: *mut CVoid) {
            unsafe {
                // NOTE: New scope so that everything is dropped before the task is deleted.
                {
                    let b = Box::from_raw(main as *mut Box<dyn FnOnce(Task)>);

                    let task = Task {
                      handle: NonNull::new_unchecked(xTaskGetCurrentTaskHandle()),
                    };
                    b(task);

                    let task_name_ptr = pcTaskGetName(ptr::null_mut());
                    let task_name = CString::from_raw(task_name_ptr);
                    drop(task_name);
                }

                vTaskDelete(ptr::null_mut());
            }
        }

        let ret = unsafe {
          xTaskCreate(
            Some(thread_start),
            name.as_ptr().cast(),
            stack_size,
            param_ptr,
            priority.to_freertos(),
            &mut task_handle,
          )
        };

        match ret {
          pdPASS if !task_handle.is_null() => {
            mem::forget(f);
            mem::forget(name);

            Ok(Task::from_raw_handle(task_handle))
          },
          errCOULD_NOT_ALLOCATE_REQUIRED_MEMORY => Err(FreeRtosError::OutOfMemory),
          _ => unreachable!(),
        }
    }

    fn spawn<F>(
        name: &str,
        stack_size: u16,
        priority: TaskPriority,
        f: F,
    ) -> Result<Task, FreeRtosError>
    where
        F: FnOnce(Task) -> (),
        F: Send + 'static,
    {
        unsafe {
            return Task::spawn_inner(Box::new(f), name, stack_size, priority);
        }
    }

    /// Get the name of the current task.
    pub fn get_name(&self) -> &CStr {
        unsafe {
            let task_name = pcTaskGetName(self.handle.as_ptr());
            assert!(!task_name.is_null());
            CStr::from_ptr(task_name)
        }
    }

    /// Try to find the task of the current execution context.
    pub fn current() -> Result<Task, FreeRtosError> {
        unsafe {
            match NonNull::new(xTaskGetCurrentTaskHandle()) {
              Some(handle) => Ok(Task { handle }),
              None => Err(FreeRtosError::TaskNotFound),
            }
        }
    }

    /// Forcibly set the notification value for this task.
    pub fn set_notification_value(&self, val: u32) {
        let _ = self.notify(TaskNotification::OverwriteValue(val));
    }

    /// Take the notification and either clear the notification value or decrement it by one.
    pub fn take_notification<D: DurationTicks>(clear: bool, wait_for: D) -> u32 {
      unsafe {
        ulTaskNotifyTake(if clear { pdTRUE } else { pdFALSE }, wait_for.to_ticks())
      }
    }

    /// Notify this task.
    pub fn notify(&self, notification: TaskNotification) -> Result<(), FreeRtosError> {
      unsafe {
          let n = notification.to_freertos();
          if xTaskNotify(self.handle.as_ptr(), n.0, n.1) == pdPASS {
            return Ok(())
          }
      }

      Err(FreeRtosError::QueueFull)
    }

    /// Notify this task with the given index.
    pub fn notify_indexed(&self, index: u32, notification: TaskNotification) -> Result<(), FreeRtosError> {
      unsafe {
          let n = notification.to_freertos();
          if freertos_rs_task_notify_indexed(self.handle.as_ptr(), index, n.0, n.1) == pdPASS {
            return Ok(())
          }
      }

      Err(FreeRtosError::QueueFull)
    }

    /// Notify this task from an interrupt.
    pub fn notify_from_isr(
        &self,
        notification: TaskNotification,
        context: &mut InterruptContext,
    ) -> Result<(), FreeRtosError> {
        unsafe {
            let n = notification.to_freertos();
            let t = xTaskNotifyFromISR(
                self.handle.as_ptr(),
                n.0,
                n.1,
                context.x_higher_priority_task_woken(),
            );
            if t == pdPASS {
                return Ok(())
            }
        }

        Err(FreeRtosError::QueueFull)
    }

    /// Notify this task from an interrupt with the given index.
    pub fn notify_indexed_from_isr(
      &self,
      index: u32,
      notification: TaskNotification,
      context: &mut InterruptContext,
    ) -> Result<(), FreeRtosError> {
        unsafe {
            let n = notification.to_freertos();
            let t = freertos_rs_task_notify_indexed_from_isr(
                self.handle.as_ptr(),
                index,
                n.0,
                n.1,
                context.x_higher_priority_task_woken(),
            );
            if t == pdPASS {
              return Ok(())
          }
      }

      Err(FreeRtosError::QueueFull)
    }

    /// Wait for a notification to be posted.
    pub fn wait_for_notification<D: DurationTicks>(
        &self,
        clear_bits_enter: u32,
        clear_bits_exit: u32,
        wait_for: D,
    ) -> Result<u32, FreeRtosError> {
        let mut val = 0;
        let r = unsafe {
            xTaskNotifyWait(
                clear_bits_enter,
                clear_bits_exit,
                &mut val as *mut _,
                wait_for.to_ticks(),
            )
        };

        if r == pdPASS {
            return Ok(val)
          }
      Err(FreeRtosError::Timeout)
    }

    /// Get the minimum amount of stack that was ever left on this task.
    pub fn get_stack_high_water_mark(&self) -> u32 {
        unsafe { freertos_rs_get_stack_high_water_mark(self.handle.as_ptr()) as u32 }
    }
}

/// Helper methods to be performed on the task that is currently executing.
pub struct CurrentTask;

impl CurrentTask {
    /// Delay the execution of the current task.
    pub fn delay<D: DurationTicks>(delay: D) {
        unsafe {
          vTaskDelay(delay.to_ticks());
        }
    }
}

#[derive(Debug)]
pub struct FreeRtosSystemState {
    pub tasks: Vec<FreeRtosTaskStatus>,
    pub total_run_time: u32,
}

impl fmt::Display for FreeRtosSystemState {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.write_str("FreeRTOS tasks\r\n")?;

        write!(fmt, "{id: <6} | {name: <16} | {state: <9} | {priority: <8} | {stack: >10} | {cpu_abs: >10} | {cpu_rel: >4}\r\n",
               id = "ID",
               name = "Name",
               state = "State",
               priority = "Priority",
               stack = "Stack left",
               cpu_abs = "CPU",
               cpu_rel = "%"
        )?;

        for task in &self.tasks {
            write!(fmt, "{id: <6} | {name: <16} | {state: <9} | {priority: <8} | {stack: >10} | {cpu_abs: >10} | {cpu_rel: >4}\r\n",
                   id = task.task_number,
                   name = task.name,
                   state = format!("{:?}", task.task_state),
                   priority = task.current_priority.0,
                   stack = task.stack_high_water_mark,
                   cpu_abs = task.run_time_counter,
                   cpu_rel = if self.total_run_time > 0 && task.run_time_counter <= self.total_run_time {
                       let p = (((task.run_time_counter as u64) * 100) / self.total_run_time as u64) as u32;
                       let ps = if p == 0 && task.run_time_counter > 0 {
                           "<1".to_string()
                       } else {
                           p.to_string()
                       };
                       format!("{: >3}%", ps)
                   } else {
                       "-".to_string()
                   }
            )?;
        }

        if self.total_run_time > 0 {
            write!(fmt, "Total run time: {}\r\n", self.total_run_time)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct FreeRtosTaskStatus {
    pub task: Task,
    pub name: String,
    pub task_number: UBaseType_t,
    pub task_state: FreeRtosTaskState,
    pub current_priority: TaskPriority,
    pub base_priority: TaskPriority,
    pub run_time_counter: FreeRtosUnsignedLong,
    pub stack_high_water_mark: FreeRtosUnsignedShort,
}

pub struct FreeRtosUtils;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FreeRtosSchedulerState {
  Suspended,
  NotStarted,
  Running
}

impl FreeRtosUtils {
    /// Start scheduling tasks.
    pub fn start_scheduler() -> ! {
        unsafe { vTaskStartScheduler() };
        unreachable!()
    }

    /// Get the current scheduler state.
    pub fn scheduler_state() -> FreeRtosSchedulerState {
      unsafe {
        match xTaskGetSchedulerState() {
          0 => FreeRtosSchedulerState::Suspended,
          1 => FreeRtosSchedulerState::NotStarted,
          2 => FreeRtosSchedulerState::Running,
          _ => unreachable!(),
        }
      }
    }

    pub fn get_tick_count() -> FreeRtosTickType {
        unsafe { xTaskGetTickCount() }
    }

    pub fn get_tick_count_duration() -> Duration {
        Duration::ticks(Self::get_tick_count())
    }

    pub fn get_number_of_tasks() -> usize {
        unsafe { uxTaskGetNumberOfTasks() as usize }
    }

    pub fn get_all_tasks(tasks_len: Option<usize>) -> FreeRtosSystemState {
        let tasks_len = tasks_len.unwrap_or(Self::get_number_of_tasks());
        let mut tasks = Vec::with_capacity(tasks_len as usize);
        let mut total_run_time = 0;

        unsafe {
            let filled = uxTaskGetSystemState(
                MaybeUninit::slice_as_mut_ptr(tasks.spare_capacity_mut()),
                tasks_len as FreeRtosUBaseType,
                &mut total_run_time,
            );
            tasks.set_len(filled as usize);
        }

        let tasks = tasks
            .into_iter()
            .map(|t| {
              let name = unsafe { CStr::from_ptr(t.pcTaskName) };
              let name = name.to_str().unwrap_or("?");

              FreeRtosTaskStatus {
                  task: Task {
                      handle: unsafe { NonNull::new_unchecked(t.xHandle) },
                  },
                  name: String::from(name),
                  task_number: t.xTaskNumber,
                  task_state: t.eCurrentState.into(),
                  current_priority: TaskPriority(t.uxCurrentPriority as u8),
                  base_priority: TaskPriority(t.uxBasePriority as u8),
                  run_time_counter: t.ulRunTimeCounter,
                  stack_high_water_mark: t.usStackHighWaterMark,
              }
            })
            .collect();

        FreeRtosSystemState {
            tasks: tasks,
            total_run_time: total_run_time,
        }
    }
}

use core::{fmt, ffi::{c_ulong, c_ushort}};

use alloc2::{
  string::ToString,
  vec::Vec,
};

use crate::shim::{
  UBaseType_t,
  configMAX_TASK_NAME_LEN,
};

use super::{Task, TaskPriority, TaskState, TaskName};

#[derive(Debug)]
pub struct TaskStatus {
  pub(crate) handle: Task,
  pub(crate) name: TaskName<{ configMAX_TASK_NAME_LEN as usize }>,
  pub(crate) number: UBaseType_t,
  pub(crate) state: TaskState,
  pub(crate) current_priority: TaskPriority,
  pub(crate) base_priority: TaskPriority,
  pub(crate) run_time_counter: c_ulong,
  pub(crate) stack_high_water_mark: c_ushort,
}

impl TaskStatus {
  /// Get the task.
  #[inline]
  pub fn task(&self) -> &Task {
    &self.handle
  }

  /// Get the task name.
  #[inline]
  pub fn name(&self) -> &str {
    self.name.as_str()
  }

  /// Get the task number.
  #[inline]
  pub fn number(&self) -> UBaseType_t {
    self.number
  }

  /// Get the task state.
  #[inline]
  pub fn state(&self) -> TaskState {
    self.state
  }

  /// Get the task's current priority.
  pub fn current_priority(&self) -> TaskPriority {
    self.current_priority
  }

  /// Get the task's base priority.
  pub fn base_priority(&self) -> TaskPriority {
    self.base_priority
  }
}

#[derive(Debug)]
pub struct SystemState {
  pub(crate) tasks: Vec<TaskStatus>,
  pub(crate) total_run_time: u32,
}

impl fmt::Display for SystemState {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    f.write_str("FreeRTOS tasks\r\n")?;

    writeln!(
      f,
      "{id: <6} | {name: <16} | {state: <9} | {priority: <8} | {stack: >10} | {cpu_abs: >10} | {cpu_rel: >4}",
      id = "ID",
      name = "Name",
      state = "State",
      priority = "Priority",
      stack = "Stack left",
      cpu_abs = "CPU",
      cpu_rel = "%"
    )?;

    for task in &self.tasks {
      writeln!(
        f,
        "{id: <6} | {name: <16} | {state: <9} | {priority: <8} | {stack: >10} | {cpu_abs: >10} | {cpu_rel: >4}",
        id = task.number(),
        name = task.name(),
        state = format!("{:?}", task.state()),
        priority = task.current_priority,
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
      writeln!(f, "Total run time: {}", self.total_run_time)?;
    }

    Ok(())
  }
}

use alloc2::{
  sync::Arc,
};

use crate::error::*;
use crate::sync::{Mutex, Queue};
use crate::task::*;
use crate::ticks::*;

pub trait ComputeTaskBuilder {
    fn compute<const SIZE: usize, F, R>(&self, func: F) -> Result<ComputeTask<R, SIZE>, FreeRtosError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Sync + Send + 'static;
}

impl ComputeTaskBuilder for TaskBuilder<'_> {
    #[cfg(target_os = "none")]
    /// Spawn a task that can post a return value to the outside.
    fn compute<const SIZE: usize, F, R>(&self, func: F) -> Result<ComputeTask<R, SIZE>, FreeRtosError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Sync + Send + 'static,
    {
        use core::time::Duration;

        let (task, result, status) = {
            let result = Arc::new(Mutex::new(None));
            let status = Arc::new(Queue::new());

            let task_result = result.clone();
            let task_status = status.clone();
            let task = self.start(move |_this_task| {
                {
                    let mut lock = task_result.timed_lock(Duration::MAX).unwrap();
                    let r = func();
                    *lock = Some(r);
                }
                // release our reference to the mutex, so it can be deconstructed
                drop(task_result);
                task_status
                    .send(ComputeTaskStatus::Finished, Duration::MAX)
                    .unwrap();
            })?;

            (task, result, status)
        };

        Ok(ComputeTask {
            task: task,
            result: result,
            status: status,
            finished: false,
        })
    }

    #[cfg(not(target_os = "none"))]
    fn compute<const SIZE: usize, F, R>(&self, func: F) -> Result<ComputeTask<R, SIZE>, FreeRtosError>
    where
        F: FnOnce() -> R,
        F: Send + 'static,
        R: Sync + Send + 'static,
    {
        let r = func();

        Ok(ComputeTask {
            task: Task::new().start(|_this_task| {}).unwrap(),
            result: Arc::new(Mutex::new(Some(r)).unwrap()),
            status: Arc::new(Queue::new().unwrap()),
            finished: false,
        })
    }
}

/// A task that can terminate and return its return value. Implemented using an
/// atomically shared mutex.
///
/// Sample usage:
///
/// ```rust
/// # use freertos_rs::*;
/// use freertos_rs::patterns::compute_task::*;
/// let task = Task::new().compute(|| {
/// 	CurrentTask::delay(Duration::from_millis(100));
/// 	42
/// }).unwrap();
///
/// let result = task.into_result(Duration::from_millis(1000)).unwrap();
/// # println!("{}", result);
/// ```

pub struct ComputeTask<R, const SIZE: usize> {
    task: Task,
    result: Arc<Mutex<Option<R>>>,
    status: Arc<Queue<ComputeTaskStatus, SIZE>>,
    finished: bool,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
enum ComputeTaskStatus {
    Finished,
}

use core::fmt::Debug;

impl<R: Debug, const SIZE: usize> ComputeTask<R, SIZE> {
    /// Get the handle of the task that computes the result.
    pub fn get_task(&self) -> &Task {
        &self.task
    }

    /// Wait until the task computes its result. Otherwise, returns a timeout.
    pub fn wait_for_result(&mut self, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
        if self.finished == true {
            Ok(())
        } else {
            match self.status.receive(timeout) {
                Ok(ComputeTaskStatus::Finished) => {
                    self.finished = true;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    }

    /// Consume the task and unwrap the computed return value.
    pub fn into_result(mut self, timeout: impl Into<Ticks>) -> Result<R, FreeRtosError> {
        self.wait_for_result(timeout)?;

        if self.finished != true {
            panic!("ComputeTask should be finished!");
        }

        let m = Arc::try_unwrap(self.result)
            .expect("ComputeTask: Arc should have only one reference left!");
        let option_r = m.into_inner();
        let r = option_r.expect("ComputeTask: Result should be a Some(R)!");

        Ok(r)
    }
}

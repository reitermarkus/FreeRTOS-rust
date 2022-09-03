use super::SemaphoreHandle;

/// An RAII implementation of a “scoped decrement” of a semaphore.
///
/// When this structure is dropped (falls out of scope), the semaphore is incremented again.
#[must_use = concat!("if unused the `Semaphore` will increment again immediately")]
// #[must_not_suspend = "holding a `Semaphore` across suspend points can cause deadlocks, delays, \
//                       and cause Futures to not implement `Send`"]
#[derive(Debug)]
pub struct SemaphoreGuard<'s> {
  pub(super) handle: &'s SemaphoreHandle,
}

impl Drop for SemaphoreGuard<'_> {
  fn drop(&mut self) {
    let _ = self.handle.give();
  }
}

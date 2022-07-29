use core::ptr::NonNull;

use crate::base::*;
use crate::isr::*;
use crate::prelude::v1::*;
use crate::shim::*;
use crate::units::*;

unsafe impl<T> Send for Queue<T> {}
unsafe impl<T> Sync for Queue<T> {}

/// A queue with a finite size. The items are owned by the queue and are
/// copied.
#[derive(Debug)]
pub struct Queue<T> {
    handle: NonNull<CVoid>,
    item_type: PhantomData<T>,
}

impl<T: Sized + Send + Copy> Queue<T> {
    pub fn new(max_size: usize) -> Result<Queue<T>, FreeRtosError> {
        let item_size = mem::size_of::<T>();

        let handle = unsafe { freertos_rs_queue_create(max_size as u32, item_size as u32) };

        match NonNull::new(handle) {
          Some(handle) => Ok(Queue { handle, item_type: PhantomData }),
          None => Err(FreeRtosError::OutOfMemory),
        }
    }

    pub unsafe fn from_raw_handle(handle: FreeRtosQueueHandle) -> Self {
      Self {
        handle: NonNull::new_unchecked(handle),
        item_type: PhantomData,
      }
    }

    /// Send an item to the end of the queue. Wait for the queue to have empty space for it.
    pub fn send<D: DurationTicks>(&self, mut item: T, max_wait: D) -> Result<(), FreeRtosError> {
        unsafe {
            if freertos_rs_queue_send(
                self.handle.as_ptr(),
                &mut item as *mut _ as FreeRtosVoidPtr,
                max_wait.to_ticks(),
            ) != 0
            {
                Err(FreeRtosError::QueueSendTimeout)
            } else {
                Ok(())
            }
        }
    }

    /// Send an item to the end of the queue, from an interrupt.
    pub fn send_from_isr(
        &self,
        context: &mut InterruptContext,
        mut item: T,
    ) -> Result<(), FreeRtosError> {
        unsafe {
            if freertos_rs_queue_send_isr(
                self.handle.as_ptr(),
                &mut item as *mut _ as FreeRtosVoidPtr,
                context.x_higher_priority_task_woken(),
            ) != 0
            {
                Err(FreeRtosError::QueueFull)
            } else {
                Ok(())
            }
        }
    }

    /// Wait for an item to be available on the queue.
    pub fn receive<D: DurationTicks>(&self, max_wait: D) -> Result<T, FreeRtosError> {
        unsafe {
            let mut buff = mem::MaybeUninit::<T>::zeroed();
            let r = freertos_rs_queue_receive(
                self.handle.as_ptr(),
                buff.as_mut_ptr() as FreeRtosVoidPtr,
                max_wait.to_ticks(),
            );
            if r == 0 {
                return Ok(buff.assume_init());
            } else {
                return Err(FreeRtosError::QueueReceiveTimeout);
            }
        }
    }
    /// Get the number of messages in the queue.
    pub fn len(&self) -> u32 {
      unsafe {
        freertos_rs_queue_messages_waiting(self.handle.as_ptr())
      }
    }
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        unsafe {
            freertos_rs_queue_delete(self.handle.as_ptr());
        }
    }
}

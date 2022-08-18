use core::mem::MaybeUninit;
use core::mem::size_of;
use core::ptr::NonNull;

use crate::base::*;
use crate::isr::*;
use crate::prelude::v1::*;
use crate::shim::*;
use crate::units::*;

pub type StaticQueue = StaticQueue_t;

#[macro_export]
macro_rules! static_queue {
    (
      $T:ty; $size:expr
    ) => {{
      const ITEM_SIZE: usize = ::core::mem::size_of::<$T>();
      const LEN: usize = $size;
      static mut BUFFER: ::core::mem::MaybeUninit<[$T; LEN]> = ::core::mem::MaybeUninit::uninit();
      static mut STATIC_QUEUE: ::core::mem::MaybeUninit<$crate::StaticQueue> = ::core::mem::MaybeUninit::uninit();
      static mut QUEUE: ::core::mem::MaybeUninit::<Queue<$T>> = ::core::mem::MaybeUninit::uninit();
      unsafe {
        $crate::Queue::<$T>::new_static(&mut BUFFER, &mut STATIC_QUEUE, &mut QUEUE)
      }
    }};
}

macro_rules! impl_send {
    () => {
        /// Send an item to the end of the queue. Wait for the queue to have empty space for it.
        #[inline]
        pub fn send<D: DurationTicks>(&self, item: T, max_wait: D) -> Result<(), FreeRtosError> {
            unsafe { queue_send(self.handle, item, max_wait) }
        }

        /// Send an item to the end of the queue, from an interrupt.
        #[inline]
        pub fn send_from_isr(
            &self,
            ic: &mut InterruptContext,
            item: T,
        ) -> Result<(), FreeRtosError> {
            unsafe { queue_send_from_isr(self.handle, item, ic) }
        }
    };
}

macro_rules! impl_receive {
    () => {
        /// Wait for an item to be available on the queue.
        #[inline]
        pub fn receive<D: DurationTicks>(&self, max_wait: D) -> Result<T, FreeRtosError> {
            unsafe { queue_receive(self.handle, max_wait) }
        }
    };
}

/// A queue with a finite size. The items are owned by the queue and are
/// copied.
#[derive(Debug)]
pub struct Queue<T> {
    handle: NonNull<CVoid>,
    item_type: PhantomData<T>,
}

unsafe impl<T> Send for Queue<T> {}
unsafe impl<T> Sync for Queue<T> {}

impl<T: Sized + Send + Copy> Queue<T> {
    pub fn new_static<const LEN: usize>(
      buffer: &'static mut MaybeUninit<[T; LEN]>,
      static_queue: &'static mut MaybeUninit<StaticQueue>,
      queue: &'static mut MaybeUninit<Queue<T>>,
    ) -> &'static mut Self {
      unsafe {
        let handle = freertos_rs_queue_create_static(
          LEN as u32,
          size_of::<T>() as u32,
          buffer.as_mut_ptr().cast(),
          static_queue.as_mut_ptr(),
        );
        assert!(!handle.is_null());

        queue.write(Self::from_raw_handle(handle))
      }
    }

    pub fn new(max_size: usize) -> Result<Queue<T>, FreeRtosError> {
        let item_size = mem::size_of::<T>();

        let handle = unsafe { freertos_rs_queue_create(max_size as u32, item_size as u32) };

        match NonNull::new(handle) {
          Some(handle) => Ok(Queue { handle, item_type: PhantomData }),
          None => Err(FreeRtosError::OutOfMemory),
        }
    }

    /// Assign a name to the queue and add it to the registry.
    pub fn add_to_registry(&self, name: &str) {
      let mut c_name = [0; configMAX_TASK_NAME_LEN as usize];
      let bytes = name.as_bytes();
      assert!(bytes.len() < configMAX_TASK_NAME_LEN as usize);
      c_name[..bytes.len()].copy_from_slice(bytes);
      unsafe { vQueueAddToRegistry(self.handle.as_ptr(), c_name.as_ptr()) }
    }

    pub const unsafe fn from_raw_handle(handle: FreeRtosQueueHandle) -> Self {
      Self {
        handle: NonNull::new_unchecked(handle),
        item_type: PhantomData,
      }
    }

    pub fn as_raw_handle(&self) -> FreeRtosQueueHandle {
      self.handle.as_ptr()
    }

    impl_send!();
    impl_receive!();

    /// Get the number of messages in the queue.
    pub fn len(&self) -> u32 {
      unsafe {
        freertos_rs_queue_messages_waiting(self.handle.as_ptr())
      }
    }

    /// Create a sender for this queue.
    pub fn sender(&self) -> Sender<T> {
      Sender { handle: self.handle, item_type: PhantomData }
    }

    /// Create a receiver for this queue.
    pub fn receiver(&self) -> Receiver<T> {
      Receiver { handle: self.handle, item_type: PhantomData }
    }
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        unsafe {
            freertos_rs_queue_delete(self.handle.as_ptr());
        }
    }
}

/// A sender for a queue.
pub struct Sender<T: Sized + Send + Copy> {
  handle: NonNull<CVoid>,
  item_type: PhantomData<T>,
}

impl<T: Sized + Send + Copy> Sender<T> {
  impl_send!();
}

/// A receiver for a queue.
pub struct Receiver<T> {
  handle: NonNull<CVoid>,
  item_type: PhantomData<T>,
}

impl<T: Sized + Send + Copy> Receiver<T> {
  impl_receive!();
}

unsafe fn queue_send<T: Sized + Send + Copy, D: DurationTicks>(handle: NonNull<CVoid>, item: T, max_wait: D) -> Result<(), FreeRtosError> {
    let res = freertos_rs_queue_send(
      handle.as_ptr(),
      ptr::addr_of!(item).cast(),
      max_wait.to_ticks(),
    );

    if res != 0 {
        return Err(FreeRtosError::QueueSendTimeout)
    }

    Ok(())
}

unsafe fn queue_send_from_isr<T: Sized + Send + Copy>(handle: NonNull<CVoid>, item: T, ic: &mut InterruptContext) -> Result<(), FreeRtosError> {
    let res = freertos_rs_queue_send_isr(
        handle.as_ptr(),
        ptr::addr_of!(item).cast(),
        ic.x_higher_priority_task_woken(),
    );

    if res != 0 {
        return Err(FreeRtosError::QueueFull)
    }

    Ok(())
}

unsafe fn queue_receive<T: Sized + Send + Copy, D: DurationTicks>(handle: NonNull<CVoid>, max_wait: D) -> Result<T, FreeRtosError> {
  let mut item = mem::MaybeUninit::<T>::zeroed();

  let res = freertos_rs_queue_receive(
      handle.as_ptr(),
      item.as_mut_ptr() as FreeRtosVoidPtr,
      max_wait.to_ticks(),
  );

  if res != 0 {
      return Err(FreeRtosError::QueueReceiveTimeout)
  }

  Ok(item.assume_init())
}

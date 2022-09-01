use core::ffi::CStr;
use core::marker::PhantomData;
use core::mem::{MaybeUninit, size_of, self};
use core::ptr::{self, NonNull};

use crate::base::*;
use crate::isr::*;
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
          let res = unsafe {
            xQueueSend(self.handle.as_ptr(), ptr::addr_of!(item).cast(), max_wait.to_ticks())
          };

          if res == pdTRUE {
            return Ok(())
          }

          match res {
            pdTRUE => Ok(()),
            errQUEUE_FULL => Err(FreeRtosError::QueueFull),
            _ => Err(FreeRtosError::Timeout),
          }
        }

        /// Send an item to the end of the queue, from an interrupt.
        #[inline]
        pub fn send_from_isr(
            &self,
            ic: &mut InterruptContext,
            item: T,
        ) -> Result<(), FreeRtosError> {
          let res = unsafe {
            xQueueSendFromISR(self.handle.as_ptr(), ptr::addr_of!(item).cast(), ic.as_ptr())
          };

          match res {
            pdTRUE => Ok(()),
            errQUEUE_FULL => Err(FreeRtosError::QueueFull),
            _ => unreachable!(),
          }
        }
    };
}

macro_rules! impl_receive {
    () => {
        /// Wait for an item to be available on the queue.
        #[inline]
        pub fn receive<D: DurationTicks>(&self, max_wait: D) -> Result<T, FreeRtosError> {
          let mut item = MaybeUninit::<T>::zeroed();

          let res = unsafe {
            xQueueReceive(self.handle.as_ptr(), item.as_mut_ptr().cast(), max_wait.to_ticks())
          };

          match res {
            pdTRUE => Ok(unsafe { item.assume_init() }),
            _ => Err(FreeRtosError::Timeout),
          }
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
        let handle = xQueueCreateStatic(
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

        let handle = unsafe { xQueueCreate(max_size as u32, item_size as u32) };

        match NonNull::new(handle) {
          Some(handle) => Ok(Queue { handle, item_type: PhantomData }),
          None => Err(FreeRtosError::OutOfMemory),
        }
    }

    /// Assign a name to the queue and add it to the registry.
    pub fn add_to_registry(&self, name: &'static CStr) {
      unsafe {
        vQueueAddToRegistry(self.handle.as_ptr(), name.as_ptr())
      }
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
        uxQueueMessagesWaiting(self.handle.as_ptr())
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
        unsafe { vQueueDelete(self.handle.as_ptr()) }
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

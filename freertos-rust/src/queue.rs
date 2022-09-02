use core::cell::UnsafeCell;
use core::ffi::CStr;
use core::marker::PhantomData;
use core::mem::{MaybeUninit, size_of, self};
use core::ptr::{self, NonNull};

use crate::alloc::{Dynamic, Static};
use crate::lazy_init::{PtrType, LazyPtr, LazyInit};
use crate::error::*;
use crate::isr::*;
use crate::shim::*;
use crate::ticks::*;

macro_rules! impl_send {
  () => {
    /// Send an item to the end of the queue. Wait for the queue to have empty space for it.
    #[inline]
    pub fn send(&self, item: T, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
      let res = unsafe {
        xQueueSend(self.handle.as_ptr(), ptr::addr_of!(item).cast(), timeout.into().as_ticks())
      };

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
    pub fn receive(&self, timeout: impl Into<Ticks>) -> Result<T, FreeRtosError> {
      let mut item = MaybeUninit::<T>::zeroed();

      let res = unsafe {
        xQueueReceive(self.handle.as_ptr(), item.as_mut_ptr().cast(), timeout.into().as_ticks())
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
pub struct Queue<T, const SIZE: usize, A = Dynamic>
where
  Self: LazyInit<QueueHandle_t>,
{
  handle: LazyPtr<Self, QueueHandle_t>,
  item_type: PhantomData<T>,
}

impl<T, const SIZE: usize> LazyInit<QueueHandle_t> for Queue<T, SIZE, Dynamic> {
  fn init(_data: &UnsafeCell<MaybeUninit<Self::Data>>) -> Self::Ptr {
    let handle = unsafe {
      xQueueCreate(
        (mem::size_of::<T>() * SIZE) as UBaseType_t,
        size_of::<T>() as UBaseType_t,
      )
    };
    assert!(!handle.is_null());
    unsafe { Self::Ptr::new_unchecked(handle) }
  }

  fn destroy(ptr: Self::Ptr) {
    unsafe { vQueueDelete(ptr.as_ptr()) }
  }
}

impl<T, const SIZE: usize> LazyInit<QueueHandle_t> for Queue<T, SIZE, Static> {
  type Data = (MaybeUninit<StaticQueue_t>, [MaybeUninit<T>; SIZE]);

  fn init(data: &UnsafeCell<MaybeUninit<Self::Data>>) -> Self::Ptr {
    let handle = unsafe {
      // SAFETY: Data only consists of `MaybeUninit`.
      let data = &mut *data.get();
      let (queue, items) = data.assume_init_mut();

      let q = queue.as_mut_ptr();

      xQueueCreateStatic(
        SIZE as UBaseType_t,
        size_of::<T>() as UBaseType_t,
        MaybeUninit::slice_as_mut_ptr(items).cast(),
        queue.as_mut_ptr(),
      )
    };
    assert!(!handle.is_null());
    unsafe { Self::Ptr::new_unchecked(handle) }
  }

  fn destroy(ptr: Self::Ptr) {
    unsafe { vQueueDelete(ptr.as_ptr()) }
  }
}

unsafe impl<T: Send, const SIZE: usize, A> Send for Queue<T, SIZE, A>
where
  Self: LazyInit<QueueHandle_t>,
{}
unsafe impl<T: Send, const SIZE: usize, A> Sync for Queue<T, SIZE, A>
where
  Self: LazyInit<QueueHandle_t>,
{}

impl<T: Sized + Send + Copy, const SIZE: usize, A> Queue<T, SIZE, A>
where
  Self: LazyInit<QueueHandle_t>,
{
    pub const fn new() -> Self {
      Self { handle: LazyPtr::new(), item_type: PhantomData }
    }

    /// Assign a name to the queue and add it to the registry.
    pub fn add_to_registry(&self, name: &'static CStr) {
      unsafe {
        vQueueAddToRegistry(self.handle.as_ptr(), name.as_ptr())
      }
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
      Sender { handle: unsafe { NonNull::new_unchecked(self.handle.as_ptr()) }, item_type: PhantomData }
    }

    /// Create a receiver for this queue.
    pub fn receiver(&self) -> Receiver<T> {
      Receiver { handle: unsafe { NonNull::new_unchecked(self.handle.as_ptr()) }, item_type: PhantomData }
    }
}

/// A sender for a queue.
pub struct Sender<T: Sized + Send + Copy> {
  handle: NonNull<<QueueHandle_t as PtrType>::Type>,
  item_type: PhantomData<T>,
}

impl<T: Sized + Send + Copy> Sender<T> {
  impl_send!();
}

/// A receiver for a queue.
pub struct Receiver<T> {
  handle: NonNull<<QueueHandle_t as PtrType>::Type>,
  item_type: PhantomData<T>,
}

impl<T: Sized + Send + Copy> Receiver<T> {
  impl_receive!();
}

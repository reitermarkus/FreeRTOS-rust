use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::{MaybeUninit, size_of, self};
use core::ops::Deref;
use core::pin::Pin;
use core::ptr;

use alloc2::sync::Arc;

use crate::FreeRtosError;
use crate::InterruptContext;
use crate::alloc::{Dynamic, Static};
use crate::lazy_init::{LazyPtr, LazyInit};
use crate::shim::*;
use crate::Ticks;

mod handle;
pub use handle::QueueHandle;

/// A fixed-size queue. Items are copied and owned by the queue.
pub struct Queue<T, const SIZE: usize, A = Dynamic>
where
  Self: LazyInit,
{
  alloc_type: PhantomData<A>,
  item_type: PhantomData<T>,
  handle: LazyPtr<Self>,
}

impl<T, const SIZE: usize> LazyInit for Queue<T, SIZE, Dynamic> {
  type Handle = QueueHandle_t;

  fn init(_storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
    let handle = unsafe {
      xQueueCreate(
        (mem::size_of::<T>() * SIZE) as UBaseType_t,
        size_of::<T>() as UBaseType_t,
      )
    };
    assert!(!handle.is_null());
    unsafe { Self::Ptr::new_unchecked(handle) }
  }

  fn destroy(ptr: Self::Ptr, _storage: &mut MaybeUninit<Self::Storage>) {
    unsafe { vQueueDelete(ptr.as_ptr()) }
  }
}

impl<T, const SIZE: usize> LazyInit for Queue<T, SIZE, Static> {
  type Handle = QueueHandle_t;
  type Storage = (MaybeUninit<StaticQueue_t>, [MaybeUninit<T>; SIZE]);

  fn init(storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> Self::Ptr {
    let handle = unsafe {
      // SAFETY: Data only consists of `MaybeUninit`.
      let storage = &mut *storage.get();
      let (queue, items) = storage.assume_init_mut();

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

  fn cancel_init_supported() -> bool {
    false
  }

  fn destroy(ptr: Self::Ptr, storage: &mut MaybeUninit<Self::Storage>) {
    unsafe {
      vQueueDelete(ptr.as_ptr());
      ptr::drop_in_place(storage.as_mut_ptr());
    }
  }
}

unsafe impl<T: Send, const SIZE: usize, A> Send for Queue<T, SIZE, A>
where
  Self: LazyInit,
{}
unsafe impl<T: Send, const SIZE: usize, A> Sync for Queue<T, SIZE, A>
where
  Self: LazyInit,
{}

impl<T: Sized + Send + Copy, const SIZE: usize> Queue<T, SIZE, Dynamic>
where
  Self: LazyInit<Data = ()>,
{
    /// Create a new dynamic `Queue`.
    pub const fn new() -> Self {
      Self {
        alloc_type: PhantomData,
        item_type: PhantomData,
        handle: LazyPtr::new(())
      }
    }
}

impl<T: Sized + Send + Copy, const SIZE: usize> Queue<T, SIZE, Static>
where
  Self: LazyInit<Data = ()>,
{
    /// Create a new static `Queue`.
    ///
    /// # Safety
    ///
    /// The returned queue must be pinned before using it.
    ///
    /// # Examples
    ///
    /// ```
    /// use freertos_rust::pin_static;
    ///
    /// pin_static!(pub static QUEUE = Queue::<8>::new_static());
    /// ```
    pub const unsafe fn new_static() -> Self {
      Self { alloc_type: PhantomData, item_type: PhantomData, handle: LazyPtr::new(()) }
    }
}

impl<T: Sized + Send + Copy, const SIZE: usize, A> Deref for Queue<T, SIZE, A>
where
  Self: LazyInit<Handle = QueueHandle_t>,
{
  type Target = QueueHandle<T>;

  fn deref(&self) -> &Self::Target {
    unsafe { QueueHandle::<T>::from_ptr(self.handle.as_ptr()) }
  }
}

impl<T: Sized + Send + Copy, const SIZE: usize> Queue<T, SIZE, Dynamic>
where
  Self: LazyInit<Handle = QueueHandle_t>,
{
    /// Create a sender/receiver pair from this queue.
    pub fn split(self: Arc<Self>) -> (Sender<Arc<Self>>, Receiver<Arc<Self>>) {
      (Sender { queue: Arc::clone(&self) }, Receiver { queue: self })
    }
}

impl<T: Sized + Send + Copy, const SIZE: usize> Queue<T, SIZE, Static>
where
  Self: LazyInit<Handle = QueueHandle_t>,
{
    /// Create a sender/receiver pair from this queue.
    pub fn split(self: Pin<&Self>) -> (Sender<&Self>, Receiver<&Self>) {
      let queue = self.get_ref();
      (Sender { queue }, Receiver { queue })
    }
}

/// A sender for a queue.
pub struct Sender<Q: Deref> {
  queue: Q,
}

impl<Q, T> Sender<Q>
where
  Q: Deref<Target = QueueHandle<T>>,
{

  /// Send an item to the end of the queue. Wait for the queue to have empty space for it.
  #[inline]
  pub fn send(&self, item: T, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
    self.queue.send(item, timeout)
  }

  /// Send an item to the end of the queue, from an interrupt.
  #[inline]
  pub fn send_from_isr(&self, ic: &mut InterruptContext, item: T) -> Result<(), FreeRtosError> {
    self.queue.send_from_isr(ic, item)
  }
}

/// A receiver for a queue.
pub struct Receiver<Q: Deref> {
  queue: Q,
}

impl<Q, T> Receiver<Q>
where
  Q: Deref<Target = QueueHandle<T>>,
{
  /// Wait for an item to be available on the queue.
  #[inline]
  pub fn receive(&self, timeout: impl Into<Ticks>) -> Result<T, FreeRtosError> {
    self.queue.receive(timeout)
  }
}

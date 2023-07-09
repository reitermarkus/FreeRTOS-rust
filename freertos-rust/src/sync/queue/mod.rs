use core::{
  marker::PhantomData,
  mem::{MaybeUninit, size_of, self},
  ops::Deref,
  ptr,
};

use alloc2::sync::Arc;

use crate::{
  FreeRtosError,
  InterruptContext,
  Ticks,
  ffi::{QueueHandle_t, UBaseType_t},
  shim::{vQueueDelete, StaticQueue_t, xQueueCreate, xQueueCreateStatic},
};

mod handle;
pub use handle::QueueHandle;

/// A fixed-size queue. Items are copied and owned by the queue.
///
/// # Example
///
/// ```
/// use freertos_rust::sync::Queue;
///
/// let queue = Queue::<u32, 8>::new();
/// queue.send(42);
///
/// let (_sender, receiver) = queue.split();
/// assert_eq!(receiver.receive(), Ok(42));
pub struct Queue<T, const SIZE: usize> {
  handle: QueueHandle_t,
  item_type: PhantomData<T>,
}

unsafe impl<T: Send, const SIZE: usize> Send for Queue<T, SIZE> {}
unsafe impl<T: Send, const SIZE: usize> Sync for Queue<T, SIZE> {}

impl<T, const SIZE: usize> Queue<T, SIZE> {
    /// Create a new dynamic queue.
    pub fn new() -> Self {
      let ptr = unsafe {
        xQueueCreate(
          (mem::size_of::<T>() * SIZE) as UBaseType_t,
          size_of::<T>() as UBaseType_t,
        )
      };
      assert!(!ptr.is_null());

      Self {
        handle: ptr,
        item_type: PhantomData,
      }
    }
}

impl<T, const SIZE: usize> Queue<T, SIZE> {
    /// Create a sender/receiver pair from this queue.
    pub fn split(self: Arc<Self>) -> (Sender<Arc<Self>>, Receiver<Arc<Self>>) {
      (Sender { queue: Arc::clone(&self) }, Receiver { queue: self })
    }
}

impl<T, const SIZE: usize> Deref for Queue<T, SIZE> {
  type Target = QueueHandle<T>;

  fn deref(&self) -> &Self::Target {
    unsafe { QueueHandle::<T>::from_ptr(self.handle) }
  }
}

impl<T, const SIZE: usize> Drop for Queue<T, SIZE> {
  fn drop(&mut self) {
    unsafe { vQueueDelete(self.handle) }
  }
}

/// A sender for a queue.
pub struct Sender<Q: Deref> {
  queue: Q,
}

impl<Q, T> Sender<Q>
where
  T: Sized + Send,
  Q: Deref<Target = QueueHandle<T>>,
{

  /// Send an item to the end of the queue. Wait for the queue to have empty space for it.
  #[inline]
  pub fn send(&self, item: T, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
    self.queue.send(item, timeout)
  }

  /// Send an item to the end of the queue, from an interrupt.
  #[inline]
  pub fn send_from_isr(&self, ic: &InterruptContext, item: T) -> Result<(), FreeRtosError> {
    self.queue.send_from_isr(ic, item)
  }
}

/// A receiver for a queue.
pub struct Receiver<Q: Deref> {
  queue: Q,
}

impl<Q, T> Receiver<Q>
where
  T: Sized + Send,
  Q: Deref<Target = QueueHandle<T>>,
{
  /// Wait for an item to be available on the queue.
  #[inline]
  pub fn receive(&self, timeout: impl Into<Ticks>) -> Result<T, FreeRtosError> {
    self.queue.receive(timeout)
  }
}

/// A statically allocated fixed-size queue. Items are copied and owned by the queue.
///
/// # Examples
///
/// ```
/// use core::mem::MaybeUninit;
///
/// use freertos_rust::sync::StaticQueue;
///
/// let queue = StaticQueue::new(unsafe {
///   static mut QUEUE: MaybeUninit<StaticQueue<u32, 8>> = MaybeUninit::uninit();
///   &mut QUEUE
/// });
/// queue.send(42);
///
/// let (_sender, receiver) = queue.split();
/// assert_eq!(receiver.receive(), Ok(42));
/// ```
pub struct StaticQueue<T, const SIZE: usize> {
  data: StaticQueue_t,
  items: [MaybeUninit<T>; SIZE],
}

unsafe impl<T: Send, const SIZE: usize> Send for StaticQueue<T, SIZE> {}
unsafe impl<T: Send, const SIZE: usize> Sync for StaticQueue<T, SIZE> {}

impl<T, const SIZE: usize> StaticQueue<T, SIZE> {
  /// Create a new static queue.
  pub fn new(queue: &'static mut MaybeUninit<Self>) -> &'static Self {
    let queue_ptr = queue.as_mut_ptr();

    unsafe {
      let ptr = xQueueCreateStatic(
        SIZE as UBaseType_t,
        size_of::<T>() as UBaseType_t,
        ptr::addr_of_mut!((*queue_ptr).items).cast(),
        ptr::addr_of_mut!((*queue_ptr).data),
      );
      debug_assert!(!ptr.is_null());
      debug_assert_eq!(ptr, ptr::addr_of!((*queue_ptr).data) as QueueHandle_t);
      queue.assume_init_ref()
    }
  }
}

impl<T, const SIZE: usize> StaticQueue<T, SIZE> {
  /// Create a sender/receiver pair from this queue.
  pub fn split(&'static self) -> (Sender<&'static Self>, Receiver<&'static Self>) {
    (Sender { queue: self }, Receiver { queue: self })
  }
}

impl<T, const SIZE: usize> Deref for StaticQueue<T, SIZE> {
  type Target = QueueHandle<T>;

  fn deref(&self) -> &Self::Target {
    unsafe { QueueHandle::<T>::from_ptr(ptr::addr_of!(self.data) as QueueHandle_t) }
  }
}

impl<T, const SIZE: usize> Drop for StaticQueue<T, SIZE> {
  fn drop(&mut self) {
    unsafe { vQueueDelete(self.as_ptr()) }
  }
}

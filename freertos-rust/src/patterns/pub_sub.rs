use alloc2::{
  sync::Arc,
  vec::Vec,
};

use crate::error::*;
use crate::sync::{Mutex, Queue};
use crate::ticks::Ticks;

/// A pub-sub queue. An item sent to the publisher is sent to every subscriber.
pub struct QueuePublisher<T: Sized + Copy, const SUB_SIZE: usize, const PUB_SIZE: usize> {
    inner: Arc<Mutex<PublisherInner<T, SUB_SIZE, PUB_SIZE>>>,
}

/// A subscribtion to the publisher.
pub struct QueueSubscriber<T: Sized + Copy, const SUB_SIZE: usize, const PUB_SIZE: usize> {
    inner: Arc<SubscriberInner<T, SUB_SIZE, PUB_SIZE>>,
}

impl<T: Sized + Send + Copy, const SUB_SIZE: usize, const PUB_SIZE: usize> QueuePublisher<T, SUB_SIZE, PUB_SIZE> {
    /// Create a new publisher
    pub fn new() -> Result<Self, FreeRtosError> {
        let inner = PublisherInner {
            subscribers: Vec::new(),
            queue_next_id: 1,
        };

        Ok(QueuePublisher {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    /// Send an item to every subscriber. Returns the number of
    /// subscribers that have received the item.
    pub fn send(&self, item: T, timeout: impl Into<Ticks>) -> usize {
      let timeout = timeout.into();

      let mut sent_to = 0;
      if let Ok(m) = self.inner.timed_lock(timeout) {
        for subscriber in &m.subscribers {
          if let Ok(_) = subscriber.queue.send(item, timeout) {
            sent_to += 1;
          }
        }
      }
      sent_to
    }

    /// Subscribe to this publisher. Can accept a fixed amount of items.
    pub fn subscribe(&self, timeout: impl Into<Ticks>) -> Result<QueueSubscriber<T, SUB_SIZE, PUB_SIZE>, FreeRtosError> {
        let mut inner = self.inner.timed_lock(timeout)?;

        let queue = Queue::new();

        let id = inner.queue_next_id;
        inner.queue_next_id += 1;

        let subscriber = SubscriberInner {
            id: id,
            queue: queue,
            publisher: self.inner.clone(),
        };
        let subscriber = Arc::new(subscriber);

        inner.subscribers.push(subscriber.clone());

        Ok(QueueSubscriber { inner: subscriber })
    }
}

impl<T: Sized + Copy, const SUB_SIZE: usize, const PUB_SIZE: usize> Clone for QueuePublisher<T, SUB_SIZE, PUB_SIZE> {
    fn clone(&self) -> Self {
        QueuePublisher {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Sized + Copy, const SUB_SIZE: usize, const PUB_SIZE: usize> Drop for QueueSubscriber<T, SUB_SIZE, PUB_SIZE> {
    fn drop(&mut self) {
        if let Ok(mut l) = self.inner.publisher.lock() {
            l.unsubscribe(&self.inner);
        }
    }
}

impl<T: Sized + Send + Copy, const SUB_SIZE: usize, const PUB_SIZE: usize> QueueSubscriber<T, SUB_SIZE, PUB_SIZE> {
    /// Wait for an item to be posted from the publisher.
    pub fn receive(&self, timeout: impl Into<Ticks>) -> Result<T, FreeRtosError> {
        self.inner.queue.receive(timeout)
    }
}

struct PublisherInner<T: Sized + Copy, const SUB_SIZE: usize, const PUB_SIZE: usize> {
    subscribers: Vec<Arc<SubscriberInner<T, SUB_SIZE, PUB_SIZE>>>,
    queue_next_id: usize,
}

impl<T: Sized + Copy, const SUB_SIZE: usize, const PUB_SIZE: usize> PublisherInner<T, SUB_SIZE, PUB_SIZE> {
    fn unsubscribe(&mut self, subscriber: &SubscriberInner<T, SUB_SIZE, PUB_SIZE>) {
        self.subscribers.retain(|ref x| x.id != subscriber.id);
    }
}

struct SubscriberInner<T: Sized + Copy, const SUB_SIZE: usize, const PUB_SIZE: usize> {
    id: usize,
    queue: Queue<T, SUB_SIZE>,
    publisher: Arc<Mutex<PublisherInner<T, SUB_SIZE, PUB_SIZE>>>,
}

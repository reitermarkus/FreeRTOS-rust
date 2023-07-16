//! FreeRTOS synchronization primitives.
//!
//! # Examples
//!
//! ## Queue
//!
//! ```
//! use core::time::Duration;
//!
//! use freertos_rust::sync::Queue;
//!
//! let queue = Queue::<u32, 4>::new();
//!
//! assert!(queue.receive(Duration::ZERO).is_err());
//!
//! queue.send(10, Duration::from_millis(5)).unwrap();
//! assert_eq!(queue.receive(Duration::MAX).unwrap(), 10);
//! ```
//!
//! ## Mutex
//!
//! ```
//! use core::mem::MaybeUninit;
//!
//! use freertos_rust::sync::Mutex;
//!
//! let mutex = Mutex::<u32>::new(16);
//!
//! {
//!   let mut v = mutex.lock().unwrap();
//!   *v += 1;
//! }
//!
//! assert_eq!(*mutex.lock().unwrap(), 17);
//! ```
//!
//! ## Binary Semaphore
//!
//! ```
//! use core::time::Duration;
//!
//! use freertos_rust::sync::Semaphore;
//!
//! let semaphore = Semaphore::new_binary();
//!
//! // semaphore.give().unwrap();
//!
//! // ...
//! println!("Taking semaphore.");
//! semaphore.take(Duration::MAX).unwrap();
//! println!("Took semaphore.");
//! drop(semaphore);
//! println!("Dropped semaphore.");
//! ```
//!
//! ## Counting Semaphore
//!
//! ```
//! use core::time::Duration;
//!
//! use freertos_rust::sync::{Counting, Semaphore};
//!
//! let semaphore = Semaphore::<Counting<4, 4>>::new_counting();
//!
//! let _guard = semaphore.lock(Duration::MAX).unwrap();
//!
//! // ...
//!
//! ```


mod mutex;
pub use mutex::*;
mod queue;
pub use queue::*;
mod semaphore;
pub use semaphore::*;

//! FreeRTOS synchronization primitives.
//!
//! # Examples
//!
//! ## Queue
//!
//! ```
//! use core::time::Duration;
//! use freertos_rust::sync::Queue;
//!
//! static Q: Queue<u32, 4> = Queue::new();
//!
//! assert!(Q.receive(Duration::ZERO).is_err());
//!
//! Q.send(10, Duration::from_millis(5)).unwrap();
//! assert_eq!(Q.receive(Duration::MAX).unwrap(), 10);
//! ```
//!
//! ## Mutex
//!
//! ```
//! use freertos_rust::sync::Mutex;
//!
//! static M: Mutex<u32> = Mutex::new(16);
//!
//! {
//! 	let mut v = M.lock().unwrap();
//! 	*v += 1;
//! }
//!
//! assert_eq!(*M.lock().unwrap(), 17)
//! ```
//!
//! ## Binary Semaphore
//!
//! ```
//! use core::time::Duration;
//! use freertos_rust::sync::{Binary, Semaphore};
//!
//! static S: Semaphore<Binary> = Semaphore::new_binary();
//!
//! S.take(Duration::MAX).unwrap();
//!
//! // ...
//!
//! S.give().unwrap();
//! ```
//!
//! ## Counting Semaphore
//!
//! ```
//! use core::time::Duration;
//! use freertos_rust::sync::{Counting, Semaphore};
//!
//! static S: Semaphore<Counting<4, 4>> = Semaphore::new_counting();
//!
//! let _guard = S.lock(Duration::MAX).unwrap();
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

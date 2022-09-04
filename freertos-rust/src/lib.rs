//! # FreeRTOS for Rust
//!
//! Rust interface for the FreeRTOS embedded operating system. Requires nightly Rust.
//! It is assumed that dynamic memory allocation is provided on the target system.
//!
//! This library interfaces with FreeRTOS using a C shim library which provides function
//! wrappers for FreeRTOS macros. The compiled Rust application should be linked to the
//! base C/C++ firmware binary.
//!
//! Examples are provided inside [freertos-rust-examples](https://github.com/lobaro/FreeRTOS-rust/tree/master/freertos-rust-examples)
//!
//! For more examples, check the enclosed GCC ARM/Rust/QEMU based unit tests. The project
//! ``qemu_runner`` cross-compiles this library, compiles the main firmware using GCC ARM and links
//! in the appropriate entry points for unit tests. [GNU ARM Eclipse QEMU](http://gnuarmeclipse.github.io/qemu/)
//! is used to run the test binaries.
//!
//! Be sure to check the [FreeRTOS documentation](http://www.freertos.org/RTOS.html).
//!
//! # Samples
//!
//! Spawning a new task
//!
//! ```rust
//! # use freertos_rs::*;
//! Task::new().name("hello").stack_size(128).start(|task| {
//! 	loop {
//! 		println!("Hello world!");
//! 		task.delay(Duration::MAX);
//! 	}
//! }).unwrap();
//!
//! Task::start_scheduler();
//! ```
//!
//! Queue
//!
//! ```rust
//! # use freertos_rs::*;
//! let q = Queue::new(10).unwrap();
//! q.send(10, Duration::from_millis(5)).unwrap();
//! q.receive(Duration::MAX).unwrap();
//! ```
//!
//! Mutex
//!
//! ```rust
//! # use freertos_rs::*;
//! let m = Mutex::new(0).unwrap();
//! {
//! 	let mut v = m.lock(Duration::MAX).unwrap();
//! 	*v += 1;
//! }
//! ```
#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![feature(maybe_uninit_slice, maybe_uninit_uninit_array, const_maybe_uninit_uninit_array, maybe_uninit_write_slice)]
#![feature(c_size_t)]
#![feature(const_option)]
#![feature(const_trait_impl)]
#![feature(const_pin)]
#![feature(associated_type_defaults)]
#![feature(const_mut_refs)]
#![feature(const_convert)]
#![feature(const_result_drop)]

#[cfg_attr(any(feature = "time", feature = "sync"), macro_use)]
extern crate alloc as alloc2;

pub mod assert;
mod error;
mod shim;
pub mod ffi;

pub mod alloc;
pub use alloc::Allocator;

#[cfg(feature = "critical_section")]
pub mod critical_section;

#[cfg(feature = "interrupt")]
mod isr;

mod lazy_init;

#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "sync")]
pub use sync::*;

pub mod task;

#[cfg(feature = "time")]
pub mod timer;
pub use crate::timer::*;


#[cfg(any(feature = "time", feature = "sync"))]
mod ticks;
mod utils;

#[cfg(feature = "sync")]
pub mod patterns;

pub use crate::error::*;

pub use crate::error::FreeRtosError;
pub use crate::assert::*;

#[cfg(feature = "interrupt")]
pub use crate::isr::*;

#[cfg(any(feature = "time", feature = "sync"))]
pub use crate::task::*;
#[cfg(feature = "time")]
#[cfg(any(feature = "time", feature = "sync"))]
pub use crate::ticks::*;

pub use crate::utils::cpu_clock_hz;

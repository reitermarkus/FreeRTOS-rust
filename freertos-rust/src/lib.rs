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
#![feature(negative_impls)]
#![warn(missing_docs)]

#[cfg_attr(any(feature = "time", feature = "sync"), macro_use)]
extern crate alloc as alloc2;

mod assertion_handler;
pub use assertion_handler::set_assertion_handler;

mod error;
mod shim;
pub mod ffi;

pub mod alloc;
pub use alloc::Allocator;

#[cfg(feature = "critical_section")]
pub mod critical_section;

mod interrupt_context;

mod lazy_init;

#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "sync")]
pub use sync::*;

pub mod task;

#[cfg(feature = "time")]
pub mod timer;
pub use crate::timer::*;

mod hooks;
pub use hooks::{set_tick_hook, set_idle_hook};


#[cfg(any(feature = "time", feature = "sync"))]
mod ticks;
mod utils;

pub use crate::error::*;

pub use crate::error::FreeRtosError;

pub use crate::interrupt_context::*;

#[cfg(any(feature = "time", feature = "sync"))]
pub use crate::task::*;
#[cfg(feature = "time")]
#[cfg(any(feature = "time", feature = "sync"))]
pub use crate::ticks::*;

pub use utils::cpu_clock_hz;

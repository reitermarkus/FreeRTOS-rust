//! Low-level FreeRTOS bindings.

/// Signed base integer type.
pub type BaseType_t = crate::shim::BaseType_t;

/// Unsigned base integer type.
pub type UBaseType_t = crate::shim::UBaseType_t;

/// Raw semaphore handle.
pub type SemaphoreHandle_t = crate::shim::SemaphoreHandle_t;

/// Raw queue handle.
pub type QueueHandle_t = crate::shim::QueueHandle_t;

/// Raw tick type.
pub type TickType_t = crate::shim::TickType_t;

/// Raw timer handle.
pub type TimerHandle_t = crate::shim::TimerHandle_t;

/// Raw task handle.
pub type TaskHandle_t = crate::shim::TaskHandle_t;

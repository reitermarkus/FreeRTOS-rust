/// Basic error type for the library.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FreeRtosError {
    OutOfMemory,
    Timeout,
    QueueFull,
    StringConversionError,
    TaskNotFound,
    InvalidQueueSize,
    ProcessorHasShutDown,
}

pub type CVoid = core::ffi::c_void;

pub type FreeRtosVoidPtr = *mut CVoid;

pub type FreeRtosBaseType = i32;
pub type FreeRtosUBaseType = u32;
pub type FreeRtosTickType = u32;

pub type FreeRtosTaskHandle = *mut CVoid;
pub type FreeRtosQueueHandle = *mut CVoid;
pub type FreeRtosSemaphoreHandle = *mut CVoid;
pub type FreeRtosTaskFunction = *const CVoid;
pub type FreeRtosTimerHandle = *mut CVoid;
pub type FreeRtosTimerCallback = *const CVoid;

pub type FreeRtosUnsignedLong = u32;
pub type FreeRtosUnsignedShort = u16;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum FreeRtosTaskState {
    /// A task is querying the state of itself, so must be running.
    Running = 0,
    /// The task being queried is in a read or pending ready list.
    Ready = 1,
    /// The task being queried is in the Blocked state.
    Blocked = 2,
    /// The task being queried is in the Suspended state, or is in the Blocked state with an infinite time out.
    Suspended = 3,
    /// The task being queried has been deleted, but its TCB has not yet been freed.
    Deleted = 4,
    /// Task state is invalid.
    Invalid = 5,
}

impl From<u32> for FreeRtosTaskState {
    fn from(s: u32) -> Self {
        match s {
            0 => Self::Running,
            1 => Self::Ready,
            2 => Self::Blocked,
            3 => Self::Suspended,
            4 => Self::Deleted,
            _ => Self::Invalid,
        }
    }
}

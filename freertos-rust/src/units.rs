use crate::base::FreeRtosTickType;
use crate::shim::*;

pub trait DurationTicks: Copy + Clone {
    /// Convert to ticks, the internal time measurement unit of FreeRTOS
    fn to_ticks(&self) -> FreeRtosTickType;
}

/// Time unit used by FreeRTOS, passed to the scheduler as ticks.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Duration {
    ticks: u32,
}

impl Duration {
    /// Milliseconds constructor
    pub fn ms(milliseconds: u32) -> Self {
        Self::ticks(milliseconds / portTICK_PERIOD_MS)
    }

    pub fn ticks(ticks: u32) -> Self {
        Self { ticks }
    }

    /// An infinite duration
    pub fn infinite() -> Self {
        Self::ticks(portMAX_DELAY)
    }

    /// A duration of zero, for non-blocking calls
    pub fn zero() -> Self {
        Self::ticks(0)
    }

    /// Smallest unit of measurement, one tick
    pub fn eps() -> Self {
        Self::ticks(1)
    }

    pub fn to_ms(&self) -> u32 {
        self.ticks * portTICK_PERIOD_MS
    }
}

impl DurationTicks for Duration {
    fn to_ticks(&self) -> FreeRtosTickType {
        self.ticks
    }
}

use crate::shim::*;

/// Get the CPU frequency in Hertz.
#[inline(always)]
pub fn cpu_clock_hz() -> u32 {
  unsafe { freertos_rs_get_configCPU_CLOCK_HZ() }
}

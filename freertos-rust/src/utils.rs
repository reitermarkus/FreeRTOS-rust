use crate::shim::freertos_rs_get_configCPU_CLOCK_HZ;

/// Get the CPU frequency in Hertz.
#[inline(always)]
pub fn cpu_clock_hz() -> usize {
  unsafe { freertos_rs_get_configCPU_CLOCK_HZ() as _ }
}

use crate::shim::*;

use critical_section::RawRestoreState;

/// Critical section implemenation based on FreeRTOS.
///
/// For more information, visit the [`critical_section`] documentation.
#[non_exhaustive]
pub struct CriticalSection {}

critical_section::set_impl!(CriticalSection);

impl critical_section::Impl for CriticalSection {
  #[inline(always)]
  unsafe fn acquire() -> RawRestoreState {
    unsafe { freertos_rs_enter_critical() }
  }

  #[inline(always)]
  unsafe fn release(_token: RawRestoreState) {
    unsafe { freertos_rs_exit_critical() }
  }
}

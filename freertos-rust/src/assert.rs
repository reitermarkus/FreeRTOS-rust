use core::{
  ffi::{c_char, c_size_t},
  mem,
  slice,
  str,
  sync::atomic::{AtomicPtr, Ordering},
};

type AssertFunction = fn(&'static str, &'static str, usize);

static ASSERT_FUNCTION: AtomicPtr<AssertFunction> = AtomicPtr::new(assert_panic as *mut _);

/// Set a custom assertion handler.
///
/// # Examples
///
/// ```
/// fn my_assertion_handler(message: &str, file_name: &str, line: usize) {
///   panic!("FreeRTOS assertion in file {} at line {} failed: {}", file_name, line, message);
/// }
///
/// freertos_rust::assert::set_handler(my_assertion_handler);
/// ```
pub fn set_handler(f: fn(&'static str, &'static str, usize)) {
  ASSERT_FUNCTION.store(f as *mut _, Ordering::Release);
}

fn assert_panic(message: &'static str, file_name: &'static str, line: usize) {
  let file_name = file_name.rsplit_once('/').map(|(_, s)| s).unwrap_or(file_name);
  panic!("assertion at {}:{} failed: {}", file_name, line, message);
}

#[export_name = "__rust__vAssertCalled"]
extern "C" fn assert_called(
  message: *const c_char, message_len: c_size_t,
  file_name: *const c_char, file_name_len: c_size_t,
  line: c_size_t,
) {
    let message = unsafe {
      str::from_utf8_unchecked(slice::from_raw_parts(message.cast(), message_len))
    };

    let file_name = unsafe {
      str::from_utf8_unchecked(slice::from_raw_parts(file_name.cast(), file_name_len))
    };

    let f: AssertFunction = unsafe { mem::transmute(ASSERT_FUNCTION.load(Ordering::Acquire)) };
    f(message, file_name, line);
}

use core::{
  ffi::{c_char, c_size_t},
  mem,
  slice,
  str,
  sync::atomic::{AtomicPtr, Ordering},
};

static ASSERT_FUNCTION: AtomicPtr<()> = AtomicPtr::new(assert_panic as *mut _);

/// Set a custom assertion handler.
///
/// The handler receives the message (i.e. the literal boolean expression as it appears in C code),
/// the file name and the line number of the failed assertion.
///
/// # Examples
///
/// ```
/// fn my_assertion_handler(message: &str, file_name: &str, line: usize) {
///   panic!("FreeRTOS assertion in file {} at line {} failed: {}", file_name, line, message);
/// }
///
/// freertos_rust::set_assertion_handler(my_assertion_handler);
/// ```
pub fn set_assertion_handler(f: fn(message: &'static str, file_name: &'static str, line: usize)) {
  ASSERT_FUNCTION.store(f as *mut _, Ordering::Release);
}

fn assert_panic(message: &'static str, file_name: &'static str, line: usize) {
  let file_name = file_name.rsplit_once('/').map(|(_, s)| s).unwrap_or(file_name);
  panic!("assertion at {}:{} failed: {}", file_name, line, message);
}

#[export_name = "vAssertCalled"]
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

    let f: fn(&'static str, &'static str, usize) = unsafe { mem::transmute(ASSERT_FUNCTION.load(Ordering::Acquire)) };
    f(message, file_name, line);
}

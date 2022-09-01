use core::{
  ffi::{CStr, c_char},
  mem::MaybeUninit,
  str,
};

/// Helper struct for passing a `&str` to `xTaskCreate`.
#[derive(Debug)]
pub(crate) struct TaskName<const CAPACITY: usize> {
  buf: [MaybeUninit<u8>; CAPACITY],
}

impl<const CAPACITY: usize> TaskName<CAPACITY> {
  pub fn new(name: &str) -> Self {
    let mut buf = MaybeUninit::uninit_array();

    let mut capacity = CAPACITY;
    for c in name.chars() {
      if capacity <= c.len_utf8() {
        break
      }

      let mut encoded = [0; 4];
      let bytes = c.encode_utf8(&mut encoded).as_bytes();
      let len = CAPACITY - capacity;
      MaybeUninit::write_slice(&mut buf[len..(len + bytes.len())], bytes);
      capacity -= bytes.len();
    }
    buf[CAPACITY - capacity].write(0);

    // Store remaining capacity in last byte. No remaining capacity
    // automatically means the last byte is a terminating NULL byte.
    buf[CAPACITY - 1].write(capacity as _);

    Self { buf }
  }

  pub unsafe fn from_ptr(ptr: *const c_char) -> Self {
    let name = CStr::from_ptr(ptr).to_str().unwrap();
    Self::new(name)
  }

  pub fn as_ptr(&self) -> *const c_char {
    MaybeUninit::slice_as_ptr(&self.buf).cast()
  }

  pub fn as_str(&self) -> &str {
    // SAFETY: An existing `TaskName` can only contain a valid UTF-8 string
    //         with the last byte storing the remaining capacity.
    unsafe {
      let len = CAPACITY - self.buf[CAPACITY - 1].assume_init() as usize;
      str::from_utf8_unchecked(MaybeUninit::slice_assume_init_ref(&self.buf[..len]))
    }
  }
}

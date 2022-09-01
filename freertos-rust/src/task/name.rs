use core::ffi::c_char;
use core::mem::MaybeUninit;

/// Helper struct for passing a `&str` to `xTaskCreate`.
pub struct TaskName<const CAPACITY: usize> {
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

    Self { buf }
  }

  pub fn as_ptr(&self) -> *const c_char {
    MaybeUninit::slice_as_ptr(&self.buf).cast()
  }
}

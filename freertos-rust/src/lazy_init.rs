use core::ffi::c_void;
use core::marker::PhantomData;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, Ordering::*};

pub trait LazyInit {
  fn init() -> NonNull<c_void>;

  fn cancel_init(ptr: NonNull<c_void>) {
    Self::destroy(ptr);
  }

  fn destroy(ptr: NonNull<c_void>);
}

pub struct LazyPtr<T: LazyInit> {
  ptr: AtomicPtr<c_void>,
  _type: PhantomData<T>,
}

impl<T: LazyInit> LazyPtr<T> {
  #[inline]
  pub const fn new() -> Self {
    Self { ptr: AtomicPtr::new(ptr::null_mut()), _type: PhantomData }
  }

  #[inline]
  pub const unsafe fn new_unchecked(ptr: *mut c_void) -> Self {
    Self { ptr: AtomicPtr::new(ptr), _type: PhantomData }
  }

  #[inline]
  pub fn as_ptr(&self) -> *mut c_void {
    let ptr = self.ptr.load(Acquire);
    if ptr.is_null() {
      self.initialize()
    } else {
      ptr
    }
  }

  #[cold]
  fn initialize(&self) -> *mut c_void {
    let new_ptr = T::init();
    match self.ptr.compare_exchange(ptr::null_mut(), new_ptr.as_ptr(), AcqRel, Acquire) {
      Ok(_) => new_ptr.as_ptr(),
      Err(ptr) => {
        T::cancel_init(new_ptr);
        ptr
      }
    }
  }
}

impl<T: LazyInit> Drop for LazyPtr<T> {
  fn drop(&mut self) {
    if let Some(ptr) = NonNull::new(*self.ptr.get_mut()) {
      T::destroy(ptr)
    }
  }
}

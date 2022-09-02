use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::marker::PhantomData;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, Ordering::*};

use crate::shim::{freertos_rs_enter_critical, freertos_rs_exit_critical};

pub trait PtrType {
  type Type;
}

impl<T> PtrType for *mut T {
  type Type = T;
}

pub trait LazyInit<P: PtrType = *mut c_void> {
  type Ptr = NonNull<P::Type>;
  type Data = ();

  fn init(data: &UnsafeCell<MaybeUninit<Self::Data>>) -> NonNull<P::Type>;

  fn cancel_init_supported() -> bool {
    true
  }

  fn cancel_init(ptr: NonNull<P::Type>) {
    Self::destroy(ptr);
  }

  fn destroy(ptr: NonNull<P::Type>);
}

pub struct LazyPtr<T: ?Sized, P>
where
  T: LazyInit<P>,
  P: PtrType,
{
  ptr: AtomicPtr<P::Type>,
  data: UnsafeCell<MaybeUninit<<T as LazyInit<P>>::Data>>,
  _type: PhantomData<T>,
}

impl<T: ?Sized, P> LazyPtr<T, P>
where
  T: LazyInit<P>,
  P: PtrType,
{
  #[inline]
  pub const fn new() -> Self {
    unsafe { Self::new_unchecked(ptr::null_mut()) }
  }

  #[inline]
  pub const unsafe fn new_unchecked(ptr: *mut P::Type) -> Self {
    Self { ptr: AtomicPtr::new(ptr), data: UnsafeCell::new(MaybeUninit::uninit()), _type: PhantomData }
  }

  #[inline]
  pub fn as_ptr(&self) -> *mut P::Type {
    let ptr = self.ptr.load(Acquire);
    if ptr.is_null() {
      self.initialize()
    } else {
      ptr
    }
  }

  #[cold]
  fn initialize(&self) -> *mut P::Type {
    // If initialization cannot be cancelled, do it inside of a critical section
    // so that initialization and storing the pointer is done atomically.
    if !T::cancel_init_supported() {
      unsafe { freertos_rs_enter_critical() };
      let mut ptr = self.ptr.load(Acquire);
      if ptr.is_null() {
        ptr = T::init(&self.data).as_ptr();
        self.ptr.store(ptr, Release);
      }

      unsafe { freertos_rs_exit_critical() };
      return ptr
    }

    let new_ptr = T::init(&self.data);
    match self.ptr.compare_exchange(ptr::null_mut(), new_ptr.as_ptr(), AcqRel, Acquire) {
      Ok(_) => new_ptr.as_ptr(),
      Err(ptr) => {
        T::cancel_init(new_ptr);
        ptr
      }
    }
  }
}

impl<T: ?Sized, P> Drop for LazyPtr<T, P>
where
  P: PtrType,
  T: LazyInit<P>,
{
  fn drop(&mut self) {
    if let Some(ptr) = NonNull::new(*self.ptr.get_mut()) {
      T::destroy(ptr)
    }
  }
}

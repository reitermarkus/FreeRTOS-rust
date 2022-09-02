use core::ffi::c_void;
use core::marker::PhantomData;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, Ordering::*};

pub trait PtrType {
  type Type;
}

impl<T> PtrType for *mut T {
  type Type = T;
}

pub trait LazyInit<P: PtrType = *mut c_void> {
  type Ptr = NonNull<P::Type>;

  fn init() -> NonNull<P::Type>;

  fn cancel_init(ptr: NonNull<P::Type>) {
    Self::destroy(ptr);
  }

  fn destroy(ptr: NonNull<P::Type>);
}

pub struct LazyPtr<T, P>
where
  P: PtrType,
  T: LazyInit<P>,
{
  ptr: AtomicPtr<P::Type>,
  _type: PhantomData<T>,
}

impl<T, P> LazyPtr<T, P>
where
  P: PtrType,
  T: LazyInit<P>,
{
  #[inline]
  pub const fn new() -> Self {
    Self { ptr: AtomicPtr::new(ptr::null_mut()), _type: PhantomData }
  }

  #[inline]
  pub const unsafe fn new_unchecked(ptr: *mut P::Type) -> Self {
    Self { ptr: AtomicPtr::new(ptr), _type: PhantomData }
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

impl<T, P> Drop for LazyPtr<T, P>
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

use core::cell::UnsafeCell;
use core::mem::{MaybeUninit, ManuallyDrop};
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, Ordering::*};

use crate::shim::{freertos_rs_enter_critical, freertos_rs_exit_critical};

pub trait PtrType {
  type Type;
  type NonNull = NonNull<Self::Type>;
}

impl<T> PtrType for *mut T {
  type Type = T;
}

pub trait LazyInit {
  type Handle: PtrType;
  type Storage = ();
  type Ptr = NonNull<<Self::Handle as PtrType>::Type>;

  fn init(storage: &UnsafeCell<MaybeUninit<Self::Storage>>) -> NonNull<<Self::Handle as PtrType>::Type>;

  fn cancel_init_supported() -> bool {
    true
  }

  fn cancel_init(ptr: NonNull<<Self::Handle as PtrType>::Type>, storage: &mut MaybeUninit<Self::Storage>) {
    Self::destroy(ptr, storage);
  }

  fn destroy(ptr: NonNull<<Self::Handle as PtrType>::Type>, storage: &mut MaybeUninit<Self::Storage>);
}

pub struct LazyPtr<T: ?Sized, D: ?Sized = ()>
where
  T: LazyInit,
{
  storage: UnsafeCell<MaybeUninit<<T as LazyInit>::Storage>>,
  ptr: AtomicPtr<<<T as LazyInit>::Handle as PtrType>::Type>,
  data: UnsafeCell<D>,
}

impl<T: ?Sized, D> LazyPtr<T, D>
where
  T: LazyInit,
{
  #[inline]
  pub const fn new(data: D) -> Self {
    unsafe { Self::new_unchecked(ptr::null_mut(), data) }
  }

  pub const fn new_with_storage(data: D, storage: <T as LazyInit>::Storage) -> Self {
    Self {
      storage: UnsafeCell::new(MaybeUninit::new(storage)),
      ptr: AtomicPtr::new(ptr::null_mut()),
      data: UnsafeCell::new(data),
    }
  }

  #[inline]
  pub const unsafe fn new_unchecked(ptr: *mut <<T as LazyInit>::Handle as PtrType>::Type, data: D) -> Self {
    Self {
      storage: UnsafeCell::new(MaybeUninit::uninit()),
      ptr: AtomicPtr::new(ptr),
      data: UnsafeCell::new(data),
    }
  }

  pub const fn ptr_ptr(&self) -> *const AtomicPtr<<<T as LazyInit>::Handle as PtrType>::Type> {
    ptr::addr_of!(self.ptr)
  }

  pub fn into_data(self) -> D {
    let mut this = ManuallyDrop::new(self);
    this.deinitialize();
    unsafe { ptr::read(this.data.get()) }
  }
}

impl<T: ?Sized, D: ?Sized> LazyPtr<T, D>
where
  T: LazyInit,
{
  #[inline]
  pub fn as_ptr(&self) -> *mut <<T as LazyInit>::Handle as PtrType>::Type {
    let ptr = self.ptr.load(Acquire);
    if ptr.is_null() {
      self.initialize()
    } else {
      ptr
    }
  }

  #[cold]
  fn initialize(&self) -> *mut <<T as LazyInit>::Handle as PtrType>::Type {
    // If initialization cannot be cancelled, do it inside of a critical section
    // so that initialization and storing the pointer is done atomically.
    if !T::cancel_init_supported() {
      unsafe { freertos_rs_enter_critical() };
      let mut ptr = self.ptr.load(Acquire);
      if ptr.is_null() {
        ptr = T::init(&self.storage).as_ptr();
        self.ptr.store(ptr, Release);
      }

      unsafe { freertos_rs_exit_critical() };
      return ptr
    }

    let new_ptr = T::init(&self.storage);
    match self.ptr.compare_exchange(ptr::null_mut(), new_ptr.as_ptr(), AcqRel, Acquire) {
      Ok(_) => new_ptr.as_ptr(),
      Err(ptr) => {
        unsafe { T::cancel_init(new_ptr, &mut *self.storage.get()) };
        ptr
      }
    }
  }

  #[cold]
  fn deinitialize(&mut self) {
    if let Some(ptr) = NonNull::new(*self.ptr.get_mut()) {
      T::destroy(ptr, self.storage.get_mut())
    }
  }
}

impl<T: ?Sized, D: ?Sized> Drop for LazyPtr<T, D>
where
  T: LazyInit,
{
  fn drop(&mut self) {
    self.deinitialize()
  }
}

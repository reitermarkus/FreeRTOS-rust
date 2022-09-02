use core::marker::PhantomPinned;

#[non_exhaustive]
pub struct Dynamic {}

#[non_exhaustive]
pub struct Static {
  _pinned: PhantomPinned,
}

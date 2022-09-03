use core::marker::PhantomPinned;

#[non_exhaustive]
pub struct Dynamic {}

#[non_exhaustive]
pub struct Static {
  _pinned: PhantomPinned,
}

#[macro_export]
macro_rules! pin_static {
  (
    $vis:vis static $NAME:ident = $Ty:ident :: < $($Ty2:ty),+ > :: $new_fn:ident () $(;)?
  ) => {
    $vis static $NAME: Pin<&'static $Ty<$($Ty2,)* $crate::alloc::Static>> = {
      static UNPINNNED: $Ty<$($Ty2,)* $crate::alloc::Static> = unsafe { $Ty::$new_fn() };
      Pin::static_ref(&UNPINNNED)
    };
  };
}

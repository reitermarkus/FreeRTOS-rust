use proc_macro2::{Ident, Span, TokenStream};
use quote::{TokenStreamExt, quote};
use syn::Token;

mod asm;
pub use asm::*;

mod ty;
pub use ty::*;

mod identifier;
pub use identifier::*;

mod expr;
pub use expr::*;

mod function_call;
pub use function_call::*;

mod function_decl;
pub use function_decl::*;

mod literal;
pub(crate) use literal::*;

mod statement;
pub(crate) use statement::*;

mod decl;
pub(crate) use decl::*;

mod stringify;
pub(crate) use stringify::*;

use super::*;


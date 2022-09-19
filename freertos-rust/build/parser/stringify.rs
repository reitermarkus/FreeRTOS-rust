use super::*;

/// Stringification of a macro argument.
///
/// ```c
/// #define STRINGIFY(x) #x
/// ```
#[derive(Debug, Clone)]
pub struct Stringify {
  id: Identifier,
}

impl Stringify {
  pub fn parse<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    map(
      preceded(terminated(token("#"), meta), identifier),
      |id| Self { id: Identifier::Literal(id.to_owned()) },
    )(tokens)
  }

  pub fn visit<'s, 't>(&mut self, ctx: &mut Context<'s, 't>) {
    ctx.export_as_macro = true;
  }

  pub fn to_tokens(&self, ctx: &mut Context, tokens: &mut TokenStream) {
    tokens.append_all(self.to_token_stream(ctx))
  }

  pub fn to_token_stream(&self, ctx: &mut Context) -> TokenStream {
    let id = self.id.to_token_stream(ctx);

    quote! {
      ::core::stringify!($#id)
    }
  }
}

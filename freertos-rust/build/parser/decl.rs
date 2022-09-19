use super::*;

#[derive(Debug)]
pub struct Decl<'t> {
  ty: Type,
  name: Identifier,
  rhs: Expr<'t>,
  is_static: bool,
}

impl<'t> Decl<'t> {
  pub fn parse<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, ((static_storage, ty), name, _, rhs)) = tuple((
      permutation((opt(token("static")), |tokens| Type::parse(ctx, tokens))),
      |tokens| Identifier::parse(ctx, tokens), token("="), |tokens| Expr::parse(ctx, tokens),
    ))(tokens)?;

    Ok((tokens, Self { ty, name, rhs, is_static: static_storage.is_some() }))
  }

  pub fn to_tokens(&self, ctx: &mut Context, tokens: &mut TokenStream) {
    let ty = self.ty.to_token_stream(ctx);
    let name = self.name.to_token_stream(ctx);
    let rhs = self.rhs.to_token_stream(ctx);

    tokens.append_all(if self.is_static {
      quote! { static mut #name: #ty = #rhs; }
    } else {
      quote! { let mut #name: #ty = #rhs }
    })
  }
}

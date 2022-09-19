use super::*;

#[derive(Debug, Clone)]
pub struct FunctionCall<'t> {
  pub name: Identifier,
  pub args: Vec<Expr<'t>>,
}

impl FunctionCall<'_> {
  pub fn to_tokens(&self, ctx: &mut Context, tokens: &mut TokenStream) {
    let mut name = TokenStream::new();
    self.name.to_tokens(ctx, &mut name);

    let args = self.args.iter().map(|arg| {
      let into = matches!(arg, Expr::Variable { .. }) && !matches!(arg, Expr::Variable { name: Identifier::Literal(id) } if id == "NULL");

      let arg = arg.to_token_stream(ctx);

      if into {
        return quote! { #arg.into() }
      }

      arg
    }).collect::<Vec<_>>();

    tokens.append_all(quote! {
      #name(#(#args),*)
    })
  }
}

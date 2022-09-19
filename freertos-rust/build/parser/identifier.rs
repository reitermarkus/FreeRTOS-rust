
use super::*;

pub fn identifier<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], &'t str> {
  if let Some(token) = tokens.get(0) {
    let mut it = token.chars();
    if let Some('a'..='z' | 'A'..='Z' | '_') = it.next() {
      if it.all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9')) {
        return Ok((&tokens[1..], token))
      }
    }
  }

  Err(nom::Err::Error(nom::error::Error::new(tokens, nom::error::ErrorKind::Fail)))
}

#[derive(Debug, Clone)]
pub enum Identifier {
  Literal(String),
  Concat(Vec<String>)
}

impl Identifier {
  pub fn parse<'i, 't>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, id) = identifier(tokens)?;

    let (tokens, id) = fold_many0(
      preceded(tuple((meta, token("##"), meta)), identifier),
      move || Self::Literal(id.to_owned()),
      |acc, item| {
        match acc {
          Self::Literal(id) => Self::Concat(vec![id.to_owned(), item.to_owned()]),
          Self::Concat(mut ids) => {
            ids.push(item.to_owned());
            Self::Concat(ids)
          }
        }
      }
    )(tokens)?;

    if let Self::Concat(ref ids) = id {
      for id in ids {
        if let Some(arg_ty) = ctx.args.get(id.as_str()) {
          arg_ty.set(MacroArgType::Ident);
        }
      }
    }

    Ok((tokens, id))
  }

  pub fn to_tokens(&self, ctx: &mut Context, tokens: &mut TokenStream) {
    match self {
      Self::Literal(s) => {
        let mut id = Ident::new(s, Span::call_site());

        if ctx.is_macro_arg(s.as_str()) {
          return tokens.append_all(quote! { $#id })
        }

        tokens.append(id)
      },
      Self::Concat(ids) => {
        let ids = ids.iter().map(|id| Self::Literal(id.to_owned()).to_token_stream(ctx)).collect::<Vec<_>>();

        tokens.append_all(quote! {
          ::core::concat_idents!(
            #(#ids),*
          )
        })
      },
    }
  }

  pub fn to_token_stream(&self, ctx: &mut Context) -> TokenStream {
    let mut tokens = TokenStream::new();
    self.to_tokens(ctx, &mut tokens);
    tokens
  }
}

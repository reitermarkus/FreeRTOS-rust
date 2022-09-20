use quote::TokenStreamExt;
use quote::ToTokens;

use super::*;

pub struct LitIdentifier {
  id: String,
}

impl LitIdentifier {
  pub fn parse<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    map(identifier, |id| Self { id: id.to_owned() })(tokens)
  }
}

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

fn concat_identifier<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], &'t str> {
  if let Some(token) = tokens.get(0) {
    let mut it = token.chars();
    if it.all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9')) {
      return Ok((&tokens[1..], token))
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
  pub fn parse<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, id) = identifier(tokens)?;

    fold_many0(
      preceded(
        delimited(meta, token("##"), meta),
        concat_identifier,
      ),
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
    )(tokens)
  }

  pub fn visit<'s, 't>(&mut self, ctx: &mut Context<'s, 't>) {
    if let Self::Concat(ref ids) = self {
      for id in ids {
        if let Some(arg_ty) = ctx.args.get_mut(id.as_str()) {
          *arg_ty = MacroArgType::Ident;
          ctx.export_as_macro = true;
        }
      }
    }
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

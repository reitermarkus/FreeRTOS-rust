use nom::combinator::cond;
use quote::ToTokens;

use super::*;

#[derive(Debug, Clone)]
pub enum Lit {
  String(LitString),
  Int(LitInt),
}

impl Lit {
  pub fn parse<'i, 't>(input: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    alt((
      map(LitString::parse, Self::String),
      map(LitInt::parse, Self::Int),
    ))(input)
  }
}

impl ToTokens for Lit {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      Self::String(s) => s.to_tokens(tokens),
      Self::Int(i) => i.to_tokens(tokens),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LitString {
  repr: String,
}

impl LitString {
  pub fn parse<'i, 't>(input: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    if let Some(token) = input.get(0) {
      let input = &input[1..];

      if token.starts_with("\"") && token.ends_with("\"") {
        return Ok((input, Self { repr: token[1..(token.len() - 1)].to_owned() }))
      }
    }

    Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Fail)))
  }
}

impl PartialEq<&str> for LitString {
  fn eq(&self, other: &&str) -> bool {
    self.repr == *other
  }
}

impl ToTokens for LitString {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.repr.to_tokens(tokens)
  }
}

/// An integer literal.
///
/// ```c
/// #define MY_INT1 1ull
/// #define MY_INT2 1u ## LL
/// #define MY_INT3 1 ## ULL
/// ```
#[derive(Debug, Clone)]
pub struct LitInt {
  repr: String,
}

impl LitInt {
  fn from_str(input: &str) -> IResult<&str, (&str, Option<&str>, Option<&str>)> {
    use nom::character::complete::{char, digit1};
    use nom::bytes::complete::tag;

    let suffix_unsigned = alt((tag("u"), tag("U")));
    let suffix_long = alt((tag("l"), tag("L")));
    let suffix_long_long = alt((tag("ll"), tag("LL")));
    let suffix_size_t = alt((tag("z"), tag("Z")));

    let suffix = permutation((
      opt(map(suffix_unsigned, |_| "u")),
      opt(
        alt((
          suffix_long_long,
          suffix_long,
          suffix_size_t,
        ))
      )
    ));

    let (input, (repr, (unsigned, size))) = terminated(pair(digit1, suffix), eof)(input)?;
    Ok((input, (repr, unsigned, size)))
  }

  pub fn parse<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    if let Some(Ok((_, (repr, unsigned1, size1)))) = tokens.get(0).copied().map(Self::from_str) {
      let tokens = &tokens[1..];

      let suffix_unsigned = alt((token("u"), token("U")));
      let suffix_long = alt((token("l"), token("L")));
      let suffix_long_long = alt((token("ll"), token("LL")));
      let suffix_size_t = alt((token("z"), token("Z")));

      let mut suffix = map(
        permutation((
          cond(unsigned1.is_none(), opt(preceded(delimited(meta, token("##"), meta), suffix_unsigned))),
          cond(size1.is_none(), opt(preceded(delimited(meta, token("##"), meta), alt((
            suffix_long_long,
            suffix_long,
            suffix_size_t,
          )))))
        )),
        |(unsigned, size)| (unsigned.flatten(), size.flatten()),
      );

      let (tokens, (unsigned2, size2)) = suffix(tokens)?;
      let unsigned = unsigned1.is_some() || unsigned2.is_some();
      let size = size1.or_else(|| size2);

      // TODO: Handle suffix.
      return Ok((tokens, Self { repr: repr.to_owned() }))
    }

    Err(nom::Err::Error(nom::error::Error::new(tokens, nom::error::ErrorKind::Fail)))
  }
}

impl ToTokens for LitInt {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    tokens.append_all(self.repr.parse::<TokenStream>().unwrap())
  }
}

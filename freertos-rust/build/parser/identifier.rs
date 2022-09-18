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
pub enum Identifier<'t> {
  Literal(&'t str),
  Concat(Vec<&'t str>)
}

impl<'t> Identifier<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, id) = identifier(tokens)?;

    fold_many0(
      preceded(tuple((meta, token("##"), meta)), identifier),
      move || Self::Literal(id),
      |acc, item| {
        match acc {
          Self::Literal(id) => Self::Concat(vec![id, item]),
          Self::Concat(mut ids) => {
            ids.push(item);
            Self::Concat(ids)
          }
        }
      }
    )(tokens)
  }
}

impl fmt::Display for Identifier<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Literal(s) => s.fmt(f),
      Self::Concat(ids) => {
        write!(f, "::core::concat_idents!(")?;
        for id in ids {
          write!(f, "{},", id)?;
        }
        write!(f, ")")
      }
    }
  }
}

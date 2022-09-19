use nom::combinator::cond;

use super::*;

pub fn number_literal<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], &'t str> {
  use nom::character::complete::{char, digit1};

  if let Some(t) = tokens.get(0) {
    let tokens = &tokens[1..];

    let t: &str = t;

    // let decimal = pair(complete::char('.'), digit1);
    let str_unsigned = alt((char('u'), char('U')));
    let str_long = alt((char('l'), char('L')));
    let str_long_long = alt((
      pair(char('l'), char('l')),
      pair(char('L'), char('L')),
    ));
    let str_size_t = alt((char('z'), char('Z')));

    let suffix = permutation((
      opt(map(str_unsigned, |_| "u")),
      opt(
        alt((
          map(str_long_long, |_| "ll"),
          map(str_long, |_| "l"),
          map(str_size_t, |_| "z"),
        ))
      )
    ));

    let res: IResult<&str, (&str, (Option<&str>, Option<&str>), &str)> = tuple((digit1, suffix, eof))(t);

    if let Ok((_, (n, (unsigned1, size1), _))) = res {
      let token_unsigned = alt((token("u"), token("U")));
      let token_long = alt((token("l"), token("L")));
      let token_long_long = alt((token("ll"), token("LL")));
      let token_size_t = alt((token("z"), token("Z")));

      let mut suffix2 = permutation((
        cond(unsigned1.is_none(), opt(preceded(delimited(meta, token("##"), meta), map(token_unsigned, |_| "u")))),
        cond(size1.is_none(), opt(preceded(delimited(meta, token("##"), meta), alt((
          map(token_long_long, |_| "ll"),
          map(token_long, |_| "l"),
          map(token_size_t, |_| "z"),
        )))))
      ));

      let (tokens, (unsigned2, size2)) = suffix2(tokens)?;
      let unsigned = unsigned1.is_some() || unsigned2.is_some();
      let size = size1.or_else(|| size2.flatten());

      // TODO: Handle suffix.
      return Ok((tokens, n))
    }
  }

  Err(nom::Err::Error(nom::error::Error::new(tokens, nom::error::ErrorKind::Fail)))
}

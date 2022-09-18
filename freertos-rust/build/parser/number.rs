use super::*;

pub fn number_literal<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], &'t str> {
  use nom::character::complete::{char, digit1};

  if let Some(token) = tokens.get(0) {
    let token: &str = token;

    // let decimal = pair(complete::char('.'), digit1);
    let unsigned = alt((char('u'), char('U')));
    let long = alt((char('l'), char('L')));
    let long_long = alt((
      pair(char('l'), char('l')),
      pair(char('L'), char('L')),
    ));
    let size_t = alt((char('z'), char('Z')));

    let suffix = permutation((
      opt(map(unsigned, |_| "u")),
      opt(
        alt((
          map(long_long, |_| "ll"),
          map(long, |_| "l"),
          map(size_t, |_| "z"),
        ))
      )
    ));

    let res: IResult<&str, (&str, (Option<&str>, Option<&str>), &str)> = tuple((digit1, suffix, eof))(token);

    if let Ok((_, (n, (unsigned, size), _))) = res {
      // TODO: Handle suffix.
      return Ok((&tokens[1..], n))
    }
  }

  Err(nom::Err::Error(nom::error::Error::new(tokens, nom::error::ErrorKind::Fail)))
}

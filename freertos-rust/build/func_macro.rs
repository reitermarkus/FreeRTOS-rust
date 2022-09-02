pub fn parse_whitespace(s: &str) -> Option<&str> {
  let mut it = s.chars().peekable();
  let mut i = 0;
  loop {
    match it.next() {
      Some(c) if c.is_whitespace() => (),
      Some('\\') if it.peek() == Some(&'\r') || it.peek() == Some(&'\n') => {
        it.next();
        i += 1;
      }
      _ => break,
    }

    i += 1;
  }

  if i == 0 {
    None
  } else {
    Some(&s[i..])
  }
}

pub fn parse_comment(s: &str) -> Option<&str> {
  let mut it = s.chars().peekable();

  if let Some(('/', '*')) = it.next().zip(it.next()) {
    let mut i = 2;

    let mut it = s.chars().skip(2).peekable();
    while let Some(c) = it.next() {
      if c == '*' && it.peek() == Some(&'/') {
        i += 2;
        return Some(&s[i..])
      }

      i += 1;
    }
  }

  None
}

pub fn skip_meta(mut s: &str) -> &str {
  while let Some(s2) = parse_whitespace(s).or_else(|| parse_comment(s)) {
    s = s2;
  }

  s
}

pub fn parse_end(s: &str) -> Option<()> {
  if s.len() == 0 {
    Some(())
  } else {
    None
  }
}

pub fn parse_char(s: &str, c: char) -> Option<&str> {
  if s.chars().next()? == c {
    return Some(&s[1..])
  }

  None
}

pub fn parse_string(s: &str) -> Option<(&str, &str)> {
  let s2 = parse_char(s, '"')?;
  let end = s2.chars().position(|c| c == '"')? + 2;

  Some((&s[..end], &s[end..]))
}

pub fn parse_ident(s: &str) -> Option<(String, &str)> {
  let mut ident = String::new();

  for c in s.chars() {
    match c {
      'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => ident.push(c),
      _ => break,
    }
  }

  if ident.is_empty() {
    None
  } else {
    let s = &s[ident.len()..];
    Some((ident, s))
  }
}

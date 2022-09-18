use super::*;

#[derive(Debug)]
pub struct Decl<'t> {
  ty: Type<'t>,
  name: Identifier<'t>,
  rhs: Expr<'t>,
  is_static: bool,
}

impl<'t> Decl<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    eprintln!("Decl::parse {:?}", tokens);

    let (tokens, ((static_storage, ty), name, _, rhs)) = tuple((
      permutation((opt(token("static")), Type::parse)),
      Identifier::parse, token("="), Expr::parse,
    ))(tokens)?;

    Ok((tokens, Self { ty, name, rhs, is_static: static_storage.is_some() }))
  }
}

impl fmt::Display for Decl<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.is_static {
      write!(f, "static mut {}: {} = {};", self.name, self.ty, self.rhs)
    } else {
      write!(f, "let {}: {} = {};", self.name, self.ty, self.rhs)
    }
  }
}

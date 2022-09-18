use super::*;

#[derive(Debug, Clone)]
pub enum Type<'t> {
  Identifier(Identifier<'t>),
  Ptr { ty: Box<Self>, mutable: bool },
}

impl<'t> Type<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, (_, (strvct, ty), _)) = tuple((
      many0_count(token("const")), pair(opt(token("struct")), Identifier::parse), many0_count(token("const")),
    ))(tokens)?;

    fold_many0(
      preceded(pair(token("*"), meta), many0_count(token("const"))),
      move || Type::Identifier(ty.clone()),
      |acc, constness| {
        Type::Ptr { ty: Box::new(acc), mutable: constness == 0 }
      },
    )(tokens)
  }
}

impl fmt::Display for Type<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Identifier(id) => id.fmt(f),
      Self::Ptr { ty, mutable } => {
        if *mutable {
          write!(f, "*mut {}", ty)
        } else {
          write!(f, "*const {}", ty)
        }
      }
    }
  }
}

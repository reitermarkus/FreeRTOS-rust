use super::*;

#[derive(Debug)]
pub struct FunctionDecl<'t> {
  ret_ty: Type<'t>,
  name: Identifier<'t>,
  args: Vec<(Type<'t>, Identifier<'t>)>,
  is_static: bool,
}

impl<'t> FunctionDecl<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    eprintln!("Decl::parse {:?}", tokens);

    let (tokens, ((static_storage, ret_ty), name, args)) = tuple((
      permutation((opt(token("static")), Type::parse)),
      Identifier::parse,
      delimited(
        pair(token("("), meta),
        separated_list0(pair(meta, token(",")), pair(Type::parse, Identifier::parse)),
        pair(meta, token(")")),
      ),
    ))(tokens)?;

    Ok((tokens, Self { ret_ty, name, args, is_static: static_storage.is_some() }))
  }
}

impl fmt::Display for FunctionDecl<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    Ok(())
  }
}

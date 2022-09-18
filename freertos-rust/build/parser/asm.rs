use super::*;

#[derive(Debug, Clone)]
pub struct Asm<'t> {
  template: Vec<&'t str>,
  outputs: Vec<Expr<'t>>,
  inputs: Vec<Expr<'t>>,
  clobbers: Vec<Expr<'t>>,
}

impl<'t> Asm<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, (template, outputs, inputs, clobbers)) = delimited(
      pair(token("("), meta),
      tuple((
        separated_list0(tuple((meta, token(","), meta)), string_literal),
        opt(preceded(token(":"), separated_list0(tuple((meta, token(","), meta)), Expr::parse))),
        opt(preceded(token(":"), separated_list0(tuple((meta, token(","), meta)), Expr::parse))),
        opt(preceded(token(":"), separated_list0(tuple((meta, token(","), meta)), Expr::parse))),
      )),
      pair(meta, token(")")),
    )(tokens)?;

    let outputs = outputs.unwrap_or_default();
    let inputs = inputs.unwrap_or_default();

    let clobbers = clobbers.unwrap_or_default().into_iter().filter_map(|c| match c {
        Expr::Literal(s) if s == r#""memory""# => None,
        clobber => Some(Expr::Literal(format!("out({}) _", clobber))),
    }).collect::<Vec<_>>();

    Ok((tokens, Self { template, outputs, inputs, clobbers }))
  }
}

impl fmt::Display for Asm<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "::core::arch::asm!(")?;

    let template = self.template.join("");
    write!(f, "{}", template)?;

    // TODO: Outputs/inputs.

    write!(f, r#", clobber_abi("C")"#)?;
    write!(f, ", options(raw)")?;

    write!(f, ")")
  }
}

use super::*;

#[derive(Debug, Clone)]
pub struct FunctionCall<'t> {
  pub name: Identifier<'t>,
  pub args: Vec<Expr<'t>>,
}

impl fmt::Display for FunctionCall<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}(", self.name)?;

    for (i, arg) in self.args.iter().enumerate() {
      if i > 0 {
        write!(f, ", ")?;
      }

      write!(f, "{}", arg)?;
    }
    write!(f, ")")
  }
}

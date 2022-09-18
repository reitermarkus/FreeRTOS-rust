use super::*;

#[derive(Debug)]
pub enum Statement<'t> {
  Expr(Expr<'t>),
  Decl(Decl<'t>),
  Block(Vec<Self>),
  If {
    condition: Expr<'t>,
    if_branch: Vec<Statement<'t>>,
    else_branch: Vec<Statement<'t>>
  },
  DoWhile { block: Vec<Statement<'t>>, condition: Expr<'t> },
}

impl<'t> Statement<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let condition = || delimited(pair(token("("), meta), Expr::parse, pair(meta, token(")")));

    let block = || map(Self::parse, |stmt| if let Self::Block(stmts) = stmt { stmts } else { vec![stmt] } );

    alt((
      map(
        delimited(token("{"), many0(preceded(meta, Self::parse)), pair(meta, token("}"))),
        |statements| Self::Block(statements),
      ),
      map(
        tuple((
          preceded(pair(token("if"), meta), condition()),
          block(),
          opt(preceded(
            tuple((meta, token("else"), meta)),
            block(),
          )),
        )),
        |(condition, if_branch, else_branch)| {
          Self::If { condition, if_branch, else_branch: else_branch.unwrap_or_default() }
        }
      ),
      map(
        preceded(
          pair(token("do"), meta),
          pair(
            block(),
            preceded(token("while"), condition()),
          ),
        ),
        |(block, condition)| Self::DoWhile { block, condition }
      ),
      map(
        terminated(Decl::parse, token(";")),
        Self::Decl,
      ),
      map(
        terminated(Expr::parse, token(";")),
        Self::Expr,
      ),
    ))(tokens)
  }
}

impl fmt::Display for Statement<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Expr(expr) => expr.fmt(f),
      Self::Decl(a) => a.fmt(f),
      Self::Block(block) => {
        write!(f, "{{")?;

        for stmt in block {
          write!(f, "{}", stmt)?;
        }

        write!(f, "}}")
      },
      Self::If { condition, if_branch, else_branch } => {
        write!(f, "if {} {{", condition)?;

        for stmt in if_branch {
          write!(f, "{}", stmt)?;
        }

        if !else_branch.is_empty() {
          write!(f, "}} else {{")?;

          for stmt in else_branch {
            write!(f, "{}", stmt)?;
          }
        }

        write!(f, "}}")
      },
      Self::DoWhile { block, condition } => {
        write!(f, "loop {{")?;
        for stmt in block {
          write!(f, "{};", stmt)?;
        }
        write!(f, "if ({} as u8 == 0) {{ break }}", condition)?;
        write!(f, "}}")?;
        Ok(())
      }
    }
  }
}

use super::*;

#[derive(Debug, Clone)]
pub enum Expr<'t> {
  Variable { name: Identifier<'t> },
  FunctionCall(FunctionCall<'t>),
  Cast { expr: Box<Expr<'t>>, ty: Type<'t> },
  Literal(String),
  Deref { expr: Box<Self>, field: Identifier<'t> },
  Stringify(&'t str),
  Concat(Vec<Expr<'t>>),
  UnaryOp { op: &'t str, expr: Box<Self>, prefix: bool },
  BinOp(Box<Self>, &'t str, Box<Self>),
  Ternary(Box<Self>, Box<Self>, Box<Self>),
  AddrOf(Box<Self>),
  Asm(Asm<'t>),
}

impl<'t> Expr<'t> {
  fn parse_string<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let mut parse_string = alt((
      map(string_literal, |s| Self::Literal(s.to_owned())),
      map(preceded(pair(token("#"), meta), identifier), Self::Stringify),
    ));

    let (tokens, s) = parse_string(tokens)?;

    fold_many0(
      preceded(meta, parse_string),
      move || s.clone(),
      |mut acc, item| {
        match acc {
          Self::Concat(ref mut args) => {
            args.push(item);
            acc
          },
          acc => Self::Concat(vec![acc, item]),
        }
      }
    )(tokens)
  }

  fn parse_factor<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    alt((
      Self::parse_string,
      map(number_literal, |n| Self::Literal(n.to_owned())),
      map(Identifier::parse, |id| Self::Variable { name: id }),
      delimited(pair(token("("), meta), Self::parse, pair(meta, token(")"))),
    ))(tokens)
  }

  fn parse_term_prec1<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, factor) = Self::parse_factor(tokens)?;

    let (tokens, arg) = match factor {
      arg @ Expr::Variable { .. } | arg @ Expr::FunctionCall(..) |
      arg @ Expr::Deref { .. } | arg @ Expr::AddrOf(..) => {
        enum Access<'t> {
          Fn(Vec<Expr<'t>>),
          Field(Identifier<'t>),
        }

        if matches!(arg, Expr::Variable { name: Identifier::Literal("__asm") }) {
          if let Ok((tokens, asm)) = preceded(opt(token("volatile")), Asm::parse)(tokens) {
            return Ok((tokens, Expr::Asm(asm)))
          }
        }

        let (tokens, arg) = fold_many0(
          alt((
            map(
              delimited(
                pair(token("("), meta),
                separated_list0(tuple((meta, token(","), meta)), Self::parse),
                pair(meta, token(")")),
              ),
              |args| Access::Fn(args)
            ),
            map(
              pair(alt((token("."), token("->"))), Identifier::parse),
              |(access, field)| Access::Field(field),
            ),
          )),
          move || arg.clone(),
          |acc, access| match (acc, access) {
            (Expr::Variable { name }, Access::Fn(args)) => Expr::FunctionCall(FunctionCall { name, args }),
            (acc, Access::Field(field)) => Expr::Deref { expr: Box::new(acc), field },
            _ => unimplemented!(),
          },
        )(tokens)?;

        if let Ok((tokens, op)) = alt((token("++"), token("--")))(tokens) {
          (tokens, Expr::UnaryOp { op, expr: Box::new(arg), prefix: false })
        } else {
          (tokens, arg)
        }
      },
      arg => (tokens, arg),
    };

    Ok((tokens, arg))
  }

  fn parse_term_prec2<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    eprintln!("parse_term_prec2 tokens = {:?}", tokens);

    alt((
      map(
        pair(
          delimited(
            pair(token("("), meta),
            Type::parse,
            pair(meta, token(")")),
          ),
          Self::parse_term_prec2,
        ),
        |(ty, term)| {
          // TODO: Handle constness.
          Expr::Cast { expr: Box::new(term), ty }
        },
      ),
      map(
        preceded(pair(token("&"), meta), Self::parse_term_prec2),
        |expr| Expr::AddrOf(Box::new(expr)),
      ),
      map(
        pair(
          alt((
            token("++"), token("--"),
            token("+"), token("-"),
            token("!"), token("~"),
          )),
          Self::parse_term_prec2,
        ),
        |(op, term)| {
          Expr::UnaryOp { op, expr: Box::new(term), prefix: true }
        }
      ),
      Self::parse_term_prec1,
    ))(tokens)
  }

  fn parse_term_prec3<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, factor) = Self::parse_term_prec2(tokens)?;

    fold_many0(
      pair(alt((token("*"), token("/"))), Self::parse_term_prec2),
      move || factor.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec4<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec3(tokens)?;

    fold_many0(
      pair(alt((token("+"), token("-"))), Self::parse_term_prec3),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec5<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec4(tokens)?;

    fold_many0(
      pair(alt((token("<<"), token(">>"))), Self::parse_term_prec4),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec6<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec5(tokens)?;

    eprintln!("parse_term_prec6 term = {:?}", term);
    eprintln!("parse_term_prec6 rest = {:?}", tokens);

    fold_many0(
      pair(alt((token("<"), token("<="), token(">"), token(">="))), Self::parse_term_prec5),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec7<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec6(tokens)?;

    eprintln!("parse_term_prec7 term = {:?}", term);
    eprintln!("parse_term_prec7 rest = {:?}", tokens);

    fold_many0(
      pair(alt((token("=="), token("!="))), Self::parse_term_prec6),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec13<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec7(tokens)?;

    // Parse ternary.
    if let Ok((tokens, _)) = token("?")(tokens) {
      let (tokens, if_branch) = Self::parse_term_prec7(tokens)?;
      let (tokens, _) = token(":")(tokens)?;
      let (tokens, else_branch) = Self::parse_term_prec7(tokens)?;
      return Ok((tokens, Expr::Ternary(Box::new(term), Box::new(if_branch), Box::new(else_branch))))
    }

    Ok((tokens, term))
  }

  fn parse_term_prec14<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec13(tokens)?;

    fold_many0(
      pair(
        alt((
          token("="),
          token("+="), token("-="),
          token("*="), token("/="), token("%="),
          token("<<="), token(">>="),
          token("&="), token("^="), token("|="),
        )),
        Self::parse_term_prec13,
      ),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    eprintln!("Expr::parse tokens = {:?}", tokens);

    let res = Self::parse_term_prec14(tokens);

    eprintln!("Expr::parse res = {:?}", res);

    res
  }
}

impl fmt::Display for Expr<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *self {
      Self::Cast { ref expr, ref ty } => {
        if matches!(ty, Type::Identifier(Identifier::Literal("void"))) {
          write!(f, "drop({})", expr)
        } else {
          write!(f, "{:#} as {}", expr, ty)
        }
      },
      Self::Variable { name: Identifier::Literal("NULL") } => {
        write!(f, "::core::ptr::null_mut()")
      },
      Self::Variable { name: Identifier::Literal("eIncrement") } => {
        write!(f, "eNotifyAction_eIncrement")
      },
      Self::Variable { ref name } => {
        if f.alternate() {
          write!(f, "{}", name)
        } else {
          write!(f, "{}.into()", name)
        }
      },
      Self::FunctionCall(ref call) => call.fmt(f),
      Self::Literal(ref lit) => lit.fmt(f),
      Self::Deref { ref expr, ref field } => write!(f, "Deref(unsafe {{ &mut *::core::ptr::addr_of_mut!((*{:#}).{}) }}).convert()", expr, field),
      Self::Stringify(name) => write!(f, "::core::stringify!({})", name),
      Self::Concat(ref names) => {
        write!(f, "::core::concat!(")?;
        for name in names.iter() {
          write!(f, "{},", name)?;
        }
        write!(f, ")")
      },
      Self::UnaryOp { ref op, ref expr, prefix } => {
        match (*op, prefix) {
          ("++", true) => write!(f, "{{ {} -= 1; {} }}", expr, expr),
          ("--", true) => write!(f, "{{ {} -= 1; {} }}", expr, expr),
          ("++", false) => write!(f, "{{ let prev = {}; {} -= 1; prev }}", expr, expr),
          ("--", false) => write!(f, "{{ let prev = {}; {} -= 1; prev }}", expr, expr),
          ("!" | "~", _) => write!(f, "!{}", expr),
          _ => write!(f, "{}{}", op, expr),
        }
      },
      Self::BinOp(ref lhs, op, ref rhs) => {
        write!(f, "{} {} {}", lhs, op, rhs)
      },
      Self::Ternary(ref cond, ref if_branch, ref else_branch) => {
        write!(f, "if {} {{", cond)?;
        write!(f, "{}", if_branch)?;
        write!(f, "}} else {{")?;
        write!(f, "{}", else_branch)?;
        write!(f, "}}")
      },
      Self::AddrOf(ref expr) => {
        write!(f, "::core::ptr::addr_of_mut({})", expr)
      },
      Self::Asm(ref asm) => asm.fmt(f),
    }
  }
}

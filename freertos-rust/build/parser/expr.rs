use super::*;

#[derive(Debug, Clone)]
pub enum Expr<'t> {
  Variable { name: Identifier },
  FunctionCall(FunctionCall<'t>),
  Cast { expr: Box<Expr<'t>>, ty: Type },
  Literal(String),
  Deref { expr: Box<Self>, field: Identifier },
  Stringify(Identifier),
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
      map(preceded(pair(token("#"), meta), identifier), |id| Self::Stringify(Identifier::Literal(id.to_owned()))),
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

  fn parse_factor<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    alt((
      Self::parse_string,
      map(number_literal, |n| Self::Literal(n.to_owned())),
      map(|tokens| Identifier::parse(ctx, tokens), |id| Self::Variable { name: id }),
      delimited(pair(token("("), meta), |tokens| Self::parse(ctx, tokens), pair(meta, token(")"))),
    ))(tokens)
  }

  fn parse_term_prec1<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, factor) = Self::parse_factor(ctx, tokens)?;

    let (tokens, arg) = match factor {
      arg @ Expr::Variable { .. } | arg @ Expr::FunctionCall(..) |
      arg @ Expr::Deref { .. } | arg @ Expr::AddrOf(..) => {
        enum Access<'t> {
          Fn(Vec<Expr<'t>>),
          Field(Identifier),
        }

        if matches!(arg, Expr::Variable { name: Identifier::Literal(ref id) } if id == "__asm") {
          if let Ok((tokens, asm)) = preceded(opt(token("volatile")), |tokens| Asm::parse(ctx, tokens))(tokens) {
            return Ok((tokens, Expr::Asm(asm)))
          }
        }

        let (tokens, arg) = fold_many0(
          alt((
            map(
              delimited(
                pair(token("("), meta),
                separated_list0(tuple((meta, token(","), meta)), |tokens| Self::parse(ctx, tokens)),
                pair(meta, token(")")),
              ),
              |args| Access::Fn(args)
            ),
            map(
              pair(alt((token("."), token("->"))), |tokens| Identifier::parse(ctx, tokens)),
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

  fn parse_term_prec2<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    alt((
      map(
        pair(
          delimited(
            pair(token("("), meta),
            |tokens| Type::parse(ctx, tokens),
            pair(meta, token(")")),
          ),
          |tokens| Self::parse_term_prec2(ctx, tokens),
        ),
        |(ty, term)| {
          // TODO: Handle constness.
          Expr::Cast { expr: Box::new(term), ty }
        },
      ),
      map(
        preceded(pair(token("&"), meta), |tokens| Self::parse_term_prec2(ctx, tokens)),
        |expr| Expr::AddrOf(Box::new(expr)),
      ),
      map(
        pair(
          alt((
            token("++"), token("--"),
            token("+"), token("-"),
            token("!"), token("~"),
          )),
          |tokens| Self::parse_term_prec2(ctx, tokens),
        ),
        |(op, term)| {
          Expr::UnaryOp { op, expr: Box::new(term), prefix: true }
        }
      ),
      |tokens| Self::parse_term_prec1(ctx, tokens),
    ))(tokens)
  }

  fn parse_term_prec3<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, factor) = Self::parse_term_prec2(ctx, tokens)?;

    fold_many0(
      pair(alt((token("*"), token("/"))), |tokens| Self::parse_term_prec2(ctx, tokens)),
      move || factor.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec4<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec3(ctx, tokens)?;

    fold_many0(
      pair(alt((token("+"), token("-"))), |tokens| Self::parse_term_prec3(ctx, tokens)),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec5<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec4(ctx, tokens)?;

    fold_many0(
      pair(alt((token("<<"), token(">>"))), |tokens| Self::parse_term_prec4(ctx, tokens)),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec6<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec5(ctx, tokens)?;

    fold_many0(
      pair(alt((token("<"), token("<="), token(">"), token(">="))), |tokens| Self::parse_term_prec5(ctx, tokens)),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec7<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec6(ctx, tokens)?;

    fold_many0(
      pair(alt((token("=="), token("!="))), |tokens| Self::parse_term_prec6(ctx, tokens)),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec13<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec7(ctx, tokens)?;

    // Parse ternary.
    if let Ok((tokens, _)) = token("?")(tokens) {
      let (tokens, if_branch) = Self::parse_term_prec7(ctx, tokens)?;
      let (tokens, _) = token(":")(tokens)?;
      let (tokens, else_branch) = Self::parse_term_prec7(ctx, tokens)?;
      return Ok((tokens, Expr::Ternary(Box::new(term), Box::new(if_branch), Box::new(else_branch))))
    }

    Ok((tokens, term))
  }

  fn parse_term_prec14<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, term) = Self::parse_term_prec13(ctx, tokens)?;

    fold_many0(
      pair(
        alt((
          token("="),
          token("+="), token("-="),
          token("*="), token("/="), token("%="),
          token("<<="), token(">>="),
          token("&="), token("^="), token("|="),
        )),
        |tokens| Self::parse_term_prec13(ctx, tokens),
      ),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  pub fn parse<'i>(ctx: &Context<'_, '_>, tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    Self::parse_term_prec14(ctx, tokens)
  }

  pub fn to_tokens(&self, ctx: &mut Context, tokens: &mut TokenStream) {
    match self {
      Self::Cast { ref expr, ref ty } => {
        let expr = expr.to_token_stream(ctx);

        tokens.append_all(if matches!(ty, Type::Identifier { name: Identifier::Literal(id), .. } if id == "void") {
          quote! { { drop(#expr) } }
        } else {
          let ty = ty.to_token_stream(ctx);
          quote! { #expr as #ty }
        })
      },
      Self::Variable { name: Identifier::Literal(id) } if id == "NULL" => {
        tokens.append_all(quote! { ::core::ptr::null_mut() });
      },
      Self::Variable { name: Identifier::Literal(id) } if id == "eIncrement" => {
        tokens.append_all(quote! { eNotifyAction_eIncrement });
      },
      Self::Variable { ref name } => {
        name.to_tokens(ctx, tokens)
      },
      Self::FunctionCall(ref call) => {
        call.to_tokens(ctx, tokens);
      },
      Self::Literal(ref lit) => {
        let lit = lit.parse::<TokenStream>().unwrap();
        tokens.append_all(quote! { #lit });
      },
      Self::Deref { ref expr, ref field } => {
        let expr = expr.to_token_stream(ctx);
        let field = field.to_token_stream(ctx);

        tokens.append_all(quote! {
          Deref(unsafe {{ &mut *::core::ptr::addr_of_mut!((*#expr).#field) }}).convert()
        })
      },
      Self::Stringify(id) => {
        let id = id.to_token_stream(ctx);

        tokens.append_all(quote! {
          ::core::stringify!(#id)
        })
      },
      Self::Concat(ref names) => {
        let names = names.iter().map(|e| e.to_token_stream(ctx)).collect::<Vec<_>>();

        tokens.append_all(quote! {
          ::core::concat!(
            #(#names),*
          )
        })
      },
      Self::UnaryOp { ref op, ref expr, prefix } => {
        let expr = expr.to_token_stream(ctx);

        tokens.append_all(match (*op, prefix) {
          ("++", true) => quote! { { #expr += 1; #expr } },
          ("--", true) => quote! { { #expr -= 1; #expr } },
          ("++", false) => quote! { { let prev = #expr; #expr += 1; prev } },
          ("--", false) => quote! { { let prev = #expr; #expr -= 1; prev } },
          ("!", _) => quote! { (#expr == Default::default()) },
          ("~", _) => quote! { !#expr },
          ("+", _) => quote! { +#expr },
          ("-", _) => quote! { -#expr },
          (op, _) => todo!("op = {:?}", op),
        })
      },
      Self::BinOp(ref lhs, ref op, ref rhs) => {
        let lhs = lhs.to_token_stream(ctx);
        let rhs = rhs.to_token_stream(ctx);

        tokens.append_all(match *op {
          "="  => quote! { { #lhs  = #rhs; #lhs } },
          "+=" => quote! { { #lhs += #rhs; #lhs } },
          "-=" => quote! { { #lhs -= #rhs; #lhs } },
          "&=" => quote! { { #lhs &= #rhs; #lhs } },
          "|=" => quote! { { #lhs |= #rhs; #lhs } },
          "^=" => quote! { { #lhs ^= #rhs; #lhs } },
          "+"  => quote! { ( #lhs +  #rhs ) },
          "-"  => quote! { ( #lhs -  #rhs ) },
          "*"  => quote! { ( #lhs *  #rhs ) },
          "/"  => quote! { ( #lhs /  #rhs ) },
          "&"  => quote! { ( #lhs &  #rhs ) },
          "|"  => quote! { ( #lhs |  #rhs ) },
          "^"  => quote! { ( #lhs ^  #rhs ) },
          op   => todo!("op {:?}", op),
        });
      },
      Self::Ternary(ref cond, ref if_branch, ref else_branch) => {
        let cond = cond.to_token_stream(ctx);
        let if_branch = if_branch.to_token_stream(ctx);
        let else_branch = else_branch.to_token_stream(ctx);

        tokens.append_all(quote! {

          if #cond {
            #if_branch
          } else {
            #else_branch
          }
        })
      },
      Self::AddrOf(ref expr) => {
        let expr = expr.to_token_stream(ctx);

        tokens.append_all(quote! {
          ::core::ptr::addr_of_mut(#expr)
        })
      },
      Self::Asm(ref asm) => asm.to_tokens(ctx, tokens),
    }
  }

  pub fn to_token_stream(&self, ctx: &mut Context) -> TokenStream {
    let mut tokens = TokenStream::new();
    self.to_tokens(ctx, &mut tokens);
    tokens
  }
}

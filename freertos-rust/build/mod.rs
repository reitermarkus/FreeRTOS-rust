use std::env;
use std::fmt;
use std::io::Write;
use std::str;
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Mutex, Arc};
use core::num::NonZeroUsize;

use bindgen::{callbacks::{ParseCallbacks, IntKind}};
use nom::multi::{separated_list0, separated_list1};
use nom::branch::alt;
use nom::combinator::map;
use nom::sequence::delimited;
use nom::combinator::{opt, eof};
use nom::sequence::tuple;
use nom::{IResult, Needed};
use nom::multi::{many0, many1, fold_many0, fold_many1};
use nom::sequence::pair;
use nom::sequence::preceded;
use nom::branch::permutation;
use nom::multi::many0_count;

mod build;
mod constants;

enum Type<'t> {
  Identifier(Identifier<'t>),
  Ptr { ty: Box<Self>, mutable: bool },
}

fn number_literal<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], &'t str> {
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
    dbg!(&tokens);

    if let Ok((_, (n, (unsigned, size), _))) = res {
      // TODO: Handle suffix.
      return Ok((&tokens[1..], n))
    }
  }

  Err(nom::Err::Error(nom::error::Error::new(tokens, nom::error::ErrorKind::Fail)))
}

fn string_literal<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], &'t str> {
  if let Some(token) = tokens.get(0) {
    if token.starts_with("\"") && token.ends_with("\"") {
      return Ok((&tokens[1..], token))
    }
  }

  Err(nom::Err::Error(nom::error::Error::new(tokens, nom::error::ErrorKind::Fail)))
}

#[derive(Debug, Clone)]
enum Identifier<'t> {
  Literal(&'t str),
  Concat(Vec<&'t str>)
}

fn identifier<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], &'t str> {
  if let Some(token) = tokens.get(0) {
    let mut it = token.chars();
    if let Some('a'..='z' | 'A'..='Z' | '_') = it.next() {
      if it.all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9')) {
        return Ok((&tokens[1..], token))
      }
    }
  }

  Err(nom::Err::Error(nom::error::Error::new(tokens, nom::error::ErrorKind::Fail)))
}

impl<'t> Identifier<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, id) = identifier(tokens)?;

    fold_many0(
      preceded(tuple((meta, token("##"), meta)), identifier),
      move || Self::Literal(id),
      |mut acc, item| {
        match acc {
          Self::Literal(id) => Self::Concat(vec![id, item]),
          Self::Concat(mut ids) => {
            ids.push(item);
            Self::Concat(ids)
          }
        }
      }
    )(tokens)
  }
}

impl fmt::Display for Identifier<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Literal(s) => s.fmt(f),
      Self::Concat(ids) => {
        write!(f, "::core::concat_idents!(")?;
        for id in ids {
          write!(f, "{},", id)?;
        }
        write!(f, ")")
      }
    }
  }
}

#[derive(Debug, Clone)]
enum Expr<'t> {
  Variable { name: Identifier<'t> },
  FunctionCall(FunctionCall<'t>),
  Cast { expr: Box<Expr<'t>>, cast: Identifier<'t>, ptr_level: usize },
  Literal(String),
  Deref { expr: Box<Self>, field: Identifier<'t> },
  Stringify(&'t str),
  Concat(Vec<Expr<'t>>),
  BinOp(Box<Self>, &'t str, Box<Self>),
  Ternary(Box<Self>, Box<Self>, Box<Self>),
  AddrOf(Box<Self>),
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
    eprintln!("parse_factor: {:?}", tokens);
    alt((
      Self::parse_string,
      map(number_literal, |n| Self::Literal(n.to_owned())),
      map(Identifier::parse, |id| Self::Variable { name: id }),
      delimited(pair(token("("), meta), Self::parse, pair(meta, token(")"))),
    ))(tokens)
  }

  fn parse_term_prec1<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    eprintln!("parse_term_prec1: {:?}", tokens);

    let (tokens, factor) = Self::parse_factor(tokens)?;

    eprintln!("parse_term_prec1: factor = {:?}", factor);

    let (tokens, arg) = match factor {
      arg @ Expr::Variable { .. } | arg @ Expr::FunctionCall(..) | arg @ Expr::Deref { .. } => {
        enum Access<'t> {
          Fn(Vec<Expr<'t>>),
          Field(Identifier<'t>),
        }

        fold_many0(
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
        )(tokens)?
      },
      arg => (tokens, arg),
    };

    Ok((tokens, arg))
  }

  fn parse_term_prec2<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    eprintln!("parse_term_prec2: {:?}", tokens);

    alt((
      map(
        pair(
          delimited(
            pair(token("("), meta),
            pair(permutation((Identifier::parse, opt(token("const")))), many0_count(token("*"))),
            pair(meta, token(")")),
          ),
          Self::parse_term_prec1,
        ),
        |(((cast, constness), ptr_level), term)| {
          // TODO: Handle constness.
          Expr::Cast { expr: Box::new(term), cast, ptr_level }
        },
      ),
      map(
        preceded(pair(token("&"), meta), Self::parse_term_prec1),
        |expr| Expr::AddrOf(Box::new(expr)),
      ),
      Self::parse_term_prec1,
    ))(tokens)
  }

  fn parse_term_prec3<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    eprintln!("parse_term_prec3: {:?}", tokens);

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
    eprintln!("parse_term_prec4: {:?}", tokens);

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
    eprintln!("parse_term_prec5: {:?}", tokens);

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
    eprintln!("parse_term_prec6: {:?}", tokens);

    let (tokens, term) = Self::parse_term_prec5(tokens)?;

    fold_many0(
      pair(alt((token("<"), token("<="), token(">"), token(">="))), Self::parse_term_prec5),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec7<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    eprintln!("parse_term_prec7: {:?}", tokens);

    let (tokens, term) = Self::parse_term_prec6(tokens)?;

    fold_many0(
      pair(alt((token("=="), token("!="))), Self::parse_term_prec6),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  fn parse_term_prec13<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    eprintln!("parse_term_prec13: {:?}", tokens);

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
    eprintln!("parse_term_prec14: {:?}", tokens);

    let (tokens, term) = Self::parse_term_prec13(tokens)?;

    fold_many0(
      pair(alt((token("="), token("+="), token("-="), token("*="), token("/="))), Self::parse_term_prec13),
      move || term.clone(),
      |lhs, (op, rhs)| {
        Self::BinOp(Box::new(lhs), op, Box::new(rhs))
      }
    )(tokens)
  }

  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, arg) = Self::parse_term_prec14(tokens)?;

    eprintln!("arg = {:?}, tokens = {:?}", arg, tokens);



    eprintln!("tokens: {:?}, arg: {:?}", tokens, arg);

    Ok((tokens, arg))
  }
}

impl fmt::Display for Expr<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *self {
      Self::Cast { ref expr, ref cast, ptr_level } => {
        if matches!(cast, Identifier::Literal("void")) && ptr_level == 0 {
          write!(f, "drop({})", expr)
        } else {
          write!(f, "{:#} as ", expr)?;
          for _ in 0..ptr_level {
            write!(f, "*mut ")?;
          }
          write!(f, "{}", cast)
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
        for (i, name) in names.iter().enumerate() {
          write!(f, "{},", name)?;
        }
        write!(f, ")")
      },
      Self::BinOp(ref lhs, op, ref rhs) => {
        write!(f, "({}) {} ({})", lhs, op, rhs)
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
      }
    }
  }
}

#[derive(Debug)]
struct MacroSig<'t> {
  name: &'t str,
  arguments: Vec<&'t str>,
}

impl<'t> MacroSig<'t> {
  pub fn parse<'i>(input: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (input, name) = identifier(input)?;

    let (input, arguments) = delimited(
      pair(token("("), meta),
      alt((
        map(
          token("..."),
          |var_arg| vec![var_arg],
        ),
        map(
          tuple((
            separated_list0(tuple((meta, token(","), meta)), identifier),
            opt(tuple((tuple((meta, token(","), meta)), token("...")))),
          )),
          |(arguments, var_arg)| {
            let mut arguments = arguments.to_vec();

            if let Some((_, var_arg)) = var_arg {
              arguments.push(var_arg);
            }

            arguments
          },
        ),
      )),
      pair(meta, token(")")),
    )(input)?;

    Ok((input, MacroSig { name, arguments }))
  }
}

#[derive(Debug)]
struct Assignment<'t> {
  lhs: Expr<'t>,
  rhs: &'t str,
}

impl<'t> Assignment<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    eprintln!("Assignment::parse {:?}", tokens);

    let (tokens, lhs) = Expr::parse(tokens)?;

    eprintln!("Assignment::parse lhs {:?}", lhs);

    let (tokens, _) = meta(tokens)?;
    let (tokens, _) = token("=")(tokens)?;
    let (tokens, _) = meta(tokens)?;
    let (tokens, rhs) = identifier(tokens)?;

    Ok((tokens, Self { lhs, rhs }))
  }
}

impl fmt::Display for Assignment<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} = {}", self.lhs, self.rhs)
  }
}

#[derive(Debug)]
enum Statement<'t> {
  Expr(Expr<'t>),
  Assignment(Assignment<'t>),
  DoWhile { block: Vec<Statement<'t>>, condition: Expr<'t> },
}

impl<'t> Statement<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    alt((
      // map(
      //   pair(Assignment::parse, token(";")),
      //   |(assignment, _)| Self::Assignment(assignment),
      // ),
      map(
        pair(Expr::parse, token(";")),
        |(assignment, _)| Self::Expr(assignment),
      ),
    ))(tokens)
  }
}

impl fmt::Display for Statement<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Expr(expr) => expr.fmt(f),
      Self::Assignment(a) => a.fmt(f),
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

#[derive(Debug)]
enum MacroBody<'t> {
  Block(Vec<Statement<'t>>),
  Expr(Expr<'t>),
}

fn comment<'i, 't>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], &'t str> {
  if let Some(token) = tokens.get(0) {
    if token.starts_with("/*") && token.ends_with("*/") {
      return Ok((&tokens[1..], token))
    }
  }

  Err(nom::Err::Error(nom::error::Error::new(tokens, nom::error::ErrorKind::Fail)))
}

fn meta<'i, 't>(input: &'i [&'t str]) -> IResult<&'i [&'t str], Vec<&'t str>> {
  many0(comment)(input)
}

fn token<'i, 't>(token: &'static str) -> impl Fn(&'i [&'t str]) -> IResult<&'i [&'t str], &'t str>
where
  't: 'i,
{
  move |tokens: &[&str]| {
    if let Some(token2) = tokens.get(0) {
      if token2 == &token {
        return Ok((&tokens[1..], token2))
      }
    }

    Err(nom::Err::Error(nom::error::Error::new(tokens, nom::error::ErrorKind::Fail)))
  }
}

impl<'t> MacroBody<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, _) = meta(tokens)?;

    if tokens.is_empty() {
      return Ok((tokens, MacroBody::Block(vec![])))
    }

    if let Ok((tokens, do_while)) = token("do")(tokens) {
      let (tokens, block) = Self::parse_block(tokens)?;
      let (tokens, _) = token("while")(tokens)?;
      let (tokens, condition) = delimited(token("("), Expr::parse, token(")"))(tokens)?;

      return Ok((tokens, MacroBody::Block(vec![Statement::DoWhile { block, condition }])))
    }

    if let Ok((tokens, block)) = Self::parse_block(tokens) {
      return Ok((tokens, MacroBody::Block(block)))
    }

    if let Ok((tokens, stmt)) = Statement::parse(tokens) {
      return Ok((tokens, MacroBody::Block(vec![stmt])))
    }

    let (tokens, expr) = Expr::parse(tokens)?;
    Ok((tokens, MacroBody::Expr(expr)))
  }

  pub fn parse_block<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Vec<Statement<'t>>> {
    map(
      delimited(token("{"), pair(meta, many0(pair(Statement::parse, meta))), token("}")),
      |(_, statements)| {
        statements.into_iter().map(|(statement, _)| statement).collect::<Vec<_>>()
      }
    )(tokens)
  }
}

#[derive(Debug, Clone)]
struct FunctionCall<'t> {
  name: Identifier<'t>,
  args: Vec<Expr<'t>>,
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

fn variable_type(macro_name: &str, variable_name: &str) -> Option<&'static str> {
  Some(match variable_name {
    "pxHigherPriorityTaskWoken" | "pxYieldPending" => "*mut BaseType_t",
    "pxPreviousWakeTime" => "*mut UBaseType_t",
    "uxQueueLength" | "uxItemSize" | "uxMaxCount" | "uxInitialCount" |
    "uxTopPriority" | "uxPriority" | "uxReadyPriorities" |
    "uxIndexToNotify" | "uxIndexToWaitOn" | "uxIndexToClear" => "UBaseType_t",
    "pvItemToQueue" => "*const ::core::ffi::c_void",
    "pvParameters" | "pvBlockToFree" => "*mut ::core::ffi::c_void",
    "pcName" => "*const ::core::ffi::c_char",
    "xMutex" | "xQueue" => "QueueHandle_t",
    "xSemaphore" => "SemaphoreHandle_t",
    "xBlockTime" | "xTicksToWait" | "xNewPeriod" | "xExpectedIdleTime" | "xTimeIncrement" => "TickType_t",
    "xTask" | "xTaskToNotify" => "TaskHandle_t",
    "pxCreatedTask" => "*mut TaskHandle_t",
    "pvTaskCode" => "TaskFunction_t",
    "xTimer" => "TimerHandle_t",
    "eAction" => "eNotifyAction",
    "ulValue" | "ulSecureStackSize" | "ulBitsToClearOnEntry" |
    "ulBitsToClearOnExit" | "ulBitsToClear" => "u32",
    "usStackDepth" => "u16",
    "pulPreviousNotificationValue" | "pulPreviousNotifyValue" | "pulNotificationValue" => "*mut u32",
    "pvTaskToDelete" | "pvBuffer" => "*mut ::core::ffi::c_void",
    "pucQueueStorage" => "*mut u8",
    "pxQueueBuffer" => "*mut StaticQueue_t",
    "pxPendYield" => "*mut BaseType_t",
    "pxSemaphoreBuffer" | "pxMutexBuffer" | "pxStaticSemaphore" => "*mut StaticSemaphore_t",
    "x" if macro_name.ends_with("_CRITICAL_FROM_ISR") => "UBaseType_t",
    "x" if macro_name.ends_with("CLEAR_INTERRUPT_MASK_FROM_ISR") => "UBaseType_t",
    "x" if macro_name.ends_with("YIELD_FROM_ISR") => "BaseType_t",
    "x" if macro_name == "xTaskCreateRestricted" => "*mut TaskParameters_t",
    "xClearCountOnExit" => "BaseType_t",
    _ => return None,
  })
}

fn return_type(macro_name: &str) -> Option<&'static str> {
  if macro_name.contains("GetMutexHolder") {
    return Some("TaskHandle_t")
  }

  if macro_name.starts_with("portGET_RUN_TIME_COUNTER_VALUE") {
    return Some("::core::ffi::c_ulong")
  }

  if macro_name.starts_with("port") && macro_name.ends_with("_PRIORITY") {
    return Some("UBaseType_t")
  }

  if macro_name.starts_with("xSemaphoreCreate") {
    return Some("SemaphoreHandle_t")
  }

  if macro_name.starts_with("xQueueCreate") {
    return Some("QueueHandle_t")
  }

  if macro_name.starts_with("ul") {
    return Some("u32")
  }

  if macro_name.starts_with("x") {
    return Some("BaseType_t")
  }

  if macro_name.starts_with("ux") {
    return Some("UBaseType_t")
  }

  None
}

mod func_macro;
use func_macro::*;

impl<'t> FunctionCall<'t> {
  pub fn parse<'i>(tokens: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (tokens, name) = Identifier::parse(tokens)?;

    if matches!(name, Identifier::Literal("__asm")) {
      if let Ok((tokens, volatile)) = token("volatile")(tokens) {
        let name = Identifier::Literal("::core::arch::asm!");

        let (tokens, (template, outputs, inputs, clobbers)) = delimited(
          pair(token("("), meta),
          tuple((
            separated_list0(tuple((meta, token(","), meta)), Expr::parse),
            opt(preceded(token(":"), separated_list0(tuple((meta, token(","), meta)), Expr::parse))),
            opt(preceded(token(":"), separated_list0(tuple((meta, token(","), meta)), Expr::parse))),
            opt(preceded(token(":"), separated_list0(tuple((meta, token(","), meta)), Expr::parse))),
          )),
          pair(meta, token(")")),
        )(tokens)?;

        let mut args = template;

        if let Some(outputs) = outputs {
          args.extend(outputs);
        }

        if let Some(inputs) = inputs {
          args.extend(inputs);
        }

        if let Some(clobbers) = clobbers {
          args.extend(clobbers.into_iter().filter_map(|c| match c {
            Expr::Literal(s) if s == r#""memory""# => None,
            clobber => Some(Expr::Literal(format!("out({}) _", clobber))),
          }));
        }

        args.push(Expr::Literal(r#"clobber_abi("C")"#.to_owned()));
        args.push(Expr::Literal("options(raw)".to_owned()));

        return Ok((tokens, Self { name, args }))
      }
    }

    let (tokens, args) = delimited(
      pair(token("("), meta),
      separated_list0(tuple((meta, token(","), meta)), Expr::parse),
      pair(meta, token(")")),
    )(tokens)?;

    Ok((tokens, FunctionCall { name, args }))
  }
}

#[derive(Debug)]
struct Callbacks {
  function_macros: Arc<Mutex<Vec<String>>>,
}

fn tokenize_name(input: &[u8]) -> Vec<&str> {
  let mut tokens = vec![];

  let n = input.len();
  let mut i = 0;

  'outer: loop {
    match input.get(i) {
      Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') => {
        let start = i;
        i += 1;

        while let Some(b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'0'..=b'9') = input.get(i) {
          i += 1;
        }

        tokens.push(unsafe { str::from_utf8_unchecked(&input[start..i]) });
      },
      Some(b'(' | b')' | b',') => {
        tokens.push(unsafe { str::from_utf8_unchecked(&input[i..(i + 1)]) });
        i += 1;
      },
      Some(b'/') if matches!(input.get(i + 1), Some(b'*')) => {
        let start = i;
        i += 2;

        while let Some(c) = input.get(i) {
          i += 1;

          if *c == b'*' {
            if let Some(b'/') = input.get(i) {
              i += 1;
              tokens.push(unsafe { str::from_utf8_unchecked(&input[start..i]) });
              break;
            }
          }
        }
      },
      Some(b'.') if matches!(input.get(i..(i + 3)), Some(b"...")) => {
        tokens.push(unsafe { str::from_utf8_unchecked(&input[i..(i + 3)]) });
        i += 3;
      },
      Some(b' ') => {
        i += 1;
      },
      Some(c) => unreachable!("{}", *c as char),
      None => break,
    }
  }

  tokens
}

impl ParseCallbacks for Callbacks {
  fn item_name(&self, name: &str) -> Option<String> {
    Some(match name {
      "pcTaskGetTaskName" => "pcTaskGetName",
      "pcTimerGetTimerName" => "pcTimerGetName",
      "xTimerGetPeriod" => {
        println!(r#"cargo:rustc-cfg=freertos_feature="timer_get_period""#);
        return None
      }
      _ => return None
    }.to_owned())
  }

  fn int_macro(&self, name: &str, value: i64) -> Option<IntKind> {
    if name == "configSUPPORT_STATIC_ALLOCATION" && value != 0 {
      println!(r#"cargo:rustc-cfg=freertos_feature="static_allocation""#);
    }

    match name {
      "configMAX_PRIORITIES" => Some(IntKind::U8),
      "configMINIMAL_STACK_SIZE" | "configTIMER_TASK_STACK_DEPTH" => Some(IntKind::U16),
      _ => None,
    }
  }

  fn func_macro(&self, name: &str, value: &[&[u8]]) {
    use std::fmt::Write;

    dbg!(&name);

    let name = tokenize_name(name.as_bytes());

    let value = value.iter().map(|bytes| str::from_utf8(bytes).unwrap()).collect::<Vec<_>>();
    dbg!(&value);

    eprintln!("{:?} -> {:?}", name, value);

    let (_, macro_sig) = MacroSig::parse(&name).unwrap();
    let (_, macro_body) = MacroBody::parse(&value).unwrap();

    let name = macro_sig.name;

    if name.starts_with("_") ||
      name == "offsetof" ||
      name.starts_with("INT") ||
      name.starts_with("UINT") ||
      name.starts_with("list") ||
      name.starts_with("trace") ||
      name.starts_with("config") ||
      name == "taskYIELD" || name == "portYIELD" ||
      name.ends_with("YIELD_FROM_ISR") ||
      name.ends_with("_CRITICAL_FROM_ISR") ||
      name.ends_with("DISABLE_INTERRUPTS") ||
      name.ends_with("ENABLE_INTERRUPTS") ||
      name.ends_with("END_SWITCHING_ISR") ||
      name.ends_with("INTERRUPT_MASK_FROM_ISR") ||
      name.starts_with("configAssert") ||
      name.starts_with("portTASK_FUNCTION") ||
      name.ends_with("_TCB") ||
      name == "CAST_USER_ADDR_T" ||
      name == "vSemaphoreCreateBinary" {
      return;
    }

    eprintln!("FUNC MACRO: {:?} -> {:?}", macro_sig, macro_body);

    let mut f = String::new();

    writeln!(f, "#[allow(non_snake_case)]").unwrap();
    writeln!(f, "#[inline(always)]").unwrap();
    write!(f, r#"pub unsafe extern "C" fn {}("#, name).unwrap();
    for (i, arg) in macro_sig.arguments.iter().enumerate() {
      if i > 0 {
        write!(f, ", ").unwrap();
      }

      let ty = variable_type(&name, &arg);
      write!(f, "{}: {}", arg, ty.unwrap_or("UNKNOWN")).unwrap();
    }

    write!(f, ") ").unwrap();

    if let Some(return_type) = return_type(&name) {
      write!(f, "-> {} ", return_type).unwrap();
    }

    writeln!(f, "{{").unwrap();

    match macro_body {
      MacroBody::Block(statements) => {
        for (i, stmt) in statements.iter().enumerate() {
          if i > 0 {
            writeln!(f, ";").unwrap();
          }

          write!(f, "  {}", stmt).unwrap();
        }
      },
      MacroBody::Expr(expr) => {
        write!(f, "{}", expr).unwrap();
      }
    }

    writeln!(f).unwrap();

    write!(f, "}}").unwrap();

    self.function_macros.lock().unwrap().push(f);
  }
}

// See: https://doc.rust-lang.org/cargo/reference/build-scripts.html
fn main() {
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=src/freertos/shim.c");

  let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
  let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
  let shim_dir = manifest_dir.join("src/freertos");
  println!("cargo:SHIM={}", shim_dir.display());

  let constants = out_dir.join("constants.h");
  let mut f = File::create(&constants).unwrap();
  constants::write_to_file(&mut f).unwrap();

  let freertos_source = if let Ok(freertos_source) = env::var("FREERTOS_SRC") {
    PathBuf::from(freertos_source)
  } else {
    println!("cargo:warning=FREERTOS_SRC is not set");
    return
  };


  let freertos_config = if let Ok(freertos_config) = env::var("FREERTOS_CONFIG") {
    PathBuf::from(freertos_config)
  } else {
    File::create(out_dir.join("FreeRTOSConfig.h")).unwrap();
    out_dir.clone()
  };

  let (mut cc, bindgen) = build::builders(freertos_source, freertos_config);

  cc.file(shim_dir.join("shim.c"));

  if let Err(err) = cc.try_compile("freertos") {
    eprintln!("Compilation failed: {}", err);
    exit(1);
  }

  let function_macros = Arc::new(Mutex::new(vec![]));

  let bindings = out_dir.join("shim.rs");

  bindgen
    .header(shim_dir.join("shim.c").display().to_string())
    .header(constants.display().to_string())
    .generate_comments(false)
    .parse_callbacks(Box::new(Callbacks {
      function_macros: function_macros.clone(),
    }))
    .generate().unwrap_or_else(|err| {
      eprintln!("Failed generating bindings: {}", err);
      exit(1);
    })
    .write_to_file(&bindings).unwrap_or_else(|err| {
      eprintln!("Failed writing bindings: {}", err);
      exit(1);
    });

  let function_macros = function_macros.lock().unwrap().join("\n\n");

  let mut f = fs::OpenOptions::new()
    .write(true)
    .append(true)
    .open(&bindings)
    .unwrap();

    panic!();

  f.write_all(function_macros.as_bytes()).unwrap()
}

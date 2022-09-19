use std::fmt::Write;
use std::collections::HashMap;
use std::cell::Cell;

use quote::quote;
use proc_macro2::{TokenStream, Ident, Span};
use syn::{Type, TypeInfer, token::Underscore};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroArgType {
  /// `ident` type
  Ident,
  /// `expr` type
  Expr,
  Unknown,
}

#[derive(Debug)]
pub struct Context<'s, 't> {
  pub args: HashMap<&'s str, Cell<MacroArgType>>,
  functions: Vec<&'t str>,
}

impl<'s, 't> Context<'s, 't> {
  pub fn is_macro_arg(&self, name: &str) -> bool {
    self.args.get(name).map(|ty| ty.get() != MacroArgType::Unknown).unwrap_or(false)
  }
}

#[derive(Debug)]
pub struct FnMacro<'t> {
  pub name: &'t str,
  pub args: Vec<(&'t str, MacroArgType)>,
  pub body: MacroBody<'t>,
}

impl<'t> FnMacro<'t> {
  pub fn parse<'i>(sig: &'t str, body: &'i [&'t str]) -> Result<Self, ()> {
    let sig = tokenize_name(sig.as_bytes());

    let (_, sig) = MacroSig::parse(&sig).unwrap();

    let mut args = HashMap::new();
    for &arg in &sig.arguments {
      args.insert(arg, Cell::new(MacroArgType::Unknown));
    }

    let mut ctx = Context { args, functions: vec![] };
    let (_, body) = MacroBody::parse(&ctx, &body).unwrap();

    let args = sig.arguments.into_iter().map(|a| (a, ctx.args.remove(a).unwrap().into_inner())).collect();

    Ok(Self { name: sig.name, args, body })
  }

  pub fn write(&self, f: &mut String) -> fmt::Result {
    let generate_macro = !self.args.iter().all(|&(_, ty)| ty == MacroArgType::Unknown);

    let mut args = HashMap::new();
    for &(arg, ty) in &self.args {
      args.insert(arg, Cell::new(ty));
    }
    let mut ctx = Context { args, functions: vec![] };

    let name = Ident::new(self.name, Span::call_site());

    let mut body = TokenStream::new();
    match &self.body {
      MacroBody::Block(stmt) => stmt.to_tokens(&mut ctx, &mut body),
      MacroBody::Expr(expr) => expr.to_tokens(&mut ctx, &mut body),
    }

    if generate_macro {
      let args = self.args.iter().map(|&(arg, ty)| {
        let id = Ident::new(arg, Span::call_site());

        if ty == MacroArgType::Ident {
          quote! { $#id:ident }
        } else {
          quote! { $#id:expr }
        }
      }).collect::<Vec<_>>();

      write!(f, "{}", quote! {
        macro_rules! #name {
          (#(#args),*) => {
            #body
          };
        }
      })
    } else {
      let args = self.args.iter().map(|&(arg, ty)| {
        let id = Ident::new(arg, Span::call_site());
        if let Some(ty) = variable_type(self.name, arg) {
          let ty = Type::Verbatim(ty.parse::<TokenStream>().unwrap());
          quote! { #id: #ty }
        } else {
          quote! { #id: _ }
        }
      }).collect::<Vec<_>>();

      let return_type = return_type(&self.name).map(|ty| {
        let ty = Type::Verbatim(ty.parse::<TokenStream>().unwrap());
        quote! { -> #ty }
      });

      writeln!(f, "{}", quote! {
        #[allow(non_snake_case)]
        #[inline(always)]
        pub unsafe extern "C" fn #name(#(mut #args),*) #return_type {
          #body
        }
      })
    }
  }
}

#[derive(Debug)]
pub struct MacroSig<'t> {
  pub name: &'t str,
  pub arguments: Vec<&'t str>,
}

fn tokenize_name(input: &[u8]) -> Vec<&str> {
  let mut tokens = vec![];

  let mut i = 0;

  loop {
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

impl<'t> MacroSig<'t> {
  pub fn parse<'i>(input: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (input, name) = identifier(input)?;

    let (input, arguments) = terminated(
      delimited(
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
      ),
      eof,
    )(input)?;
    assert!(input.is_empty());

    Ok((input, MacroSig { name, arguments }))
  }
}

#[derive(Debug)]
pub enum MacroBody<'t> {
  Block(Statement<'t>),
  Expr(Expr<'t>),
}

impl<'t> MacroBody<'t> {
  pub fn parse<'i>(ctx: &Context<'_, '_>, input: &'i [&'t str]) -> IResult<&'i [&'t str], Self> {
    let (input, _) = meta(input)?;

    if input.is_empty() {
      return Ok((input, MacroBody::Block(Statement::Block(vec![]))))
    }

    let (input, body) = terminated(
      alt((
        map(|tokens| Statement::parse(ctx, tokens), MacroBody::Block),
        map(|tokens| Expr::parse(ctx, tokens), MacroBody::Expr),
      )),
      eof,
    )(input)?;
    assert!(input.is_empty());

    Ok((input, body))
  }
}

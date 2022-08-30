use std::env;
use std::fmt;
use std::io::Write;
use std::str;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Mutex, Arc};

use bindgen::callbacks::ParseCallbacks;

#[derive(Debug)]
struct Arg {
  name: String,
  cast: Option<String>,
}

impl fmt::Display for Arg {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.name == "NULL" {
      write!(f, "::core::ptr::null_mut()")?;
    } else if self.name == "0U" {
      write!(f, "0")?;
    } else if self.name == "eIncrement" {
      write!(f, "eNotifyAction_eIncrement")?;
    } else {
      write!(f, "{}", self.name)?;
    }

    if let Some(cast) = &self.cast {
      write!(f, " as {}", cast)?;
    }

    Ok(())
  }
}

#[derive(Debug)]
struct MacroSig {
  name: String,
  arguments: Vec<String>,
}

impl MacroSig {
  pub fn parse(s: &str) -> Option<Self> {
    let (name, s) = parse_ident(s)?;

    let mut s = parse_char(s, '(')?;

    let mut args = vec![];

    if let Some(s2) = parse_char(s, ')') {
      s = s2;
    } else {
      loop {
        let (arg, s2) = parse_ident(s)?;
        args.push(arg);
        s = s2;

        if let Some(s2) = parse_char(s, ',') {
          s = skip_meta(s2);
          continue
        }

        s = skip_meta(s);
        s = parse_char(s, ')')?;
          break
      }
    }

    s = skip_meta(s);
    parse_end(s)?;

    Some(MacroSig { name, arguments: args })
  }
}

#[derive(Debug)]
struct Assignment {
  lhs: String,
  rhs: String,
}

impl Assignment {
  pub fn parse(s: &str) -> Option<(Self, &str)> {
    let s = skip_meta(s);

    let (ident, s) = parse_ident(s)?;
    let s = skip_meta(s);
    let s = parse_char(s, '=')?;
    let s = skip_meta(s);
    let (expr, s) = parse_ident(s)?;

    Some((Self { lhs: ident, rhs: expr }, s))
  }
}

impl fmt::Display for Assignment {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} = {}", self.lhs, self.rhs)
  }
}

#[derive(Debug)]
enum Statement {
  FunctionCall(FunctionCall),
  Assignment(Assignment),
}

impl Statement {
  pub fn parse(s: &str) -> Option<(Self, &str)> {
    if let Some((a, s)) = Assignment::parse(s) {
      return Some((Self::Assignment(a), s))
    }

    if let Some((call, s)) = FunctionCall::parse(s) {
      return Some((Self::FunctionCall(call), s))
    }

    None
  }
}

impl fmt::Display for Statement {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::FunctionCall(call) => call.fmt(f),
      Self::Assignment(a) => a.fmt(f),
    }
  }
}

#[derive(Debug)]
struct MacroBody {
  statements: Vec<Statement>,
}

impl MacroBody {
  pub fn parse(s: &str) -> Option<(MacroBody, &str)> {
    let s = skip_meta(s);

    if s.is_empty() {
      return Some((MacroBody { statements: vec![] }, s))
    }

    if let Some((block, s)) = Self::parse_block(s) {
      return Some((MacroBody { statements: block }, s))
    }

    if let Some((stmt, s)) = Statement::parse(s) {
      return Some((MacroBody { statements: vec![stmt] }, s))
    }

    None
  }

  pub fn parse_block(s: &str) -> Option<(Vec<Statement>, &str)> {
    let s = parse_char(s, '{')?;
    let mut s = skip_meta(s);

    let mut stmts = vec![];

    if let Some(s2) = parse_char(s, '}') {
      s = s2;
    } else {
      loop {
        let (stmt, s2) = Statement::parse(s)?;
        stmts.push(stmt);
        s = s2;
        s = skip_meta(s);
        s = parse_char(s, ';')?;
        s = skip_meta(s);
        if let Some(s2) = parse_char(s, '}') {
          s = s2;
          break
        }
      }
    }

    Some((stmts, s))
  }
}

#[derive(Debug)]
struct FunctionCall {
  name: String,
  arguments: Vec<Arg>,
}

impl fmt::Display for FunctionCall {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.name == "__asmvolatile" {
      write!(f, "::core::arch::asm!")?;
    } else {
      write!(f, "{}", self.name)?;
    }

    write!(f, "(")?;
    for (i, arg) in self.arguments.iter().enumerate() {
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
    "uxQueueLength" | "uxItemSize" | "uxMaxCount" | "uxInitialCount" |
    "uxTopPriority" | "uxPriority" | "uxReadyPriorities" => "UBaseType_t",
    "pvItemToQueue" => "*const ::cty::c_void",
    "xMutex" | "xQueue" => "QueueHandle_t",
    "xSemaphore" => "SemaphoreHandle_t",
    "xBlockTime" | "xTicksToWait" | "xNewPeriod" | "xExpectedIdleTime" => "TickType_t",
    "xTaskToNotify" => "TaskHandle_t",
    "xTimer" => "TimerHandle_t",
    "eAction" => "eNotifyAction",
    "ulValue" => "u32",
    "pulPreviousNotificationValue" | "pulPreviousNotifyValue" => "*mut u32",
    "pvTaskToDelete" | "pvBuffer" => "*mut ::cty::c_void",
    "pxTCB" => "*mut TCB_t",
    "x" if macro_name.ends_with("_CRITICAL_FROM_ISR") => "UBaseType_t",
    "x" if macro_name.ends_with("CLEAR_INTERRUPT_MASK_FROM_ISR") => "UBaseType_t",
    "x" if macro_name.ends_with("YIELD_FROM_ISR") => "BaseType_t",
    _ => return None,
  })
}

fn return_type(macro_name: &str) -> Option<&'static str> {
  if macro_name.ends_with("MutexHolder") {
    return Some("TaskHandle_t")
  }

  if macro_name.starts_with("xSemaphoreCreate") {
    return Some("SemaphoreHandle_t")
  }

  if macro_name.starts_with("xQueueCreate") {
    return Some("QueueHandle_t")
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

impl FunctionCall {
  fn parse_args(s: &str) -> Option<(Vec<Arg>, &str)> {
    let s = parse_char(s, '(')?;
    let mut s = skip_meta(s);

    let mut args = vec![];

    if let Some(s2) = parse_char(s, ')') {
      s = s2;
    } else {
      loop {
        let (arg, s2) = Self::parse_arg(s)?;
        args.push(arg);
        s = s2;

        if let Some(s2) = parse_char(s, ',') {
          s = skip_meta(s2);
          continue
        }

        s = parse_char(s, ')')?;
        break
      }
    }

    Some((args, s))
  }

  fn parse_arg(s: &str) -> Option<(Arg, &str)> {
    // Parse argument with cast or parentheses.
    if let Some(s) = parse_char(s, '(') {
      let s = skip_meta(s);

      if let Some((ty, s)) = parse_ident(s) {
        let s = skip_meta(s);

        if let Some(s) = parse_char(s, ')') {
          let s = skip_meta(s);

          if let Some((mut arg, s)) = Self::parse_arg(s) {
            arg.cast = Some(ty);
            return Some((arg, s))
          }
        }
      }

      if let Some((arg, s)) = Self::parse_arg(s) {
        let s = skip_meta(s);
        let s = parse_char(s, ')')?;
        return Some((arg, s))
      }
    }

    // String.
    if let Some(s) = parse_char(s, '"') {
      if let Some(end) = s.chars().position(|c| c == '"') {
        return Some((Arg { name: format!("{:?}", &s[..end]), cast: None }, &s[(end + 1)..]))
      }
    }

    // Bare identifier.
    let (ident, s) = parse_ident(s)?;
    Some((Arg { name: ident, cast: None }, s))
  }

  fn parse(s: &str) -> Option<(Self, &str)> {
    let (name, s) = parse_ident(s)?;
    let (arguments, s) = Self::parse_args(s)?;

    Some((FunctionCall { name, arguments }, s))
  }
}

#[derive(Debug)]
struct Callbacks {
  function_macros: Arc<Mutex<Vec<String>>>,
}

impl ParseCallbacks for Callbacks {
  fn func_macro(&self, name: &str, value: &[&[u8]]) {
    use std::fmt::Write;

    let value = value.iter().map(|bytes| str::from_utf8(bytes).unwrap()).collect::<String>();

    eprintln!("{} -> {}", name, value);

    let macro_call_lhs = MacroSig::parse(name);
    let macro_call_rhs = MacroBody::parse(&value);

    eprintln!("FUNC MACRO: {:?} -> {:?}", macro_call_lhs, macro_call_rhs);

    let mut f = String::new();

    if let Some((macro_sig, macro_body)) = macro_call_lhs.zip(macro_call_rhs.map(|rhs| rhs.0)) {
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
        name.starts_with("configAssert") {
        return;
      }

      writeln!(f, "#[allow(non_snake_case)]").unwrap();
      writeln!(f, "#[inline]").unwrap();
      write!(f, r#"unsafe extern "C" fn {}("#, name).unwrap();
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

      for (i, stmt) in macro_body.statements.iter().enumerate() {
        if i > 0 {
          writeln!(f, ";").unwrap();
        }

        write!(f, "  {}", stmt).unwrap();
      }
      writeln!(f).unwrap();

      write!(f, "}}").unwrap();
    }

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

    if let Ok(freertos_source) = env::var("FREERTOS_SRC") {
      let mut heap = None;
      for i in 1..=5 {
        if env::var(&format!("CARGO_FEATURE_HEAP_{i}")).is_ok() {
          if let Some(h) = heap {
            eprintln!("Features `heap_{h}` and `heap_{i}` are mutually exclusive.");
            exit(1);
          }

          heap = Some(i);
        }
      }
      let heap = format!("heap_{}.c", heap.unwrap_or(4));

      let mut freertos_builder = freertos_cargo_build::Builder::new();
      let freertos_source = PathBuf::from(freertos_source);
      let freertos_config = match env::var("FREERTOS_CONFIG") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
          eprintln!("`FREERTOS_CONFIG` must be set to the directory containing `FreeRTOSConfig.h`.");
          exit(1);
        }
      };

      freertos_builder.get_cc().define("RUST", None);

      freertos_builder.freertos(&freertos_source);
      freertos_builder.freertos_config(&freertos_config);
      freertos_builder.freertos_shim(&shim_dir);
      freertos_builder.heap(heap);

      if let Err(err) = freertos_builder.compile() {
        eprintln!("Compilation failed: {}", err);
        exit(1);
      }

      let function_macros = Arc::new(Mutex::new(vec![]));

      let bindings = out_dir.join("shim.rs");

      bindgen::builder()
          .use_core()
          .ctypes_prefix("::cty")
          .clang_arg(format!("-I{}", freertos_source.join("include").display()))
          .clang_arg(format!("-I{}", freertos_config.display()))
          .clang_arg(format!("-I{}", freertos_builder.get_freertos_port_dir().display()))
          .header(shim_dir.join("shim.c").display().to_string())
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

      let function_macros = function_macros.lock().unwrap().join("\n");

      let mut f = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(bindings)
        .unwrap();

      f.write_all(function_macros.as_bytes()).unwrap();
    }
}

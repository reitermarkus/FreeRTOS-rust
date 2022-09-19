use std::env;
use std::fmt;
use std::io::Write;
use std::str;
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Mutex, Arc};

use bindgen::{callbacks::{ParseCallbacks, IntKind}};
use nom::multi::{separated_list0};
use nom::branch::alt;
use nom::combinator::map;
use nom::sequence::delimited;
use nom::combinator::{opt, eof};
use nom::sequence::tuple;
use nom::IResult;
use nom::multi::{many0, fold_many0};
use nom::sequence::pair;
use nom::sequence::preceded;
use nom::branch::permutation;
use nom::multi::many0_count;
use nom::sequence::terminated;

mod build;
mod constants;
mod fn_macro;
use fn_macro::*;
mod parser;
use parser::*;

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
    if let Some(token2) = tokens.get(0).as_deref() {
      let token2 = if token2.starts_with("\\\n") { // TODO: Fix in tokenizer/lexer.
        &token2[2..]
      } else {
        token2
      };

      if token2 == token {
        return Ok((&tokens[1..], token2))
      }
    }

    Err(nom::Err::Error(nom::error::Error::new(tokens, nom::error::ErrorKind::Fail)))
  }
}

pub(crate) fn variable_type(macro_name: &str, variable_name: &str) -> Option<&'static str> {
  Some(match variable_name {
    "pxListItem" => "*mut ListItem_t",
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
    "pxOwner" => "*mut ::core::ffi::c_void",
    "pxQueueBuffer" => "*mut StaticQueue_t",
    "pxPendYield" => "*mut BaseType_t",
    "pxSemaphoreBuffer" | "pxMutexBuffer" | "pxStaticSemaphore" => "*mut StaticSemaphore_t",
    "xTimeInMs" => "u32",
    "x" if macro_name.ends_with("_CRITICAL_FROM_ISR") => "UBaseType_t",
    "x" if macro_name.ends_with("CLEAR_INTERRUPT_MASK_FROM_ISR") => "UBaseType_t",
    "x" if macro_name.ends_with("YIELD_FROM_ISR") => "BaseType_t",
    "x" if macro_name == "xTaskCreateRestricted" => "*mut TaskParameters_t",
    "xClearCountOnExit" => "BaseType_t",
    _ => return None,
  })
}

pub(crate) fn return_type(macro_name: &str) -> Option<&'static str> {
  if macro_name.starts_with("pdMS_TO_TICKS") {
    return Some("TickType_t")
  }

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

#[derive(Debug)]
struct Callbacks {
  function_macros: Arc<Mutex<Vec<String>>>,
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
    let sig = name;
    let body = value.iter().map(|bytes| str::from_utf8(bytes).unwrap()).collect::<Vec<_>>();

    dbg!(&name);
    dbg!(&body);

    let name = name.split_once("(").map(|(n, _)| n).unwrap_or(name);
    if name.starts_with("_") ||
      name == "offsetof" ||
      ((name.starts_with("INT") || name.starts_with("UINT")) && name.ends_with("_C")) ||
      // name.starts_with("list") ||
      // name.starts_with("trace") ||
      name.starts_with("config") ||
      // name == "taskYIELD" || name == "portYIELD" ||
      name.ends_with("YIELD_FROM_ISR") ||
      name.ends_with("_CRITICAL_FROM_ISR") ||
      name.ends_with("DISABLE_INTERRUPTS") ||
      name.ends_with("ENABLE_INTERRUPTS") ||
      name.ends_with("END_SWITCHING_ISR") ||
      name.ends_with("INTERRUPT_MASK_FROM_ISR") ||
      name.starts_with("portTASK_FUNCTION") ||
      name == "portGET_HIGHEST_PRIORITY" ||
      name == "portRECORD_READY_PRIORITY" ||
      name == "portRESET_READY_PRIORITY" ||
      name.ends_with("_TCB") ||
      name == "CAST_USER_ADDR_T" ||
      name == "vSemaphoreCreateBinary" {
      return;
    }

    let fn_macro = FnMacro::parse(sig, &body).unwrap();

    eprintln!("FUNC MACRO: {:?}({:?}) -> {:?}", fn_macro.name, fn_macro.args, fn_macro.body);

    let mut f = String::new();

    fn_macro.write(&mut f).unwrap();

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

  // panic!();

  f.write_all(function_macros.as_bytes()).unwrap()
}

use std::env;
use std::str;
use std::fs::{File};
use std::path::PathBuf;
use std::process::exit;

use bindgen::{callbacks::{ParseCallbacks, IntKind}};

mod build;

#[derive(Debug)]
struct Callbacks;

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
    if name == "configSUPPORT_DYNAMIC_ALLOCATION" && value != 0 {
      println!(r#"cargo:rustc-cfg=freertos_feature="dynamic_allocation""#);
    }

    if name == "configSUPPORT_STATIC_ALLOCATION" && value != 0 {
      println!(r#"cargo:rustc-cfg=freertos_feature="static_allocation""#);
    }

    match name {
      "configMAX_PRIORITIES" => Some(IntKind::U8),
      "configTIMER_TASK_STACK_DEPTH" => Some(IntKind::U16),
      _ => None,
    }
  }

  fn fn_macro_arg_type(&self, name: &str, arg: &str) -> Option<syn::Type> {
    let ty = match arg {
      "pxList" => syn::parse_quote! { *mut List_t },
      "pxListItem" | "pxItem" => syn::parse_quote! { *mut ListItem_t },
      // "pxHigherPriorityTaskWoken" | "pxYieldPending" => "*mut BaseType_t",
      // "pxPreviousWakeTime" => "*mut UBaseType_t",
      // "uxQueueLength" | "uxItemSize" | "uxMaxCount" | "uxInitialCount" |
      // "uxTopPriority" | "uxPriority" | "uxReadyPriorities" |
      // "uxIndexToNotify" | "uxIndexToWaitOn" | "uxIndexToClear" => "UBaseType_t",
      // "pvItemToQueue" => "*const ::core::ffi::c_void",
      // "pvParameters" | "pvBlockToFree" => "*mut ::core::ffi::c_void",
      // "pcName" => "*const ::core::ffi::c_char",
      "uxPriority" | "uxTopPriority" | "uxReadyPriorities" => syn::parse_quote! { UBaseType_t },
      "xQueue" => syn::parse_quote! { QueueHandle_t },
      "xMutex" | "xSemaphore" => syn::parse_quote! { SemaphoreHandle_t },
      // "xBlockTime" | "xTicksToWait" | "xNewPeriod" | "xExpectedIdleTime" | "xTimeIncrement" => "TickType_t",
      // "xTask" | "xTaskToNotify" => "TaskHandle_t",
      // "pxCreatedTask" => "*mut TaskHandle_t",
      // "pvTaskCode" => "TaskFunction_t",
      // "xTimer" => "TimerHandle_t",
      // "eAction" => "eNotifyAction",
      // "ulValue" | "ulSecureStackSize" | "ulBitsToClearOnEntry" |
      // "ulBitsToClearOnExit" | "ulBitsToClear" => "u32",
      // "usStackDepth" => "u16",
      // "pulPreviousNotificationValue" | "pulPreviousNotifyValue" | "pulNotificationValue" => "*mut u32",
      // "pvTaskToDelete" | "pvBuffer" => "*mut ::core::ffi::c_void",
      // "pucQueueStorage" => "*mut u8",
      "pxOwner" => syn::parse_quote! { *mut ::core::ffi::c_void },
      // "pxQueueBuffer" => "*mut StaticQueue_t",
      // "pxPendYield" => "*mut BaseType_t",
      // "pxSemaphoreBuffer" | "pxMutexBuffer" | "pxStaticSemaphore" => "*mut StaticSemaphore_t",
      // "xTimeInMs" => "u32",
      // "x" if name.ends_with("ENTER_CRITICAL_FROM_ISR") => "::core::ffi::c_long",
      // "x" if name.ends_with("CLEAR_INTERRUPT_MASK_FROM_ISR") => "::core::ffi::c_long",
      // "x" if name.ends_with("YIELD_FROM_ISR") => "BaseType_t",
      // "x" if name == "xTaskCreateRestricted" => "*mut TaskParameters_t",
      // "xClearCountOnExit" | "xSwitchRequired" => "BaseType_t",
      "xValue" if name == "listSET_LIST_ITEM_VALUE" => syn::parse_quote! { TickType_t },
      _ => return None,
    };

    Some(ty)
  }
}

// See: https://doc.rust-lang.org/cargo/reference/build-scripts.html
fn main() {
  println!("cargo:rerun-if-changed=src/freertos/shim.c");

  let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
  let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
  let shim_dir = manifest_dir.join("src/freertos");
  println!("cargo:SHIM={}", shim_dir.display());

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

  let bindings = out_dir.join("shim.rs");

  bindgen
    .header(shim_dir.join("shim.c").display().to_string())
    .generate_comments(false)
    .parse_callbacks(Box::new(Callbacks))
    .blocklist_function("__.*")
    .blocklist_function("U?INT(MAX|\\d+)_C")
    .blocklist_function("task(ENTER|EXIT)_CRITICAL_FROM_ISR")
    .blocklist_function("task(ENABLE|DISABLE)_INTERRUPTS")
    .blocklist_function("port(SET|CLEAR)_INTERRUPT_MASK_FROM_ISR")
    .blocklist_function("port(ENABLE|DISABLE)_INTERRUPTS")
    // Trace macros only work if defined in C.
    .blocklist_function("trace[A-Z_]+")
    .generate().unwrap_or_else(|err| {
      eprintln!("Failed generating bindings: {}", err);
      exit(1);
    })
    .write_to_file(&bindings).unwrap_or_else(|err| {
      eprintln!("Failed writing bindings: {}", err);
      exit(1);
    });
}

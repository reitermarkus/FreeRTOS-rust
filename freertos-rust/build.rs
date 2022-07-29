use std::env;
use std::path::PathBuf;
use std::process::exit;

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

      bindgen::builder()
          .use_core()
          .ctypes_prefix("::cty")
          .clang_arg(format!("-I{}", freertos_source.join("include").display()))
          .clang_arg(format!("-I{}", freertos_config.display()))
          .clang_arg(format!("-I{}", freertos_builder.get_freertos_port_dir().display()))
          .header(shim_dir.join("shim.c").display().to_string())
          .generate().unwrap_or_else(|err| {
            eprintln!("Failed generating bindings: {}", err);
            exit(1);
          })
          .write_to_file(out_dir.join("shim.rs")).unwrap_or_else(|err| {
            eprintln!("Failed writing bindings: {}", err);
            exit(1);
          });
    }
}

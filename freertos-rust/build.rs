use std::env;
use std::path::PathBuf;

// See: https://doc.rust-lang.org/cargo/reference/build-scripts.html
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/freertos/shim.c");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let shim_dir = manifest_dir.join("src/freertos");
    println!("cargo:SHIM={}", shim_dir.display());

    if let Ok(freertos_source) = env::var("FREERTOS_SRC") {
      let heap = if env::var("CARGO_FEATURE_HEAP_4").is_ok() {
        Some("heap_4.c")
      } else if env::var("CARGO_FEATURE_HEAP_3").is_ok() {
        Some("heap_3.c")
      } else if env::var("CARGO_FEATURE_HEAP_2").is_ok() {
        Some("heap_2.c")
      } else if  env::var("CARGO_FEATURE_HEAP_1").is_ok() {
        Some("heap_1.c")
      } else {
        None
      };

      let mut freertos_builder = freertos_cargo_build::Builder::new();
      let pwd = PathBuf::from(env::var("PWD").unwrap());
      let freertos_source = pwd.join(freertos_source);
      let freertos_config = pwd.join(env::var("FREERTOS_CONFIG").unwrap());

      freertos_builder.get_cc().define("RUST", None);

      freertos_builder.freertos(&freertos_source);
      freertos_builder.freertos_config(&freertos_config);
      freertos_builder.freertos_shim(&shim_dir);

      if let Some(heap) = heap {
        freertos_builder.heap(heap.to_owned());
      }

      freertos_builder.compile().unwrap_or_else(|e| { panic!("{}", e.to_string()) });

      bindgen::builder()
          .use_core()
          .ctypes_prefix("::cty")
          .clang_arg(format!("-I{}", freertos_source.join("include").display()))
          .clang_arg(format!("-I{}", freertos_config.display()))
          .clang_arg(format!("-I{}", freertos_builder.get_freertos_port_dir().display()))
          .header(shim_dir.join("shim.c").display().to_string())
          .generate().unwrap()
          .write_to_file(out_dir.join("shim.rs")).unwrap();
    }
}

use std::env;
use std::path::{Path, PathBuf};
use std::process::exit;

use walkdir::WalkDir;

/// Get the heap implementation from `cargo` features.
pub fn heap() -> PathBuf {
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

  format!("heap_{}.c", heap.unwrap_or(4)).into()
}

/// Get the port directory for the target.
pub fn port() -> PathBuf {
  let target = env::var("TARGET").unwrap_or_default();
  let target_family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();

  match (target.as_str(), target_family.as_str()) {
    ("thumbv7m-none-eabi" | "thumbv7em-none-eabi", _) => Path::new("GCC").join("ARM_CM3"),
    ("thumbv7em-none-eabihf", _) => Path::new("GCC").join("ARM_CM4F"),
    ("thumbv8m.main-none-eabi", _) => Path::new("GCC").join("ARM_CM33_NTZ").join("non_secure"),
    ("thumbv8m.main-none-eabihf", _) => Path::new("GCC").join("ARM_CM33_NTZ").join("non_secure"),
    (_, "unix") => Path::new("ThirdParty").join("GCC").join("Posix"),
    (_, "windows") => PathBuf::from("MSVC-MingW"),
    _ => {
      eprintln!("Target '{}' is not supported.", target);
      exit(1);
    }
  }
}

/// Find `.c` files until the given depth.
pub fn find_c_files(dir: impl AsRef<Path>, depth: Option<usize>) -> Result<Vec<PathBuf>, std::io::Error> {
  let mut w = WalkDir::new(dir).follow_links(false);

  if let Some(depth) = depth {
    w = w.max_depth(depth);
  }

  w.into_iter()
    .map(|entry| {
      let entry = entry?;

      let f_name = entry.path();

      Ok(f_name.extension().and_then(|ext| if ext == "c" {
        Some(f_name.into())
      } else {
        None
      }))
    })
    .filter_map(|res| res.transpose())
    .collect()
}

pub fn builders(
  source: impl AsRef<Path>,
  config: impl AsRef<Path>,
) -> (cc::Build, bindgen::Builder) {
  let source = source.as_ref();
  let config = config.as_ref();

  let include = source.join("include");
  let portable = source.join("portable");
  let port = portable.join(&port());
  let heap = portable.join("MemMang").join(&heap());

  let mut c_files = find_c_files(source, Some(1)).unwrap();
  c_files.extend(find_c_files(&port, None).unwrap());
  c_files.push(heap);

  let includes = vec![
    include,
    port,
    config.into(),
  ];

  let mut cc = cc::Build::new();
  let mut bindgen = bindgen::builder()
    .use_core()
    .ctypes_prefix("::core::ffi")
    .parse_callbacks(Box::new(bindgen::CargoCallbacks));

  cc.define("RUST", None);
  bindgen = bindgen.clang_arg(format!("-DRUST"));

  for c_file in c_files {
    cc.file(c_file);
  }

  for include in includes {
    cc.include(&include);
    bindgen = bindgen.clang_arg(format!("-I{}", include.display()));
  }

  (cc, bindgen)
}

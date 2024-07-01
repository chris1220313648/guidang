use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use executor_ops as ops;
use std::env;
use std::path::PathBuf;

fn main() {
    let snapshot_path = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("SNAPSHOT.bin");
    let mut rt = JsRuntime::new(RuntimeOptions {
        module_loader: None,
        extensions: ops::extensions(),
        startup_snapshot: None,
        will_snapshot: true,
        ..Default::default()
    });
    let snapshot = rt.snapshot();
    let snapshot_slice: &[u8] = &*snapshot;
    println!("Snapshot size: {}", snapshot_slice.len());
    std::fs::write(&snapshot_path, snapshot_slice).unwrap();
    println!("Snapshot written to: {} ", snapshot_path.display());
}

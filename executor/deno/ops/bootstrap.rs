use deno_core::{include_js_files, Extension};

pub fn init() -> Extension {
    Extension::builder()
        .js(include_js_files!(
            prefix "executor:bootstrap",
            "bootstrap/06_util.js",
            "bootstrap/99_main.js",
        ))
        .build()
}

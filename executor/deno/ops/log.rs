use std::{rc::Rc, vec};

use deno_core::{error::AnyError, include_js_files, op, Extension, OpState};

use crate::Rule;

macro_rules! event {
    ($level:expr, $($args:tt)*) => {{
        use ::tracing::Level;

        match $level {
            0 => ::tracing::event!(Level::TRACE, $($args)*),
            1 => ::tracing::event!(Level::DEBUG, $($args)*),
            2 => ::tracing::event!(Level::INFO, $($args)*),
            3 => ::tracing::event!(Level::WARN, $($args)*),
            _ => ::tracing::event!(Level::ERROR, $($args)*),
        }
    }};
}

pub fn init() -> Extension {
    Extension::builder()
        .js(include_js_files!(
            prefix "executor/deno:",
            "log/01_colors.js",
            "log/02_console.js",
        ))
        .middleware(|op| match op.name {
            "op_print" => op_log::decl(),
            _ => op,
        })
        .ops(vec![op_log::decl()])
        .build()
}

#[op]
pub fn op_log(state: &mut OpState, level: u8, msg: String) -> Result<(), AnyError> {
    let rule: &Rc<Rule> = state.borrow();
    let id: u32 = rule.script_id.into();
    event!(level, msg = %msg, script_id = %id, name = ?rule.name, version = ?rule.version, register = ?rule.register);
    Ok(())
}

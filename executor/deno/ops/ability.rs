use std::{cell::RefCell, rc::Rc};

use deno_core::{
    error::resource_unavailable, error::AnyError, include_js_files, op, Extension, OpState,
};
use reqwest::Client;

pub fn init() -> Extension {
    Extension::builder()
        .js(include_js_files!(
            prefix "executor/deno:",
            "ability/01_ability.js",
        ))
        .ops(vec![op_http_post::decl(), op_http_get::decl()])
        .build()
}

#[op]
pub async fn op_http_post(
    state: Rc<RefCell<OpState>>,
    url: String,
    body: String,
) -> Result<String, AnyError> {
    let op_state = state.try_borrow().map_err(|_| resource_unavailable())?;
    let http: &Client = op_state.borrow();
    let res = http.post(url).body(body).send().await?;
    let res = res.text().await?;
    Ok(res)
}

#[op]
pub async fn op_http_get(
    state: Rc<RefCell<OpState>>,
    url: String,
    body: String,
) -> Result<String, AnyError> {
    let op_state = state.try_borrow().map_err(|_| resource_unavailable())?;
    let http: &Client = op_state.borrow();
    let res = http.get(url).body(body).send().await?;
    let res = res.text().await?;
    Ok(res)
}

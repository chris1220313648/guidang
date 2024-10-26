use std::{cell::RefCell, rc::Rc};
//一个模块的一部分，用于处理设备的读取、写入、获取状态、
//设置状态和提交设备操作。它是为Deno环境编写的，利用Deno的底层API来与设备进行交互
use deno_core::{//导入Deno的核心库，这包括错误处理、操作注册、扩展创建等功
    error::resource_unavailable, error::AnyError, include_js_files, op, Extension, OpState,
};
use reqwest::Client;
use tracing::debug;
use std::collections::HashMap;
use deno_core::serde_json;

use crate::{ReadableAbilities, WritableAbilities}; //导入reqwest库的Client类型，用于执行HTTP请求。

pub fn init() -> Extension {//创建一个新的扩展构建器实例，通过链式调用配置扩展：
    Extension::builder()
        .js(include_js_files!(// 添加JavaScript文件，这些文件定义了扩展提供的功能在JavaScript侧的接口。
            prefix "executor/deno:",
            "ability/01_ability.js",
        ))
        .ops(vec![op_http_post::decl(), op_http_get::decl(),op_get_ability_status::decl(),op_get_ability_url::decl()])
        .build()//注册操作，每个操作通过Rust异步函数实现具体的功能，例如HTTP请求。
}

#[op]
pub async fn op_http_post(
    state: Rc<RefCell<OpState>>, // 包含 Deno 实例状态的引用计数指针
    url: String,                 // HTTP 请求的 URL
    body: String,                // 请求体
    headers: String,             // headers 参数为 JSON 字符串
) -> Result<String, AnyError> {
    let op_state = state.try_borrow().map_err(|_| resource_unavailable())?; // 借用客户端
    let http: &Client = op_state.borrow();

    // 将 headers 字符串解析为 HashMap
    let headers_map: HashMap<String, String> = serde_json::from_str(&headers)?;

    // 构建请求
    let mut request = http.post(url).body(body);

    // 设置请求头
    for (key, value) in headers_map {
        request = request.header(key, value);
    }

    let res = request.send().await?; // 发送请求
    let res = res.text().await?;     // 获取响应内容
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

#[op]
pub fn op_get_ability_status(
    state: &mut OpState,
    name: String,
    property: String,
) -> Result<Option<(String, String)>, AnyError> {
    // 特殊判断：如果输入是 "ability-target-name" 和 "router"，直接返回默认值
    if name == "ability-target-name" && property == "router" {
        return Ok(Some(("alert".to_string(), "http://192.168.1.204:5000".to_string())));
    }

    // 继续原始逻辑
    let readableability: &ReadableAbilities = state.borrow(); // 借用可读设备集合
    debug!("{:?}", readableability);
    
    // 获取 value
    let value = readableability
        .abilities
        .get(&name) // 从设备映射中找到指定名称的设备
        .and_then(|d| d.status.get(&property)) // 查询设备的状态中是否存在指定属性
        .map(|v| v.to_owned());

    // 获取 http_url
    let http_url = readableability
        .abilities
        .get(&name)
        .map(|d| d.http_url.clone());

    debug!(name = ?name, property = ?property, value = ?value, http_url = ?http_url);

    // 同时返回 value 和 http_url
    match (value, http_url) {
        (Some(v), Some(url)) => Ok(Some((v, url))), // 如果 value 和 http_url 都存在，返回元组
        _ => Ok(None), // 否则返回 None
    }
}


#[op]
pub fn op_get_ability_url(
    state: &mut OpState,
    name: String,

) -> Result<Option<String>, AnyError> {
    let writeableability: &WritableAbilities = state.borrow();
   

    // 获取 http_url
    let http_url = writeableability
    .abilities
    .get(&name)
    .map(|d| d.http_url.clone());

    debug!(name = ?name,   http_url = ?http_url);

    // 同时返回 value 和 http_url
    Ok(http_url)

}

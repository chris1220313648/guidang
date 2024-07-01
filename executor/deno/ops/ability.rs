use std::{cell::RefCell, rc::Rc};
//一个模块的一部分，用于处理设备的读取、写入、获取状态、
//设置状态和提交设备操作。它是为Deno环境编写的，利用Deno的底层API来与设备进行交互
use deno_core::{//导入Deno的核心库，这包括错误处理、操作注册、扩展创建等功
    error::resource_unavailable, error::AnyError, include_js_files, op, Extension, OpState,
};
use reqwest::Client;//导入reqwest库的Client类型，用于执行HTTP请求。

pub fn init() -> Extension {//创建一个新的扩展构建器实例，通过链式调用配置扩展：
    Extension::builder()
        .js(include_js_files!(// 添加JavaScript文件，这些文件定义了扩展提供的功能在JavaScript侧的接口。
            prefix "executor/deno:",
            "ability/01_ability.js",
        ))
        .ops(vec![op_http_post::decl(), op_http_get::decl()])
        .build()//注册操作，每个操作通过Rust异步函数实现具体的功能，例如HTTP请求。
}

#[op]//是一个属性宏，用于标记一个异步函数为Deno操作（operation）。这些操作是扩展与JavaScript代码交互的接口
pub async fn op_http_post(
    state: Rc<RefCell<OpState>>,//包含了当前Deno实例状态的引用计数指针（Rc<RefCell<OpState>>），允许操作访问全局状态。
    url: String,//: 分别表示HTTP请求的URL和正文。
    body: String,
) -> Result<String, AnyError> {
    let op_state = state.try_borrow().map_err(|_| resource_unavailable())?;//借用客户端
    let http: &Client = op_state.borrow();
    let res = http.post(url).body(body).send().await?;// 使用reqwest客户端发起一个POST请求，异步等待结果。
    let res = res.text().await?;//
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

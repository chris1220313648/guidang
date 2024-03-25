use color_eyre::Result;
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, Level};
use tracing_subscriber::{filter, prelude::*};

use axum::{//web框架
    extract::{Extension, Query},
    routing::get,
    Router,
};
use tokio::sync::Mutex;//异步运行时
//这个函数作为Axum框架中的处理函数（handler），用于处理特定的web请求
#[tracing::instrument]//从请求的查询字符串中提取参数arg。这里Arg应该是一个结构体，其中包含请求可能带有的查询参数从请求的查询字符串中提取参数arg。
async fn filter(Query(arg): Query<Arg>, Extension(state): Extension<Arc<Mutex<f32>>>) -> String {//使用Axum的Extension特性从应用的状态中提取共享状态。//Arc<Mutex<f32>>类型，表示一个被互斥锁保护的浮点数，允许跨多个请求安全地共享和修改这个值。
    let mut state = state.lock().await;//异步等待并锁定状态，以便安全更新
    *state = (*state + arg.value) / 2.0;//更新状态值为其当前值加上查询参数arg的value字段值的平均。
    info!(state =? *state);
    state.to_string()//将更新后的状态值转换为字符串并作为响应返回。这样，请求者可以看到更新后的状态值。
    //arg.value.to_string()
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct Arg {//定义一个名为Arg的结构体，用于从请求中反序列化参数。它包含一个名为value的f32类型字段。这个结构体可以自动从请求的查询字符串中解析value参数
    value: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::registry()//配置并初始化tracing日志，包括美化的日志格式和线程名称。设置默认的日志级别为DEBUG。
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_thread_names(true),
        )
        .with(filter::Targets::new().with_default(Level::DEBUG))
        .init();
    let value = Arc::new(Mutex::new(0.0f32));//创建一个初始值为0.0的浮点数，使用Arc<Mutex<>>进行封装，以实现跨异步任务的状态共享和线程安全访问。
    let app = Router::new()
        .route("/api/v1alpha1/filter", get(filter))//一个是处理/api/v1alpha1/filter路径的filter函数，
        .layer(Extension(value))
        .route("/", get(|| async { "Hello, World!" }));//另一个是根路径/，返回固定的字符串"Hello, World!"。通过.layer(Extension(value))为所有路由添加状态共享

    let addr = SocketAddr::from(([0, 0, 0, 0], 8003));//设置监听地址0.0.0.0:8003
    println!("filter webserver listening on {}", addr);
    axum::Server::bind(&addr)//使用axum::Server启动Web服务器，监听指定地址，并使用之前定义的app作为请求处理器。
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

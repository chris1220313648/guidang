//! Publib tasks for rule engine controller
//这段Rust代码展示了如何使用axum框架和tonic库来构建基于HTTP和gRPC协议的服务器，分别用于处理Web请求和gRPC调用。这种结构是现代微服务架构中常见的，能够同时支持RESTful API和高效的gRPC通信。
use axum::extract::Extension;
use color_eyre::Result;
use flume::Sender;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

use crate::{
    api::Script,
    scheduler::{Reflector, ResourceIndex},
    session::SessionManager,
    trigger,
};

#[tracing::instrument]//接收一个Extension包装的Arc<Reflector>
async fn debug(Extension(state): Extension<Arc<Reflector>>) -> String {
    let mut result = String::new();
    result.push_str(&format!("Device: {:?}\n", state.device_store));
    result.push_str(&format!("Script: {:?}\n", state.script_store));
    result.push_str(&format!("Map: {:?}\n", state.selector_map));
    result
}//这个debug函数可以作为一个HTTP处理函数，通过Web请求调用，

#[tracing::instrument(skip_all)]
pub async fn web_server(
    scheduler: Sender<ResourceIndex<Script>>,//用于发送脚本资源索引到调度器。
    store: Arc<Reflector>,
    addr: SocketAddr,//服务器监听的地址和端口
) -> Result<()> {
    use axum::{routing::get, Router};//引入了axum框架的Router类型和get函数
//创建了一个Arc（原子引用计数的智能指针）来包裹scheduler
    let endpoint = Arc::new(scheduler);

    let app = Router::new()
        .route("/api/v1alpha/webhook", get(trigger::webhook::webhook))//路由配置为处理webhook请求，具体处理逻辑由trigger::webhook::webhook函数处理。
        .layer(Extension(endpoint))
        .route("/api/v1alpha/debug", get(debug))
        .layer(Extension(store));//封装为axum的Extension，从而可以在请求处理函数中通过类型系统自动提取和访问这些共享状态。

    info!("Rule engine webserver listening on {}", addr);
    axum::Server::bind(&addr)//绑定地址和端口
        .serve(app.into_make_service())//启动服务
        .await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn grpc_server(addr: SocketAddr, mgr: SessionManager) -> Result<()> {//指定gRPC服务应监听的地址和端口
    use proto::controller_service_server::ControllerServiceServer;//管理会话或业务逻辑的实例
    use tonic::transport::Server;//tonic的服务器构建器，用于配置和启动gRPC服务器。
    info!("Rule engine grpc server listening on {}", addr);

    Server::builder()
        // .add_service(ControllerServiceServer::new(mgr))/自动生成的gRPC服务添加到服务器中
        .serve(addr)//配置服务器监听的地址（addr），并异步等待服务的启动
        .await?;

    Ok(())
}

//! Publib tasks for rule engine controller

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

#[tracing::instrument]
async fn debug(Extension(state): Extension<Arc<Reflector>>) -> String {
    let mut result = String::new();
    result.push_str(&format!("Device: {:?}\n", state.device_store));
    result.push_str(&format!("Script: {:?}\n", state.script_store));
    result.push_str(&format!("Map: {:?}\n", state.selector_map));
    result
}

#[tracing::instrument(skip_all)]
pub async fn web_server(
    scheduler: Sender<ResourceIndex<Script>>,//用于发送脚本信息
    store: Arc<Reflector>,
    addr: SocketAddr,
) -> Result<()> {
    use axum::{routing::get, Router};

    let endpoint = Arc::new(scheduler);

    let app = Router::new()
        .route("/api/v1alpha/webhook", get(trigger::webhook::webhook))
        .layer(Extension(endpoint))//通过 axum::Extension 传递给请求处理器，以便在请求生命周期内共享
        .route("/api/v1alpha/debug", get(debug))
        .layer(Extension(store));

    info!("Rule engine webserver listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())//来处理接收的HTTP请求
        .await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn grpc_server(addr: SocketAddr, mgr: SessionManager) -> Result<()> {
    use proto::controller_service_server::ControllerServiceServer;//根据 Protocol Buffers 文件自动生成的 Rust 类型，
    use tonic::transport::Server;//用于构建grpc服务器
    info!("Rule engine grpc server listening on {}", addr);

    Server::builder()
        .add_service(ControllerServiceServer::new(mgr))//将一个grpc服务（会话管理器）添加到grpc服务器中
        .serve(addr)//方法启动 gRPC 服务器并使其在指定的地址上监听传入的请求
        .await?;

    Ok(())
}

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
    scheduler: Sender<ResourceIndex<Script>>,
    store: Arc<Reflector>,
    addr: SocketAddr,
) -> Result<()> {
    use axum::{routing::get, Router};

    let endpoint = Arc::new(scheduler);

    let app = Router::new()
        .route("/api/v1alpha/webhook", get(trigger::webhook::webhook))
        .layer(Extension(endpoint))
        .route("/api/v1alpha/debug", get(debug))
        .layer(Extension(store));

    info!("Rule engine webserver listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn grpc_server(addr: SocketAddr, mgr: SessionManager) -> Result<()> {
    use proto::controller_service_server::ControllerServiceServer;
    use tonic::transport::Server;
    info!("Rule engine grpc server listening on {}", addr);

    Server::builder()
        .add_service(ControllerServiceServer::new(mgr))
        .serve(addr)
        .await?;

    Ok(())
}

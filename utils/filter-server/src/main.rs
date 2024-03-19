use color_eyre::Result;
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, Level};
use tracing_subscriber::{filter, prelude::*};

use axum::{
    extract::{Extension, Query},
    routing::get,
    Router,
};
use tokio::sync::Mutex;

#[tracing::instrument]
async fn filter(Query(arg): Query<Arg>, Extension(state): Extension<Arc<Mutex<f32>>>) -> String {
    let mut state = state.lock().await;
    *state = (*state + arg.value) / 2.0;
    info!(state =? *state);
    state.to_string()
    //arg.value.to_string()
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct Arg {
    value: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_thread_names(true),
        )
        .with(filter::Targets::new().with_default(Level::DEBUG))
        .init();
    let value = Arc::new(Mutex::new(0.0f32));
    let app = Router::new()
        .route("/api/v1alpha1/filter", get(filter))
        .layer(Extension(value))
        .route("/", get(|| async { "Hello, World!" }));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8003));
    println!("filter webserver listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

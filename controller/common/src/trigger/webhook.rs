use std::sync::Arc;

use axum::{
    extract::{Extension, Query},
    http::StatusCode,
};
use flume::Sender;

use crate::api::Script;
use crate::scheduler::ResourceIndex;

#[tracing::instrument]
pub async fn webhook(
    Query(arg): Query<ResourceIndex<Script>>,
    Extension(state): Extension<Arc<Sender<ResourceIndex<Script>>>>,
) -> StatusCode {
    if state.send(arg).is_err() {
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        StatusCode::OK
    }
}

#[cfg(test)]
mod test {
    use crate::api::Script;
    use crate::scheduler::ResourceIndex;
    use axum::{routing::get, Extension, Router};
    use std::{net::SocketAddr, str::FromStr, sync::Arc};
    use tokio::process::Command;

    #[tokio::test]
    async fn test_webhook() {
        const DEVICE_NAME: &str = "test_name";
        const DEVICE_NAMESPACE: &str = "test_namespace";

        let (tx, rx) = flume::bounded::<ResourceIndex<Script>>(3);
        tokio::spawn(async move {
            let endpoint = Arc::new(tx);

            let app = Router::new()
                .route("/api/v1alpha/webhook", get(super::webhook))
                .layer(Extension(endpoint));
            let addr = SocketAddr::from_str("127.0.0.1:10080").unwrap();
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .await
                .unwrap()
        });

        let status = Command::new("curl")
            .arg(format!("http://127.0.0.1:10080/api/v1alpha/webhook?name={DEVICE_NAME}&namespace={DEVICE_NAMESPACE}"))
            .spawn()
            .unwrap()
            .wait().await
            .unwrap();
        assert!(status.success());
        let ri = rx.recv_async().await.unwrap();
        assert_eq!(ri.name, DEVICE_NAME);
        assert_eq!(ri.namespace, DEVICE_NAMESPACE);
    }
}

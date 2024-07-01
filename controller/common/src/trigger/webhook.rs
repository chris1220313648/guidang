//这段代码演示了如何使用Rust语言和axum库创建一个简单的Webhook服务器，以及如何进行自动化测试以验证其功能。Webhook接收通过查询参数传递的资源索引（ResourceIndex<Script>），并通过一个共享状态（Sender<ResourceIndex<Script>>）异步发送这个资源索引。如果发送成功，返回HTTP状态码200（OK），否则返回500（内部服务器错误）。
use std::sync::Arc;

use axum::{
    extract::{Extension, Query},//axum 的一个提取器（extractor
    http::StatusCode,
};
use flume::Sender;

use crate::api::Script;
use crate::scheduler::ResourceIndex;
//用于接收来自外部系统的通知，然后将这些通知（封装在 ResourceIndex<Script> 中）
//发送到一个异步处理队列。通过使用 flume crate 和 Arc 进行线程安全的通信，这个函数能够在高并发环境下安全高效地工作。
#[tracing::instrument]
pub async fn webhook(//这个异步函数处理一个 HTTP 请求。它从请求中提取 ResourceIndex<Script> 实例和共享状态，然后尝试通过 flume 通道发送这个实例。
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
        tokio::spawn(async move {//创建一个新的异步任务来启动和运行 HTTP 服务器。
            let endpoint = Arc::new(tx);
//将 flume 的发送端 tx 包装在 Arc 中，使得这个发送端可以在多个异步任务之间安全共享
            let app = Router::new()
                .route("/api/v1alpha/webhook", get(super::webhook))//向路由器添加一个路由，这个路由监听 /api/v1alpha/webhook 路径的 GET 请求
                .layer(Extension(endpoint));//添加一个中间件层，通过 Extension 包装共享状态（此处是通道的发送端），使其可以在处理函数中被访问
            let addr = SocketAddr::from_str("127.0.0.1:10080").unwrap();
            axum::Server::bind(&addr)//创建一个 axum 服务器并绑定到地址
                .serve(app.into_make_service())//开始服务 HTTP 请求
                .await//等待服务器启动完成
                .unwrap()//断言服务器成功启动，否则测试将因为错误而失败
        });

        let status = Command::new("curl")
            .arg(format!("http://127.0.0.1:10080/api/v1alpha/webhook?name={DEVICE_NAME}&namespace={DEVICE_NAMESPACE}"))
            .spawn()// 启动 curl 命令，并返回一个 Child 类型的实例，代表子进程
            .unwrap()
            .wait().await//异步等待
            .unwrap();
        assert!(status.success());
        let ri = rx.recv_async().await.unwrap();// 异步等待从 flume 接收端接收消息。这里使用的是 recv_async 方法，它允许在 tokio 的异步任务中等待消息到达而不阻塞
        assert_eq!(ri.name, DEVICE_NAME);
        assert_eq!(ri.namespace, DEVICE_NAMESPACE);
        //最后两个 assert_eq! 断言验证接收到的消息内容是否与预期一致。
        //这里检查 ResourceIndex<Script> 实例的 name 和 namespace 字段是否分别等于 DEVICE_NAME 和 DEVICE_NAMESPACE，确保 webhook 处理函数正确解析了查询参数并通过通道发送了预期的数据。
    }
}

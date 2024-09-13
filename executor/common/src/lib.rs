use async_stream::stream;//异步代码块转成流  来构建异步 gRPC 客户端的例子
use color_eyre::eyre::eyre;
use color_eyre::{eyre::WrapErr, Result};
use flume::{Receiver, Sender};//futures 库是 Rust 的异步编程的基础，提供了未来（future）、任务（task）、以及异步流（stream）等抽象
use futures::StreamExt;
use proto::server_message::Msg;
use proto::{//gRPC 服务的协议定义
    client_message::{ClientCode, ClientInfo},
    controller_service_client::ControllerServiceClient,
    server_message::RunScript,
    ClientMessage, ServerMessage,
};
use std::result::Result as StdResult;
use tokio::task::JoinHandle;//用于 gRPC 服务的协议定义
use tonic::Streaming;//tonic 是一个基于 tokio 的异步 gRPC 框架
use tonic::{metadata::MetadataValue, transport::channel::Channel, Request, Status};
use tracing::{error, info};

const RE_VERSION: &str = "re-version";
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Client {
    pub client: ControllerServiceClient<Channel>,//tonic gRPC 客户端的一个实例
    pub tasks: Vec<JoinHandle<()>>,//存储的是后台运行的异步任务的句柄
    pub id: u32,//客户端id
    pub rx: Receiver<RunScript>,//接受通道
}
//尝试连接到 gRPC 服务器，并初始化客户端。
impl Client {
    pub async fn try_connect(url: String) -> Result<Client> {
        info!("Connecting to server {}", url);//尝试连接到提供的 URL 对应的 gRPC 服务器，并初始化客户端实例。
        let client = ControllerServiceClient::connect(url).await?;// 异步连接到服务器
        let main_client = client.clone();
        let mut tasks = Vec::new();
        let (tx, rx) = flume::bounded(10);
        let (id, stream) = connect(main_client).await?;//注册或握手操作，返回一个客户端 ID 和一个流（stream）
        info!("Connected!");
        let handle = tokio::spawn(async move {//使用 tokio::spawn 启动一个异步任务来运行 run 函数，处理来自服务器的流式消息
            if let Err(e) = run(stream, tx).await {
                error!(error =? e, "Connection to controller get a error");
            }
        });
        tasks.push(handle);//将这个新创建的任务的句柄添加到 tasks 向量中。
        Ok(Client {
            client,
            tasks,
            id,
            rx,
        })
    }
}
//发送连接和持续连接的消息到服务器，处理服务器的第一条响应消息，并返回客户端 ID 和消息流
async fn connect(
    mut client: ControllerServiceClient<Channel>,
) -> Result<(u32, Streaming<ServerMessage>)> {
    let client_stream = stream! {
        yield ClientMessage {//这个流首先产生一个 ClientMessage，表示客户端想要连接，并可能包含一些初始信息
            code: ClientCode::Connect as i32,
            info: Some(ClientInfo {
                max_job: 0
            })
        };
        loop {
            yield ClientMessage {//不断生成 ClientCode::Continue 消息，表示客户端保持连接
                code: ClientCode::Continue as i32,
                info: None
            }
        }
    };
    let mut request = Request::new(client_stream);
    request//个消息流被包装成 tonic 的 Request 对象，并添加了必要的元数据
        .metadata_mut()//使用 metadata_mut 方法添加元数据，将客户端版本信息添加到请求的元数据中。
        .insert(RE_VERSION, MetadataValue::from_static(CLIENT_VERSION));

    let mut stream = client
        .run(request)//发送请求并从服务器接收响应
        .await//异步等待
        .wrap_err("Got error from server")?
        .into_inner();
    info!("wait for receive first msg from controller");
    let msg = stream.next().await;//从 stream 中异步等待并接收第一条消息 msg
    let executor_id = handle_first_message(msg)?;//处理第一条消息
    info!("Complete process first msg from controller");
    Ok((executor_id, stream))
}
//用于处理从服务器接收到的第一条消息的逻辑
fn handle_first_message(msg: Option<StdResult<ServerMessage, Status>>) -> Result<u32> {
    match msg {
        None => Err(eyre!("Got None on first message")),
        Some(msg) => {//如果 msg 是 Some，函数继续解包并处理内部的 StdResult
            let msg = msg
                .wrap_err("Got error on first message")?
                .msg//从 ServerMessage 中提取 msg 字段，如果 msg 字段是 None，则返回带有错误信息的 Result。
                .ok_or_else(|| eyre!("Got None on ServerMessage"))?;
            match msg {
                Msg::Connected(c) => Ok(c.executor_id),//成功建立连接，函数提取 executor_id
                Msg::Disconnect(d) => Err(eyre!("Got Disconnect on first message: {:?}", d)),
                Msg::Script(s) => Err(eyre!("Got Script on first message: {:?}", s)),
            }
        }
    }
}
//
async fn run(mut stream: Streaming<ServerMessage>, tx: Sender<RunScript>) -> Result<()> {
    loop {//一个异步流，其中包含从服务器接收到的消息
        match stream.next().await {//函数通过一个无限循环不断从 stream 中接收消息，使用 stream.next().await 异步等待下一条消息。
            Some(msg) => {
                let msg = msg
                    .wrap_err("Got error when receive message")?
                    .msg//函数首先检查消息是否成功接收，如果在接收消息时发生错误，则返回一个包装过的错误
                    .ok_or_else(|| eyre!("Got None on ServerMessage"))?;
                match msg {
                    Msg::Connected(_) => return Err(eyre!("Unexpect Connected on ServerMessage")),
                    Msg::Disconnect(d) => {
                        info!(reason =? d.reason(), "Disconnect");
                        break Ok(());
                    }
                    Msg::Script(r) => {
                        tx.send_async(r).await?;// 如果是脚本执行请求，将其通过 tx 发送端发送到另一部分的应用程序处理。
                    }
                }
            }
            None => return Err(eyre!("Unexpect disconnect")),
        }
    }
}

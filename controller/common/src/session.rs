//! This module implement ControllerService
//!

use std::{net::SocketAddr, sync::Arc};//SocketAddr用于表示网络套接字地址

use crate::api::device::{DeviceStatus, Twin, TwinProperty};
use crate::api::{self, Device, Script};
use crate::controller::{wait_for_stop, ControllerState};
use crate::id::{ExecutorID, ExecutorIDGenerator, ScriptID};
use crate::scheduler::ManagerMsg;
use async_stream::stream;
use color_eyre::Result;
use dashmap::DashMap;
use flume::Receiver;
use futures::stream::BoxStream;
use futures::StreamExt;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client};
use proto::server_message::disconnect::DisconnectReason;
use proto::server_message::Msg;
use proto::{client_message::ClientCode, controller_service_server::ControllerService};
use proto::{ClientMessage, QosPolicy, ServerMessage};
use tokio::sync::watch;
use tonic::{async_trait, metadata::MetadataMap, Request, Response, Status, Streaming};//它提供了构建gRPC服务和客户端的基础设施。
use tracing::{error, info, trace};

const RE_VERSION: &str = "re-version";
const MANAGER: &str = "ruleengine";

pub struct SessionManager {
    scripts: Arc<DashMap<ScriptID, ScriptStatus>>,//脚本索引和状态哈希表
    executors: Arc<DashMap<ExecutorID, ExecutorInfo>>,//包含了执行器的详细信息，如执行能力、当前状态等
    executor_idgen: Arc<ExecutorIDGenerator>,//用于生成新的ExecutorID
    client: Client,
    pp: PatchParams,//这是kube库中的一个结构体，用于配置资源补丁操作的参数。
    scheduler: Receiver<ManagerMsg>,//这是一个接收端，用于接收调度器发送的消息
    state: watch::Receiver<ControllerState>,//这是tokio::sync::watch通道的接收端，用于接收关于ControllerState的更新
}

#[derive(Debug)]
struct ScriptStatus {
    name: String,
    namespace: String,
    executor: ExecutorID,
}

#[derive(Debug)]
struct ExecutorInfo {
    addr: SocketAddr,
}

#[derive(Debug, Clone, Copy)]
pub enum ExecutorState {
    Init,
    Ready,
    Pause,
    Disconnect,
}

impl SessionManager {
    pub fn new(//new函数是SessionManager的构造器，用于创建一个新的SessionManager实例
        client: Client,
        scheduler: Receiver<ManagerMsg>,
        state: watch::Receiver<ControllerState>,//用于接收控制器状态的更新
    ) -> Self {
        Self {
            scripts: Default::default(),
            executors: Default::default(),
            executor_idgen: Default::default(),
            client,
            pp: PatchParams::apply(MANAGER),
            scheduler,
            state,
        }
    }
//函数用于验证gRPC请求中的元数据，确保客户端使用的版本与服务器端匹配。
    fn validate_metadata(meta: &MetadataMap) -> Result<(), Status> {
        let version = meta//
            .get(RE_VERSION)//从元数据中获取名为RE_VERSION的条目
            .and_then(|v| v.to_str().ok())//尝试将获取到的版本信息转换为字符串，如果不存在，则默认为空字符串""。
            .unwrap_or("");
        const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");//将获取到的版本与服务器版本SERVER_VERSION进行比较
        if version != SERVER_VERSION {
            error!(version, "Unaccept version");
            return Err(Status::invalid_argument(format!(
                "Version mismatch, server: {}, client: {}",
                SERVER_VERSION, version
            )));
        }
        Ok(())
    }

    fn handle_first_message(//这个handle_first_message方法设计用来处理gRPC服务中接收到的第一个客户端消息，确保它是一个连接（Connect）消息，并且包含必要的连接信息
        &self,
        msg: Result<Option<ClientMessage>, Status>,
    ) -> Result<(), Status> {
        match msg {
            Ok(Some(m)) => {
                if m.code() != ClientCode::Connect {
                    trace!(msg =? m, "First message is not connect message");
                    Err(Status::invalid_argument(
                        "First message is not connect message",
                    ))
                } else {
                    match m.info {
                        None => {
                            trace!(msg =? m, "Connect message don't have connection field");
                            Err(Status::invalid_argument(
                                "Connect message don't have connection field",
                            ))
                        }
                        // TODO: client info ?
                        Some(_info) => Ok(()),
                    }
                }
            }
            Ok(None) => {
                trace!("Got None on connect message");
                Err(Status::invalid_argument("Got None on connect message"))
            }
            Err(e) => {
                error!(msg =? e, "Got error on connect message");
                Err(Status::unknown("Got error on connect message"))
            }
        }
    }
}

#[async_trait]
impl ControllerService for SessionManager {
    type runStream = BoxStream<'static, Result<ServerMessage, Status>>;
//这个 trait 定义了gRPC服务的接口。特别是，这里实现了run异步方法，它处理来自客户端的连接和消息流。
    #[tracing::instrument(skip(self))]
    async fn run(
        // &self,/run方法接收一个Request<Streaming<ClientMessage>>类型的参数request，这代表一个从客户端接收的消息流
        request: Request<Streaming<ClientMessage>>,
    ) -> Result<Response<Self::runStream>, Status> {
        // Header check
        Self::validate_metadata(request.metadata())?;//请求元数据验证 确保客户端使用的版本与服务端兼容
        let addr = request.remote_addr().unwrap();//获取客户端的远程地址。然后
        let mut stream = request.into_inner();//获取客户端的流
        // connect message handle
        self.handle_first_message(stream.message().await)?;//处理流中的第一条消息，这应该是一个连接消息
        info!(addr =? addr, "New executor connection");
        let exeinfo = ExecutorInfo { addr };//创建一个新的ExecutorInfo实例并使用执行器的地址初始化，然后为这个新连接生成一个唯一的executor_id。之后，将执行器信息插入到executors哈希映射中。
        let executor_id = self.executor_idgen.gen();
        let scheduler = self.scheduler.clone();
        let mut state = self.state.clone();
        self.executors.insert(executor_id, exeinfo);
        let executors = self.executors.clone();
        let scripts = self.scripts.clone();
        let s = stream! {
            // connect message response
            yield Ok(message::connected(executor_id));//向客户端发送确认连接的响应，包含执行器的ID。

            // main loop
            loop {
                tokio::select! {//在不同的异步任务之间进行选择：
                    msg = stream.next() => match msg {
                        Some(Ok(msg)) => match msg.code() {
                            ClientCode::Continue => match scheduler.recv_async().await {//: 接收来自调度器的脚本执行任务，更新脚本状态，并向客户端发送执行脚本的消息
                                Err(e) => {
                                    error!(error =? e, "Scheduler is down!");
                                    yield Err(Status::internal("Scheduler is down"));
                                    break;
                                }
                                Ok(task) => {
                                    scripts.insert(task.run.script_id.into(), ScriptStatus {
                                        name: task.name,
                                        namespace: task.namespace,
                                        executor: executor_id
                                    });
                                    yield Ok(ServerMessage {
                                        msg: Some(Msg::Script(task.run))
                                    })
                                }
                            },//如果再次收到连接消息，则记录错误并结束会话
                            ClientCode::Connect => {
                                error!(msg =? msg, "Got unexpect Connect message");
                                yield Err(Status::invalid_argument("Got unexpect Connect message"));
                                break;
                            },// 如果客户端发起断开连接，记录并结束会话
                            ClientCode::Disconnect => {
                                trace!("Client disconnected");
                                yield Ok(message::disconnect(DisconnectReason::ClientExit));
                                break
                            },
                        }
                        Some(Err(e)) => {
                            error!(msg =? e, "Got unexpect Error");
                            yield Err(Status::invalid_argument("Got unexpect Error"));
                            break;
                        }
                        None => {
                            error!("Client unexpect disconnect");
                            break;
                        }
                    },
                    _ = wait_for_stop(&mut state) => {
                        yield Ok(message::disconnect(DisconnectReason::ServerExit));
                        break
                    },
                    else => break,
                };
            }

            // Disconnect
            match executors.remove(&executor_id) {
                Some((id, info)) => {
                    trace!(id =? id, info =? info, "Executor disconnected")
                }
                None => {
                    error!(id =? executor_id, "Unknown executor disconnected")
                }
            }
        }
         .boxed();//使用.boxed()方法将流封装并返回，以适配async_trait中定义的返回类型Result<Response<Self::runStream>, Status>。
        Ok(Response::new(s))
    }

    #[tracing::instrument(skip(self))]
    async fn update_script_status(
        &self,
        status: Request<proto::ScriptStatus>,//通过protobuf定义的数据结构，包含了脚本ID、执行开始时间、持续时间、状态代码和消息等信息。
    ) -> Result<Response<()>, Status> {
        let id = ScriptID::from(status.get_ref().script_id);//从请求中提取脚本ID，并尝试从SessionManager管理的脚本状态集合中移除与此ID对应的条目。
        info!(id =? id, "Script exit");
        match self.scripts.remove(&id) {
            Some((_, sess_status)) => {//成功找到并移除对应的脚本状态
                info!(status =? sess_status, "Script exit");
                let client = self.client.clone();
                let api: Api<Script> = Api::namespaced(client, &sess_status.namespace);//构造一个针对特定命名空间的Api<Script>实例，用于更新Kubernetes中相应脚本资源的状态
                let last_run = status
                    .get_ref()
                    .start
                    .as_ref()
                    .map(|t| t.seconds * 1000 + t.nanos as i64 / 1_000_000)
                    .unwrap_or_default();
                let elapsed_time = status
                    .get_ref()
                    .duration
                    .as_ref()
                    .map(|d| (d.seconds * 1_000_000 + d.nanos as i64 / 1000) as u32)
                    .unwrap_or_default();
                let api_status = api::script::ScriptStatus {
                    last_run,
                    elapsed_time,
                    status: status.get_ref().code,
                    message: status.into_inner().message,
                };//根据请求中提供的状态信息构造新的脚本状态api::script::ScriptStatus，包括最后一次运行时间、运行持续时间、状态代码和消息
                let patch = serde_json::json!({ "status": api_status });
                let patch = Patch::Merge(&patch);
                match api.patch_status(&sess_status.name, &self.pp, &patch).await {//异步发送PATCH请求，更新脚本资源的状态。
                    Ok(_) => Ok(Response::new(())),
                    Err(e) => {
                        error!(error =? e, "Failed to update status of Script");
                        Err(Status::internal("Failed to update status of Script"))
                    }
                }
            }
            None => {//如果在脚本状态集合中找不到对应的脚本ID，记录错误日志
                error!(request =? status, "Got message of updating script status， but script isn't running");
                Err(Status::invalid_argument(
                    "Got message of updating script status， but script isn't running",
                ))
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn update_device_desired(
        &self,
        device: Request<proto::UpdateDevice>,//接收更新设备所需状态的请求
    ) -> Result<Response<()>, Status> {
        let id = ScriptID::from(device.get_ref().script_id);
        info!(id =? id, "Script update device");
        match self.scripts.get(&id) {//从请求中提取脚本ID，并查找SessionManager管理的脚本状态集合中是否存在该脚本ID对应的条目。
            Some(sess_script) => {
                let client = self.client.clone();//构建实例
                let api: Api<Device> = Api::namespaced(client, &sess_script.namespace);
                let mut twins = Vec::new();
                for (k, v) in device.get_ref().desired.iter() {
                    twins.push(Twin {//遍历请求中提供的所需状态信息，构造Twin对象的数组，每个Twin对象包含属性名称、所需属性值，以及空的报告属性值（
                        property_name: k.to_owned(),
                        desired: TwinProperty::new(v.to_owned()),
                        reported: None,
                    })
                }
                let api_status = DeviceStatus { twins };//构建新的设备状态DeviceStatus，包括上一步构造的Twins数组。
                let patch = serde_json::json!({ "status": api_status });
                let patch = Patch::Merge(&patch);
                if let Err(e) = api.patch(&device.get_ref().name, &self.pp, &patch).await {
                    error!(error =? e, "Failed to update status of Device");
                    return Err(Status::internal("Failed to update status of Device"));
                }//使用api.patch方法异步发送PATCH请求，更新设备资源的状态。如果更新操作失败，记录错误日志并返回相应的错误状态
                // FIXME: Qos
                match device.get_ref().qos() {
                    QosPolicy::AtMostOnce => Ok(Response::new(())),
                    QosPolicy::AtLeastOnce => Err(Status::internal("Not implemented")),
                    QosPolicy::OnlyOnce => Err(Status::internal("Not implemented")),
                }
            }
            None => {
                error!(request =? device, "Got message of updating device desired, but script isn't running");
                Err(Status::invalid_argument(
                    "Got message of updating device desired, but script isn't running",
                ))
            }
        }
    }
}

mod message {//它提供了两个函数：connected和disconnect，用于构造ServerMessage类型的消息
    use crate::id::ExecutorID;
    use proto::{
        server_message::{disconnect::DisconnectReason, Connected, Disconnect, Msg},
        ServerMessage,
    };

    pub(super) fn connected(executor_id: ExecutorID) -> ServerMessage {
        ServerMessage {
            msg: Some(Msg::Connected(Connected {
                executor_id: executor_id.into(),
            })),//函数构造一个ServerMessage实例，其中msg字段被设置为Msg::Connected变体，携带一个Connected类型的消息。Connected消息包含了executor_id，转换为内部表示（可能是一个整数或字符串，取决于ExecutorID类型到相应内部类型的转换逻辑）。
        }
    }

    pub(super) fn disconnect(reason: DisconnectReason) -> ServerMessage {
        ServerMessage {
            msg: Some(Msg::Disconnect(Disconnect {
                reason: reason as i32,
            })),//它构造一个ServerMessage实例，其中msg字段被设置为Msg::Disconnect变体，携带一个Disconnect类型的消息。Disconnect消息中包含了断开连接的原因，原因值被转换为i32类型（因为protobuf通常使用基础类型来表示枚举值）。
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tracing::Level;
    use tracing_subscriber::{filter::Targets, prelude::*};
    #[tokio::test]
    async fn patch_device() {//结合格式化层(fmt::layer())和目标级别(Targets)来初始化日志系统
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .pretty()
                    .with_thread_names(true),
            )
            .with(Targets::new().with_default(Level::TRACE))
            .init();

        let client = Client::try_default().await.unwrap();//尝试创建一个默认的Kubernetes客户端。这通常会从运行环境（如服务账户或~/.kube/config文件）加载配置。
        let api: Api<Device> = Api::namespaced(client, "default");//创建一个类型为Device的Api实例
        let mut twins = Vec::new();
        twins.push(Twin {
            property_name: "temperature".to_owned(),
            desired: TwinProperty::new("10".to_owned()),
            reported: Some(TwinProperty::new("-17".to_owned())),
        });//构建一个包含单个Twin对象的向量，该对象包含属性名"temperature"，所需状态为"10"，报告状态为"-17"
        let api_status = DeviceStatus { twins };
        let patch = serde_json::json!({ "status": api_status });//使用serde_json::json!宏构造一个JSON补丁，其中包含上一步构造的设备状态。
        let patch = Patch::Merge(&patch);//将这个JSON对象包装为一个Merge类型的Patch，这表示使用合并补丁的方式来更新资源状态。
        let pp = PatchParams::apply(MANAGER);//建一个PatchParams实例，指定补丁的来源为MANAGER（可能是一个表示管理系统或操作者的标识符）
        api.patch("dht11", &pp, &patch).await.unwrap();//异步发送PATCH请求，尝试更新名为"dht11"的设备的状态。这里的unwrap()用于断言操
    }
}

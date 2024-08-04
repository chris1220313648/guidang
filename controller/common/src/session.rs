//! This module implement ControllerService
//!

use std::{net::SocketAddr, sync::Arc};

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
use tonic::{async_trait, metadata::MetadataMap, Request, Response, Status, Streaming};
use tracing::{error, info, trace};
use rusqlite::{params, Connection};
use std::sync::{Mutex};
const RE_VERSION: &str = "re-version";
const MANAGER: &str = "ruleengine";

pub struct SessionManager {
    scripts: Arc<DashMap<ScriptID, ScriptStatus>>,//记录控制器id和脚本执行状态
    executors: Arc<DashMap<ExecutorID, ExecutorInfo>>,//执行器id和执行器信息
    executor_idgen: Arc<ExecutorIDGenerator>,//执行器id生成
    client: Client,//kube客户端
    pp: PatchParams,//用于配置更新操作的参数
    scheduler: Receiver<ManagerMsg>,
    state: watch::Receiver<ControllerState>,//接受控制器状态变化
    conn: Arc<Mutex<Connection>>
}

#[derive(Debug)]
struct ScriptStatus {//包括执行器id
    name: String,
    namespace: String,
    executor: ExecutorID,
}

#[derive(Debug)]
struct ExecutorInfo {//执行器地址
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
    pub fn new(
        client: Client,
        scheduler: Receiver<ManagerMsg>,
        state: watch::Receiver<ControllerState>,
    ) -> Self {
        let conn = match Connection::open("./test.db") {
            Ok(conn) =>Arc::new(Mutex::new(conn)),
            Err(e) => {
                eprintln!("Failed to open database connection: {}", e);
                std::process::exit(1); // 如果无法打开数据库连接，则退出程序
            }
        };
        Self {
            scripts: Default::default(),
            executors: Default::default(),
            executor_idgen: Default::default(),
            client,
            pp: PatchParams::apply(MANAGER),//用于控制更新操作
            scheduler,
            state,
            conn,
        }
    }

    fn validate_metadata(meta: &MetadataMap) -> Result<(), Status> {//验证元数据信息和服务器版本是否匹配
        let version = meta
            .get(RE_VERSION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
        if version != SERVER_VERSION {
            error!(version, "Unaccept version");
            return Err(Status::invalid_argument(format!(
                "Version mismatch, server: {}, client: {}",
                SERVER_VERSION, version
            )));
        }
        Ok(())
    }

    fn handle_first_message(//处理服务器连接上来的第一条消息
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
    type runStream = BoxStream<'static, Result<ServerMessage, Status>>;//响应流？

    #[tracing::instrument(skip(self))]
    async fn run(
        &self,
        request: Request<Streaming<ClientMessage>>,
    ) -> Result<Response<Self::runStream>, Status> {
        // Header check
        Self::validate_metadata(request.metadata())?;
        let addr = request.remote_addr().unwrap();//获取客户端地址
        let mut stream = request.into_inner();
        // connect message handle
        self.handle_first_message(stream.message().await)?;//处理连接消息
        info!(addr =? addr, "New executor connection");
        let exeinfo = ExecutorInfo { addr };//创建执行器信息
        let executor_id = self.executor_idgen.gen();//生成新的执行器id
        let scheduler = self.scheduler.clone();//克隆mabagermsg接受端
        let mut state = self.state.clone();//
        self.executors.insert(executor_id, exeinfo);
        let executors = self.executors.clone();//
        let scripts = self.scripts.clone();
        let s = stream! {
            // connect message response
            yield Ok(message::connected(executor_id));

            // main loop
            loop {
                tokio::select! {//用于同时等待多个异步操作，处理第一个完成的操作。
                    msg = stream.next() => match msg {//从客户端流中读取下一条消息。
                        Some(Ok(msg)) => match msg.code() {//客户端continue
                            ClientCode::Continue => match scheduler.recv_async().await {//接收控制器传来的消息ManagerMsg
                                Err(e) => {
                                    error!(error =? e, "Scheduler is down!");
                                    yield Err(Status::internal("Scheduler is down"));
                                    break;
                                }
                                Ok(task) => {
                                    scripts.insert(task.run.script_id.into(), ScriptStatus {//？？？？？？？
                                        name: task.name,
                                        namespace: task.namespace,
                                        executor: executor_id
                                    });
                                    yield Ok(ServerMessage {
                                        msg: Some(Msg::Script(task.run))
                                    })
                                }
                            },
                            ClientCode::Connect => {
                                error!(msg =? msg, "Got unexpect Connect message");
                                yield Err(Status::invalid_argument("Got unexpect Connect message"));
                                break;
                            },
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
            match executors.remove(&executor_id) {//停止后执行器并记录日志
                Some((id, info)) => {
                    trace!(id =? id, info =? info, "Executor disconnected")
                }
                None => {
                    error!(id =? executor_id, "Unknown executor disconnected")
                }
            }
        }
        .boxed();
        Ok(Response::new(s))
    }

    #[tracing::instrument(skip(self))]
    async fn update_script_status(
        &self,
        status: Request<proto::ScriptStatus>,
    ) -> Result<Response<()>, Status> {
        let id = ScriptID::from(status.get_ref().script_id);
        info!(id =? id, "Script exit");
        match self.scripts.remove(&id) {//移除脚本id的状态
            Some((_, sess_status)) => {
                info!(status =? sess_status, "Script exit");//
                let client = self.client.clone();
                // let api: Api<Script> = Api::namespaced(client, &sess_status.namespace);//创建一个命名空间的Api<Script>
                let last_run = status//提取开始时间
                    .get_ref()
                    .start
                    .as_ref()
                    .map(|t| t.seconds * 1000 + t.nanos as i64 / 1_000_000)
                    .unwrap_or_default();
                let elapsed_time = status//提取持续时间
                    .get_ref()
                    .duration
                    .as_ref()
                    .map(|d| (d.seconds * 1_000_000 + d.nanos as i64 / 1000) as u32)
                    .unwrap_or_default();
                let new_status = api::script_sqlite3::ScriptStatus {
                    last_run,
                    elapsed_time,
                    status: status.get_ref().code,
                    message: status.into_inner().message,
                };
                let conn = self.conn.lock().unwrap();
                match conn.execute(
                    "UPDATE Script SET LastRun = ?, ElapsedTime = ?, Status = ?, Message = ? WHERE ScriptID = ?",
                    params![
                        new_status.last_run,
                        new_status.elapsed_time,
                        new_status.status,
                        new_status.message,
                        id.to_u32() as i32,
                        
                    ],
                ) {
                    Ok(_) => Ok(Response::new(())),
                    Err(e) => {
                        error!(error =? e, "Failed to update status of Script");
                        Err(Status::internal("Failed to update status of Script"))
                    }
                }
            }
            None => {
                error!(request =? status, "Got message of updating script status, but script isn't running");
                Err(Status::invalid_argument(
                    "Got message of updating script status, but script isn't running",
                ))
            }
                // let patch = serde_json::json!({ "status": api_status });
                // let patch = Patch::Merge(&patch);//转化为json并创建和合并补丁
                // match api.patch_status(&sess_status.name, &self.pp, &patch).await {//更新脚本状态
                //     Ok(_) => Ok(Response::new(())),
                //     Err(e) => {
                //         error!(error =? e, "Failed to update status of Script");
                //         Err(Status::internal("Failed to update status of Script"))
                //     }
                // }
            }

        }
    

    #[tracing::instrument(skip(self))]
    async fn update_device_desired(
        &self,
        device: Request<proto::UpdateDevice>,
    ) -> Result<Response<()>, Status> {
        let id = ScriptID::from(device.get_ref().script_id);
        info!(id =? id, "Script update device");
        match self.scripts.get(&id) {
            Some(sess_script) => {//sess_script是脚本执行状态信息
                let client = self.client.clone();
                
                let mut twins = Vec::new();
                for (k, v) in device.get_ref().desired.iter() {
                    twins.push(Twin {//从请求中提取期望的设备状态，创建 Twin 对象，并添加到 twins 列表中
                        property_name: k.to_owned(),//属性名
                        desired: TwinProperty::new(v.to_owned()),//属性值
                        reported: None,
                    })
                }
                let status: DeviceStatus = DeviceStatus { twins };
                let conn = self.conn.lock().unwrap();
                let device_name = device.get_ref().name.clone();

                // 查询设备 ID
                let mut stmt = conn.prepare("SELECT id FROM devices WHERE name = ?")?;
                let device_id: i32 = stmt.query_row(params![device_name], |row| row.get(0))?;

                for twin in twins {
                    let desired_json = serde_json::to_string(&twin.desired).unwrap();
                    let reported_json = twin.reported.map(|r| serde_json::to_string(&r).unwrap());

                    conn.execute(
                        "INSERT INTO twins (device_id, property_name, desired, reported) VALUES (?, ?, ?, ?)
                         ON CONFLICT(device_id, property_name) DO UPDATE SET desired = excluded.desired, reported = excluded.reported",
                        params![device_id, twin.property_name, desired_json, reported_json],
                    )?;
                }

                info!("Device status updated successfully.");
                Ok(Response::new(()))

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

mod message {//定义了一些与消息相关的函数，这些函数用于创建特定类型的 ServerMessage 实例
    use crate::id::ExecutorID;
    use proto::{
        server_message::{disconnect::DisconnectReason, Connected, Disconnect, Msg},
        ServerMessage,
    };

    pub(super) fn connected(executor_id: ExecutorID) -> ServerMessage {
        ServerMessage {
            msg: Some(Msg::Connected(Connected {
                executor_id: executor_id.into(),
            })),
        }
    }

    pub(super) fn disconnect(reason: DisconnectReason) -> ServerMessage {
        ServerMessage {
            msg: Some(Msg::Disconnect(Disconnect {
                reason: reason as i32,
            })),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tracing::Level;
    use tracing_subscriber::{filter::Targets, prelude::*};
    #[tokio::test]
    async fn patch_device() {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .pretty()
                    .with_thread_names(true),
            )
            .with(Targets::new().with_default(Level::TRACE))
            .init();

        let client = Client::try_default().await.unwrap();
        let api: Api<Device> = Api::namespaced(client, "default");
        let mut twins = Vec::new();
        twins.push(Twin {
            property_name: "temperature".to_owned(),
            desired: TwinProperty::new("10".to_owned()),
            reported: Some(TwinProperty::new("-17".to_owned())),
        });
        let api_status = DeviceStatus { twins };
        let patch = serde_json::json!({ "status": api_status });
        let patch = Patch::Merge(&patch);
        let pp = PatchParams::apply(MANAGER);
        api.patch("dht11", &pp, &patch).await.unwrap();
    }
}

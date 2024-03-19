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

const RE_VERSION: &str = "re-version";
const MANAGER: &str = "ruleengine";

pub struct SessionManager {
    scripts: Arc<DashMap<ScriptID, ScriptStatus>>,
    executors: Arc<DashMap<ExecutorID, ExecutorInfo>>,
    executor_idgen: Arc<ExecutorIDGenerator>,
    client: Client,
    pp: PatchParams,
    scheduler: Receiver<ManagerMsg>,
    state: watch::Receiver<ControllerState>,
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
    pub fn new(
        client: Client,
        scheduler: Receiver<ManagerMsg>,
        state: watch::Receiver<ControllerState>,
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

    fn validate_metadata(meta: &MetadataMap) -> Result<(), Status> {
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

    fn handle_first_message(
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

    #[tracing::instrument(skip(self))]
    async fn run(
        &self,
        request: Request<Streaming<ClientMessage>>,
    ) -> Result<Response<Self::runStream>, Status> {
        // Header check
        Self::validate_metadata(request.metadata())?;
        let addr = request.remote_addr().unwrap();
        let mut stream = request.into_inner();
        // connect message handle
        self.handle_first_message(stream.message().await)?;
        info!(addr =? addr, "New executor connection");
        let exeinfo = ExecutorInfo { addr };
        let executor_id = self.executor_idgen.gen();
        let scheduler = self.scheduler.clone();
        let mut state = self.state.clone();
        self.executors.insert(executor_id, exeinfo);
        let executors = self.executors.clone();
        let scripts = self.scripts.clone();
        let s = stream! {
            // connect message response
            yield Ok(message::connected(executor_id));

            // main loop
            loop {
                tokio::select! {
                    msg = stream.next() => match msg {
                        Some(Ok(msg)) => match msg.code() {
                            ClientCode::Continue => match scheduler.recv_async().await {
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
            match executors.remove(&executor_id) {
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
        match self.scripts.remove(&id) {
            Some((_, sess_status)) => {
                info!(status =? sess_status, "Script exit");
                let client = self.client.clone();
                let api: Api<Script> = Api::namespaced(client, &sess_status.namespace);
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
                };
                let patch = serde_json::json!({ "status": api_status });
                let patch = Patch::Merge(&patch);
                match api.patch_status(&sess_status.name, &self.pp, &patch).await {
                    Ok(_) => Ok(Response::new(())),
                    Err(e) => {
                        error!(error =? e, "Failed to update status of Script");
                        Err(Status::internal("Failed to update status of Script"))
                    }
                }
            }
            None => {
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
        device: Request<proto::UpdateDevice>,
    ) -> Result<Response<()>, Status> {
        let id = ScriptID::from(device.get_ref().script_id);
        info!(id =? id, "Script update device");
        match self.scripts.get(&id) {
            Some(sess_script) => {
                let client = self.client.clone();
                let api: Api<Device> = Api::namespaced(client, &sess_script.namespace);
                let mut twins = Vec::new();
                for (k, v) in device.get_ref().desired.iter() {
                    twins.push(Twin {
                        property_name: k.to_owned(),
                        desired: TwinProperty::new(v.to_owned()),
                        reported: None,
                    })
                }
                let api_status = DeviceStatus { twins };
                let patch = serde_json::json!({ "status": api_status });
                let patch = Patch::Merge(&patch);
                if let Err(e) = api.patch(&device.get_ref().name, &self.pp, &patch).await {
                    error!(error =? e, "Failed to update status of Device");
                    return Err(Status::internal("Failed to update status of Device"));
                }
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

mod message {
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

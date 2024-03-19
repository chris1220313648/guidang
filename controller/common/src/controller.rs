use crate::api::{Device, Script};
use crate::scheduler::{trigger, ManagerMsg, Reflector, ResourceIndex, Scheduler};
use crate::session::SessionManager;
use color_eyre::Result;
use flume::{Receiver, Sender};
use futures::StreamExt;
use kube::{api::ListParams, Api, Client};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{error, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControllerState {
    Init,
    Running,
    Stop,
}

#[tracing::instrument(skip_all)]
pub async fn wait_for_init(rx: &mut watch::Receiver<ControllerState>) {
    loop {
        let state = *rx.borrow_and_update();
        if state != ControllerState::Init {
            return;
        }
        let _ = rx.changed().await;
    }
}

#[tracing::instrument(skip_all)]
pub async fn wait_for_stop(rx: &mut watch::Receiver<ControllerState>) {
    loop {
        let state = *rx.borrow_and_update();
        if state == ControllerState::Stop {
            return;
        }
        let _ = rx.changed().await;
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Config {
    pub webaddr: SocketAddr,
    pub grpcaddr: SocketAddr,
    pub mqttaddr: SocketAddr,
}

pub struct Controller {
    pub controller_tasks: Vec<JoinHandle<()>>,
    state: watch::Sender<ControllerState>,
    state_rx: watch::Receiver<ControllerState>,
    config: Config,
}

impl Controller {
    pub fn new(config: Config) -> Result<Controller> {
        let (tx, rx) = tokio::sync::watch::channel(ControllerState::Init);
        Ok(Controller {
            controller_tasks: Vec::new(),
            state: tx,
            state_rx: rx,
            config,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn run(mut self) -> Result<()> {
        self.init();
        use tokio::{signal, time};
        tokio::select! {
            _ = signal::ctrl_c() => {}
            _ = Self::setup_term_handler() => {}
            _ = Self::setup_hup_handler() => {}
        }
        self.stop();
        time::sleep(time::Duration::from_millis(100)).await;
        self.kill_all();
        Ok(())
    }

    #[tracing::instrument]
    pub async fn setup_term_handler() -> Result<()> {
        use tokio::signal;
        let mut signal = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        signal.recv().await;
        Ok(())
    }

    #[tracing::instrument]
    pub async fn setup_hup_handler() -> Result<()> {
        use tokio::signal;
        let mut signal = signal::unix::signal(signal::unix::SignalKind::hangup())?;
        signal.recv().await;
        Ok(())
    }

    pub fn spawn(&mut self, task: impl Future<Output = Result<()>> + Send + 'static) {
        let mut state = self.state_rx.clone();
        let handle = tokio::spawn(async move {
            wait_for_init(&mut state).await;
            if let Err(e) = tokio::select! {
                r = task => r,
                _ = wait_for_stop(&mut state) => Ok(())
            } {
                error!(error =? e, "Task throw a error")
            }
        });
        self.controller_tasks.push(handle)
    }

    pub fn spawn_kubeapi(
        &mut self,
        client: Client,
        is_cloud: bool,
    ) -> (
        Sender<ResourceIndex<Script>>,
        Sender<ResourceIndex<Device>>,
        Receiver<ManagerMsg>,
        Arc<Reflector>,
    ) {
        use crate::trigger::kubeapi::*;
        let reflector_store = Arc::new(Reflector::default());

        // Scheduler
        let (schin_tx, schin_rx) = flume::bounded(10);
        let (schout_tx, schout_rx) = flume::bounded(10);
        let reflector_clone = reflector_store.clone();
        self.spawn(async move {
            let mut in_rx = schin_rx.into_stream();
            let mut scheduler = Scheduler::new(reflector_clone);
            while let Some(index) = in_rx.next().await {
                info!("Triger new script to run: {:?}", index);
                match scheduler.lookup(index) {
                    Ok(msg) => schout_tx.send(msg)?,
                    Err(e) => error!(error =? e, "Scheduler throw a error"),
                }
            }
            Ok(())
        });

        // Device to Script map
        let reflector_clone = reflector_store.clone();
        let (schdevin_tx, schdevin_rx) = flume::bounded(10);
        let schin_tx_clone = schin_tx.clone();
        self.spawn(async move { trigger(reflector_clone, schdevin_rx, schin_tx_clone).await });

        // script reflector
        let mut script_async_hooks = Vec::new();
        let script_sync_hooks = vec![logger_hook()];
        let script_api: Api<Script> = Api::all(client.clone());

        // device reflector
        let mut device_async_hooks = Vec::new();
        let device_sync_hooks = vec![logger_hook()];
        let device_api: Api<Device> = Api::all(client);

        // device_hook for device reflector
        let (device_tx, device_rx) = flume::bounded(3);

        if is_cloud {
            let device_rx = device_rx.clone();
            let schdevin_tx = schdevin_tx.clone();
            self.spawn(async move { trigger_hook(device_rx, schdevin_tx).await });
        }

        let reflector_clone = reflector_store.clone();
        self.spawn(async move { device_hook(device_rx, reflector_clone).await });
        device_async_hooks.push(device_tx);

        // script_hook for device reflector
        let (script_tx, script_rx) = flume::bounded(3);
        let reflector_clone = reflector_store.clone();
        self.spawn(async move { script_hook(script_rx, reflector_clone).await });
        script_async_hooks.push(script_tx);

        // script reflector
        self.spawn(async move {
            reflector(
                script_api,
                ListParams::default(),
                script_async_hooks,
                script_sync_hooks,
            )
            .await
        });

        // device reflector
        self.spawn(async move {
            reflector(
                device_api,
                ListParams::default(),
                device_async_hooks,
                device_sync_hooks,
            )
            .await
        });

        (schin_tx, schdevin_tx, schout_rx, reflector_store)
    }

    pub fn spawn_mqtt(&mut self, scheduler: Sender<ResourceIndex<Device>>) {
        use crate::trigger::mqtt::*;
        let sync_hooks = vec![trigger_hook(scheduler), logger_hook()];
        let async_hooks = Vec::new();
        let host = self.config.mqttaddr.ip().to_string();
        let port = self.config.mqttaddr.port();
        self.spawn(async move { mqtt_client(host, port, async_hooks, sync_hooks).await });
    }

    pub fn spawn_webserver(
        &mut self,
        scheduler: Sender<ResourceIndex<Script>>,
        store: Arc<Reflector>,
    ) {
        use crate::server::*;
        let addr = self.config.webaddr;
        self.spawn(async move { web_server(scheduler, store, addr).await });
    }

    pub fn spawn_grpc(&mut self, client: Client, scheduler: Receiver<ManagerMsg>) {
        let addr = self.config.grpcaddr;
        let mut state = self.state_rx.clone();
        let handle = tokio::spawn(async move {
            wait_for_init(&mut state).await;
            let mgr = SessionManager::new(client, scheduler, state);
            if let Err(e) = crate::server::grpc_server(addr, mgr).await {
                error!(error =? e, "Grpc server is down!");
            }
        });
        self.controller_tasks.push(handle)
    }

    pub fn init(&mut self) {
        {
            let state = *self.state_rx.borrow();
            if state != ControllerState::Init {
                panic!("Can't run twice");
            }
        }
        self.state.send(ControllerState::Running).unwrap();
        info!("Init!")
    }

    pub fn stop(&mut self) {
        {
            let state = *self.state_rx.borrow();
            match state {
                ControllerState::Init => panic!("not init yet"),
                ControllerState::Stop => panic!("Already stop"),
                _ => {}
            }
        }
        self.state.send(ControllerState::Stop).unwrap();
    }

    pub fn kill_all(&self) {
        for j in &self.controller_tasks {
            j.abort()
        }
    }
}

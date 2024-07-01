use crate::api::{Device, Script};
use crate::scheduler::{trigger, ManagerMsg, Reflector, ResourceIndex, Scheduler};
use crate::session::SessionManager;
use crate::trigger::sqlite3api::reflector_sqlite3;
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
use rusqlite::{params, Connection};
use std::sync::Mutex;
use color_eyre::Report;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControllerState {
    Init,
    Running,
    Stop,
}

#[tracing::instrument(skip_all)]
//定义异步函数 等待控制器状态变为非Init
pub async fn wait_for_init(rx: &mut watch::Receiver<ControllerState>) {
    loop {
        let state = *rx.borrow_and_update();//获取当前状态并立即更新内部版本，返回当前值的引用
        if state != ControllerState::Init {
            return;
        }
        let _ = rx.changed().await;//等待直到状态发生变化。这个操作会暂停当前任务，直到有新的状态广播,有新的广播就会检测
    }
}
// 定义一个异步函数wait_for_stop，等待控制器状态变为Stop。
#[tracing::instrument(skip_all)]
pub async fn wait_for_stop(rx: &mut watch::Receiver<ControllerState>) {
    loop {
        let state = *rx.borrow_and_update();
        if state == ControllerState::Stop {
            return;
        }
        let _ = rx.changed().await;//等待状态变化
    }
}
// 定义一个结构体Config，包含Web服务器、gRPC服务器和MQTT服务器的地址。
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Config {
    pub webaddr: SocketAddr,
    pub grpcaddr: SocketAddr,
    pub mqttaddr: SocketAddr,
}

// 定义一个结构体Controller，表示控制器。
pub struct Controller {
    pub controller_tasks: Vec<JoinHandle<()>>, // 控制器启动的任务列表。
    state: watch::Sender<ControllerState>, // 用于修改控制器状态的发送端。
    state_rx: watch::Receiver<ControllerState>, // 用于接收控制器状态的接收端。
    config: Config, // 控制器配置。
}

impl Controller {
    // 定义一个函数new，用于创建一个Controller实例。
    pub fn new(config: Config) -> Result<Controller> {
        let (tx, rx) = tokio::sync::watch::channel(ControllerState::Init);
        Ok(Controller {
            controller_tasks: Vec::new(),
            state: tx,
            state_rx: rx,
            config,
        })
    }
// // 定义一个异步函数run，启动控制器的主逻辑。
    #[tracing::instrument(skip_all)]
    pub async fn run(mut self) -> Result<()> {
        self.init();
        use tokio::{signal, time};
        tokio::select! {//同时监听多个异步事件
            _ = signal::ctrl_c() => {}
            _ = Self::setup_term_handler() => {}
            _ = Self::setup_hup_handler() => {}
        }
        self.stop();
        time::sleep(time::Duration::from_millis(100)).await;
        self.kill_all();
        Ok(())
    }
// 定义一个异步函数setup_term_handler，设置终止信号处理器。
    #[tracing::instrument]
    pub async fn setup_term_handler() -> Result<()> {
        use tokio::signal;
        let mut signal = signal::unix::signal(signal::unix::SignalKind::terminate())?;//创建一个监听 SIGTERM 的信号流
        signal.recv().await;// 等待信号。
        Ok(())
    }
 // 定义一个异步函数setup_hup_handler，设置挂起信号处理器。
    #[tracing::instrument]
    pub async fn setup_hup_handler() -> Result<()> {
        use tokio::signal;
        let mut signal = signal::unix::signal(signal::unix::SignalKind::hangup())?;
        signal.recv().await;
        Ok(())
    }
 // 定义一个函数spawn，用于启动一个异步任务。
    pub fn spawn(&mut self, task: impl Future<Output = Result<()>> + Send + 'static) {
        let mut state = self.state_rx.clone();
        let handle = tokio::spawn(async move {//使用 tokio::spawn 函数将一个异步块提交到 Tokio 执行器上执
            wait_for_init(&mut state).await;
            if let Err(e) = tokio::select! {//用 tokio::select! 宏来同时监听任务执行和停止信号
                r = task => r,
                _ = wait_for_stop(&mut state) => Ok(())//控制器stop状态 
            } {
                error!(error =? e, "Task throw a error")
            }
        });
        self.controller_tasks.push(handle)//存储任务句柄
    }
// 定义一个函数spawn_kubeapi，用于启动与Kubernetes API相关的异步任务。
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
        use crate::trigger::kubeapi::*;// // 创建一个Reflector实例。
        let reflector_store = Arc::new(Reflector::default());

        // Scheduler
        // 创建一个Scheduler输入的通道。 发出和接收脚本信息
        let (schin_tx, schin_rx) = flume::bounded(10);
        // 创建一个Scheduler输出的通道。  发出和接受管理消息
        let (schout_tx, schout_rx) = flume::bounded(10);
         // 克隆Reflector实例。
        let reflector_clone = reflector_store.clone();
        self.spawn(async move {// 启动Scheduler相关的异步任务。
            let mut in_rx = schin_rx.into_stream();// 将接收端转换为Stream。
            let mut scheduler = Scheduler::new(reflector_clone);// 创建Scheduler实例。
            while let Some(index) = in_rx.next().await {// 循环处理接收到的消息。
                info!("Triger new script to run: {:?}", index);// 记录日志。
                match scheduler.lookup(index) {// 查找并处理消息。
                    Ok(msg) => schout_tx.send(msg)?,// 发送处理结果。
                    Err(e) => error!(error =? e, "Scheduler throw a error"),// 记录错误日志。
                }
            }
            Ok(())
        });
      
        // Device to Script map
        let reflector_clone = reflector_store.clone();
        // trigger输入 输出
        let (schdevin_tx, schdevin_rx) = flume::bounded(10);
        // 克隆Scheduler输入通道的发送端。
        let schin_tx_clone = schin_tx.clone();
        // 启动设备到脚本映射的异步任务。接收设备信息 发出对应脚本信息
        self.spawn(async move { trigger(reflector_clone, schdevin_rx, schin_tx_clone).await });

        // script reflector
        // 创建空的异步钩子列表。
        // let mut script_async_hooks = Vec::new();
        // 创建同步钩子列表。
        // let script_sync_hooks = vec![logger_hook()];
        // 创建Script API实例
        // let script_api: Api<Script> = Api::all(client.clone());
 

        // device reflector
        // 创建空的异步钩子列表。
        let mut device_async_hooks = Vec::new();
        // 创建同步钩子列表。
        let device_sync_hooks = vec![logger_hook()];
        // 创建Device API实例。
        let device_api: Api<Device> = Api::all(client);

        // device_hook for device reflector
        let (device_tx, device_rx) = flume::bounded(3);
        // 创建设备钩子的通道。
        if is_cloud {
            let device_rx = device_rx.clone();
            let schdevin_tx = schdevin_tx.clone();
            self.spawn(async move { trigger_hook(device_rx, schdevin_tx).await });//设备事件-设备索引发到调度期
        }

        let reflector_clone = reflector_store.clone();// 克隆Reflector实例。
        self.spawn(async move { device_hook(device_rx, reflector_clone).await });
        device_async_hooks.push(device_tx);// 将设备发送端添加到异步钩子列表中。

        // script_hook for device reflector
        // 创建脚本钩子的通道。
        // let (script_tx, script_rx) = flume::bounded(3);
        // let reflector_clone = reflector_store.clone();// 克隆Reflector实例。
        // self.spawn(async move { script_hook(script_rx, reflector_clone).await });// 启动脚本钩子的异步任务。
        // script_async_hooks.push(script_tx);//将脚本事件发送端添加到异步钩子中

        // script reflector
        // self.spawn(async move {
        //     reflector(
        //         script_api,
        //         ListParams::default(),
        //         script_async_hooks,
        //         script_sync_hooks,         
        let reflector_clone = reflector_store.clone();// 克隆Reflector实例。
        let _conn = match Connection::open("./test.db") {
            Ok(conn) => {
                info!("open db sucessfully");
                let conn=Arc::new(Mutex::new(conn));
                self.spawn(async move {
                    reflector_sqlite3(conn,reflector_clone).await 
                    
                });
            
            }
            Err(e) => {
                eprintln!("Failed to open database connection: {}", e);
                std::process::exit(1); // 如果无法打开数据库连接，则退出程序
            }
        };
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

    pub fn spawn_mqtt(&mut self, scheduler: Sender<ResourceIndex<Device>>) {//传递schudele 的设备信息发送端
        use crate::trigger::mqtt::*;
        let sync_hooks = vec![trigger_hook(scheduler), logger_hook()];
        let async_hooks = Vec::new();
        let host = self.config.mqttaddr.ip().to_string();
        let port = self.config.mqttaddr.port();
        self.spawn(async move { mqtt_client(host, port, async_hooks, sync_hooks).await });//启动客户端
    }

    pub fn spawn_webserver(//启动http服务器
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
            let mgr = SessionManager::new(client, scheduler, state);//启动会话管理器
            if let Err(e) = crate::server::grpc_server(addr, mgr).await {//在等待初始化完成后，创建一个会话管理器 SessionManager，并尝试启动gRPC服务器。
                error!(error =? e, "Grpc server is down!");
            }
        });
        self.controller_tasks.push(handle)
    }

    pub fn init(&mut self) {
        {
            let state = *self.state_rx.borrow();
            if state != ControllerState::Init {
                panic!("Can't run twice");//获取当前状态，如果状态不是 ControllerState::Init，则触发 panic。
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
        self.state.send(ControllerState::Stop).unwrap();//设置停止
    }

    pub fn kill_all(&self) {//如果状态是 ControllerState::Init，表示还未初始化
        for j in &self.controller_tasks {
            j.abort()//停止所有任务
        }
    }
}

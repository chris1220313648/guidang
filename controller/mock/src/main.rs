use api::{protocol::*, ExecutorID};//引入了api模块中的protocol子模块里的所有项（*表示引入所有公共项）
use clap::Parser;
use color_eyre::eyre::{eyre, Context, ContextCompat, Result};
use controller::api::{Device, Script};
use controller::selector::query_device_from_server;
use controller::session::{ControllerBuilder, HostMessage, HostsMap, IOSession};
use controller::ControllerState;
use kube::{Api, Client, Resource};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::mpsc::Receiver;
use tokio::sync::watch;

const DEFAULT_CONFIG: &str = "/home/han/Project/rule_engine/config/controller.toml";

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    /// path to executor
    executor: HashMap<String, PathBuf>,//映射了执行器的名称到它们的路径。这可以被用来存储不同执行器的位置信息
}

#[derive(Parser)]
struct Args {
    /// path to resource yaml/json file
    resource: PathBuf,//储资源文件的路径，这个字段是必须的，因为没有提供默认值或可选性

    /// path to executor
    #[clap(short, long)]
    executor: Option<PathBuf>,//表示这些字段是可选的。如果提供了对应的命令行参数，这些字段会包含Some(PathBuf)，否则会是None

    #[clap(short, long)]
    /// path to config file
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Args::parse();//解析 命令行参数 存在 opt中
    color_eyre::install()?;
    let mut resource_file = File::open(&opt.resource)?;//打开由命令行参数指定的资源文件，并将其内容读取到字符串resource中
    let mut resource = String::new();
    resource_file.read_to_string(&mut resource)?;//转换字符窜
    drop(resource_file);
    let resource: Script = match opt.resource.extension() {
        Some(e) if e == "json" => {
            serde_json::from_str(&resource).context("Parse resource file as json")?//。context为上下文添加错误信息
        }
        Some(e) if e == "yaml" || e == "yml" => {
            serde_yaml::from_str(&resource).context("Parse resource file as yaml")?//?操作符用于简化错误处理
        }
        Some(_) | None => {
            let try_yaml: Result<Script> =
                serde_yaml::from_str(&resource).context("Parse resource file as json");
            match try_yaml {
                Ok(s) => s,
                Err(_) => {
                    let try_json: Result<Script> =
                        serde_json::from_str(&resource).context("Parse resource file as yaml");
                    if try_json.is_err() {
                        Err(try_json
                            .unwrap_err()
                            .wrap_err(eyre!("Can't parse resource file")))?
                    } else {
                        try_json.unwrap()
                    }
                }
            }
        }
    };
    let config_file = if let Some(config) = opt.config.as_ref() {//如果结果是 Some(config)
        config//果匹配成功（即，opt.config 是 Some），则 config_file 变量被赋值为 config
    } else {
        Path::new(DEFAULT_CONFIG)
    };
    if !config_file.exists() || !config_file.is_file() {//检查文件是否存在
        println!(
            "Config file not found! The default config path is {}",
            DEFAULT_CONFIG
        );
        println!("The default config is:");
        let mut config = Config {
            executor: HashMap::new(),
        };
        config
            .executor
            .insert("<script type>".into(), "/Path/to/executor".into());//中插入一个键值对，键为 "<script type>"，值为 "/Path/to/executor"。
        println!("{}", toml::to_string_pretty(&config).unwrap());
        panic!("Config file not found!")
    }
    let mut config_file = File::open(&config_file)?;//文件路进
    let mut config = String::new();
    config_file.read_to_string(&mut config)?;
    drop(config_file);
    let config: Config = toml::from_str(&config).context("Can't parse config")?;//toml::from_str()函数将读取到的配置文件内容（TOML格式）解析为Config结构体的实例。这里Config是一个自定义的Rust结构体，用来反序列化配置文件的内容
    let rt = tokio::runtime::Builder::new_multi_thread()//建一个多线程的Tokio异步运行时环境
        .enable_all()
        .worker_threads(4)
        .build()?;
    let cmd = Command::new(//使用Command::new()创建一个新的命令，命令的路径是从命令行参数中提供的执行器路径或配置文件中查找到的对应脚本类型的执行器路径
        opt.executor
            .as_ref()//尝试从命令行参数获取先
            .or(config.executor.get(&resource.spec.script_type.to_string()))//命令的路径是从命令行参数中提供的执行器路径或配置文件中查找到的对应脚本类型的执行器路径。
            .context("Can't find executor")?,
    );
    rt.block_on(async { run(resource, cmd).await })?;
    Ok(())
}

async fn prepare(resource: Script) -> Result<HostMessage> {//是准备和构造一个代表脚本运行请求的 HostMessage
    let client = Client::try_default().await?;//创建一个 Kubernetes 客户端
    let ns = resource.meta().namespace.as_deref().unwrap_or("default");//确定命名空间和资源名称
    let resource_name = resource.meta().name.clone().unwrap();
    let devices = Api::<Device>::namespaced(client, ns);//用 kube 库的 Api 类型创建一个针对特定命名空间的 Device 资源的 API 接口
    let read = query_device_from_server(&devices, &resource.spec.read_selector).await?;
    let manifest = resource.spec.manifest;// 通过 query_device_from_server 函数查询满足特定选择器条件的设备。这个选择器定义在 Script 对象的 spec.read_selector 字段中。
    let devices = read.into_iter().map(|d| d.to_api()).collect();//将查询到的设备列表转换成 API 对象列表。这里假设每个设备都实现了一个 to_api 方法，用于转换成对应的 API 对象。
    let msg = HostMessage::ScriptRun(Box::new(ScriptRun {// 创建一个 HostMessage::ScriptRun 枚举变量，它包含了一个 ScriptRun 结构体的实例。ScriptRun 包含了会话 ID、资源名称、清单和设备列表。
        session_id: Default::default(),
        resource_name,
        manifest,
        devices,
    }));
    Ok(msg)
}

pub async fn letency_handler(
    mut rx: Receiver<(ExecutorID, Duration)>,
    m: HostsMap,
    mut state: watch::Receiver<ControllerState>,//一个监听控制器状态变化的接收器
) -> Result<()> {
    controller::wait_for_init(&mut state).await;//等待初始化
    loop {// 创建一个无限循环，内部使用 tokio::select! 宏来同时监听来自延迟信息的接收器 rx 和控制器状态变化的接收器 state
        tokio::select! {
            Some((id, letency)) = rx.recv() => {//监听来自延迟信息的接收器 rx 和控制器状态变化的接收器 state
                println!(
                    "[`{:?}` executror `{:?}`] Letency: {:?}",
                    m.get_types(id).unwrap(),
                    id,
                    letency
                );
            }
            _ = state.changed() => {
                break
            }
        }
    }
    println!("letency_handler stop");
    Ok(())
}

pub async fn script_status_handler(//主要用于处理脚本状态更新
    mut rx: Receiver<(ExecutorID, ScriptStatus)>,
    m: HostsMap,//用于映射执行器ID到类型和管理执行器通信
    mut state: watch::Receiver<ControllerState>,
) -> Result<()> {
    controller::wait_for_init(&mut state).await;
    if let Some((id, msg)) = rx.recv().await {
        println!(
            "[`{:?}` executror `{:?}`] script status: {:?}, time: {:?} ms",
            m.get_types(id).unwrap(),//意味着如果找不到执行器 ID 对应的类型，程序将会 panic
            id,
            msg,
            msg.duration
        );
        m.send_by_id(id, HostMessage::Disconnect).await.unwrap();//为每个更新请求构造一个 DeviceResponse 消息，并通过 m.send_by_id 方法发送给相应的执行器
    }
    Ok(())
}

pub async fn update_device_handler(
    mut rx: Receiver<(ExecutorID, api::protocol::UpdateDevice)>,
    m: HostsMap,
    mut state: watch::Receiver<ControllerState>,
) -> Result<()> {
    controller::wait_for_init(&mut state).await;
    loop {
        tokio::select! {
            Some((id, msg)) = rx.recv() => {
                println!(
                    "[`{:?}` executror `{:?}`] update device: {:?}",
                    m.get_types(id).unwrap(),
                    id,
                    msg
                );
                let msg = api::protocol::DeviceResponse {//构造一个 api::protocol::DeviceResponse 消息，包含会话ID和设备状态
                    session_id: msg.session_id,
                    status: DeviceStatus::Mock,
                };
                m.send_by_id(id, HostMessage::DeviceResponse(Box::new(msg)))
                    .await
                    .unwrap();
            }
            _ = state.changed() => {
                break
            }
        }
    }
    println!("update_device_handler stop");
    Ok(())
}

async fn run(resource: Script, cmd: Command) -> Result<()> {
    let msg = prepare(resource).await?;
    let sess = IOSession::new(cmd, api::manifest::ScriptType::Js.into())?;//使用 prepare 函数准备一个消息，然后创建一个 IOSession 来执行某个命令。
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (tx2, rx2) = tokio::sync::oneshot::channel();//创建两个 oneshot 通道 tx/rx 和 tx2/rx2。这些通道用于在任务之间发送单次消息。
    let mut controller = ControllerBuilder::new()
        .spawn_letency(letency_handler)//构建一个控制器，并配置它以并发运行设备更新处理器、脚本状态处理器等
        .spawn_script_status(|rx, h, state| async move {
            script_status_handler(rx, h, state).await?;
            tx.send(()).expect("oneshot failed");
            Ok(())
        })
        .spawn_update_device(update_device_handler)
        .spwan_with_hostsmap(|h, mut state| async move {
            controller::wait_for_init(&mut state).await;
            let host_id = rx2.await?;
            h.send_by_id(host_id, msg).await?;
            Ok(())
        })
        .build();
    let id = controller.spwan_session(Box::new(sess));//启动会话 
    tx2.send(id).unwrap();
    controller.run();
    tokio::select! {
        _ = rx => {}
        _ = tokio::signal::ctrl_c() => {}
    }
    controller.stop();
    for _ in 0..controller.controller_tasks.len() {//使用 tokio::task::yield_now().await 暂时让出任务，确保所有控制器任务有机会完成。
        tokio::task::yield_now().await
    }
    controller.kill_all();
    Ok(())
}

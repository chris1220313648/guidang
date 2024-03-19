use api::{protocol::*, ExecutorID};
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
    executor: HashMap<String, PathBuf>,
}

#[derive(Parser)]
struct Args {
    /// path to resource yaml/json file
    resource: PathBuf,

    /// path to executor
    #[clap(short, long)]
    executor: Option<PathBuf>,

    #[clap(short, long)]
    /// path to config file
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Args::parse();
    color_eyre::install()?;
    let mut resource_file = File::open(&opt.resource)?;
    let mut resource = String::new();
    resource_file.read_to_string(&mut resource)?;
    drop(resource_file);
    let resource: Script = match opt.resource.extension() {
        Some(e) if e == "json" => {
            serde_json::from_str(&resource).context("Parse resource file as json")?
        }
        Some(e) if e == "yaml" || e == "yml" => {
            serde_yaml::from_str(&resource).context("Parse resource file as yaml")?
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
    let config_file = if let Some(config) = opt.config.as_ref() {
        config
    } else {
        Path::new(DEFAULT_CONFIG)
    };
    if !config_file.exists() || !config_file.is_file() {
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
            .insert("<script type>".into(), "/Path/to/executor".into());
        println!("{}", toml::to_string_pretty(&config).unwrap());
        panic!("Config file not found!")
    }
    let mut config_file = File::open(&config_file)?;
    let mut config = String::new();
    config_file.read_to_string(&mut config)?;
    drop(config_file);
    let config: Config = toml::from_str(&config).context("Can't parse config")?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .build()?;
    let cmd = Command::new(
        opt.executor
            .as_ref()
            .or(config.executor.get(&resource.spec.script_type.to_string()))
            .context("Can't find executor")?,
    );
    rt.block_on(async { run(resource, cmd).await })?;
    Ok(())
}

async fn prepare(resource: Script) -> Result<HostMessage> {
    let client = Client::try_default().await?;
    let ns = resource.meta().namespace.as_deref().unwrap_or("default");
    let resource_name = resource.meta().name.clone().unwrap();
    let devices = Api::<Device>::namespaced(client, ns);
    let read = query_device_from_server(&devices, &resource.spec.read_selector).await?;
    let manifest = resource.spec.manifest;
    let devices = read.into_iter().map(|d| d.to_api()).collect();
    let msg = HostMessage::ScriptRun(Box::new(ScriptRun {
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
    mut state: watch::Receiver<ControllerState>,
) -> Result<()> {
    controller::wait_for_init(&mut state).await;
    loop {
        tokio::select! {
            Some((id, letency)) = rx.recv() => {
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

pub async fn script_status_handler(
    mut rx: Receiver<(ExecutorID, ScriptStatus)>,
    m: HostsMap,
    mut state: watch::Receiver<ControllerState>,
) -> Result<()> {
    controller::wait_for_init(&mut state).await;
    if let Some((id, msg)) = rx.recv().await {
        println!(
            "[`{:?}` executror `{:?}`] script status: {:?}, time: {:?} ms",
            m.get_types(id).unwrap(),
            id,
            msg,
            msg.duration
        );
        m.send_by_id(id, HostMessage::Disconnect).await.unwrap();
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
                let msg = api::protocol::DeviceResponse {
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
    let sess = IOSession::new(cmd, api::manifest::ScriptType::Js.into())?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (tx2, rx2) = tokio::sync::oneshot::channel();
    let mut controller = ControllerBuilder::new()
        .spawn_letency(letency_handler)
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
    let id = controller.spwan_session(Box::new(sess));
    tx2.send(id).unwrap();
    controller.run();
    tokio::select! {
        _ = rx => {}
        _ = tokio::signal::ctrl_c() => {}
    }
    controller.stop();
    for _ in 0..controller.controller_tasks.len() {
        tokio::task::yield_now().await
    }
    controller.kill_all();
    Ok(())
}

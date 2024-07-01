use std::{path::PathBuf, rc::Rc, thread};

use tracing_subscriber::{filter, prelude::*};

use anyhow::Result;
use clap::Parser;
use deno_executor::{
    loader::{FsLoader, RegisterLoader},
    worker::{DenoWorker, GlobalOption},
};
use executor::Client;
use proto::controller_service_client::ControllerServiceClient;
use tracing::{info, Level};
#[derive(Debug, Parser)]
struct Args {
    /// Set the default register
    #[clap(
        short,
        long,
        default_value = "/Users/han/Project/rule_engine/config/new_register"
    )]
    register: String,
    server: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::registry()
        //.with(console_subscriber::spawn())
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_thread_names(true),
        )
        .with(
            filter::Targets::new()
                .with_default(Level::INFO)
                .with_target("executor", Level::TRACE)
                .with_target("deno_executor", Level::TRACE),
        )
        .init();

    let global_option = GlobalOption {
        default_register: args.register,
        module_loader: RegisterLoader::new(),
    };
    let url = args.server;
    let Client {
        client,
        tasks,
        id,
        rx,
    } = Client::try_connect(url.clone()).await.unwrap();
    loop {
        let run = rx.recv_async().await.unwrap();
        let global = global_option.clone();
        let url = url.clone();
        info!("New script to run: {:?}", run.manifest);
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .thread_name(format!(
                    "ruleengine-{}",
                    &run.manifest.as_ref().unwrap().package_name
                ))
                .build()
                .unwrap();
            rt.block_on(async move {
                let client = ControllerServiceClient::connect(url).await.unwrap();
                let worker = DenoWorker::new(run, global, client);
                worker.run().await;
            });
        });
    }
    Ok(())
}

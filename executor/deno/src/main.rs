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
    #[clap(//register字段使用了clap的short, long, 和 default_value属性来定义其命令行参数的短格式、长格式和默认值。
        short,
        long,
        default_value = "/root/guidang/config/register"
    )]
    register: String,//注册地址
    server: String,//服务器地址gprc

}

#[tokio::main]//使用[tokio::main]属性宏将main函数标记为异步入口点 这允许在函数内部使用.await。
async fn main() -> Result<()> {
    let args = Args::parse();//通过Args::parse()调用clap库来解析命令行参数，Args结构体定义了命令行接受的参数。

    tracing_subscriber::registry()//使用tracing_subscriber配置应用的日志记录
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
//创建GlobalOption实例，包含默认注册路径和模块加载器
    let global_option = GlobalOption {
        default_register: args.register,//文件服务器
        module_loader: RegisterLoader::new(),
    };
    //使用自定义的Client类型尝试连接到指定的grpc服务器URL，并解构返回的结构以获取客户端实例和其他相关字段。
    let url = args.server;
    let Client {
        client,
        tasks,
        id,
        rx,
    } = Client::try_connect(url.clone()).await.unwrap();
    loop {//解构返回的结构以获取客户端实例和其他相关字段。
        let run = rx.recv_async().await.unwrap();//从接收器(rx)异步接收任务消息。每个run包含了任务的相关信息，如要执行的脚本的清单（manifest）。
        let global = global_option.clone();
        let url = url.clone();
        info!("New script to run: {:?}", run.manifest);
        thread::spawn(move || {//为新脚本任务创建新线程
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .thread_name(format!(
                    "ruleengine-{}",
                    &run.manifest.as_ref().unwrap().package_name
                ))//配置并构建新的Tokio运行时，为其设置线程名称以便于识别
                .build()
                .unwrap();
            rt.block_on(async move {//首先异步连接到ControllerServiceClient（可能是用于任务管理的服务），然后创建一个DenoWorker实例来处理任务。
                let client = ControllerServiceClient::connect(url).await.unwrap();
                let worker: DenoWorker = DenoWorker::new(run, global, client);
                worker.run().await;
            });//当调用rt.block_on时，它接收一个Future（通常是一个async块）作为参数，然后当前线程会停在这里等待，直到这个Future被执行完成。完成可以是成功的返回值，也可以是发生的错误
        });
    }
    Ok(())
}

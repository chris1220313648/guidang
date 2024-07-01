use clap::Parser;//通过派生 `Parser` 特质自动实现参数解析
use color_eyre::{Report, Result};
use tracing::Level;//tracing 是一个异步友好的日志记录和跟踪框架
use tracing_subscriber::{filter, prelude::*};

#[derive(Parser)]//可以直接使用Parser特质
struct Args {
    //web：定义了一个名为 web 的命令行参数，通过 -w 短标记指定，如果未指定，则默认值为 "0.0.0.0:8000"。
    #[clap(short, default_value = "127.0.0.1:8000")]
    web: String,
    #[clap(short, default_value = "0.0.0.0:8001")]
    grpc: String,
    #[clap(short, default_value = "127.0.0.1:1883")]
    mqtt: String,
}

fn main() -> Result<()> {
    let opt = Args::parse();//解析命令行参数 并填充 Args 结构体的实例
    color_eyre::install()?;//调用 color_eyre::install() 会设置全局错误报告钩子，当使用 ? 操作符或 unwrap 在错误上时，color_eyre 会提供详细的错误报告。
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_thread_names(true),//包含线程名
        )
        .with(
            filter::Targets::new()
                .with_default(Level::INFO)
                .with_target("controller", Level::TRACE),
        )
        .init();
    let config = controller::controller::Config {
        webaddr: opt.web.parse()?,//这里的 parse() 方法尝试将字符串参数转换为适当的类型，可能是网络地址。
        grpcaddr: opt.grpc.parse()?,
        mqttaddr: opt.mqtt.parse()?,
    };

    let rt = tokio::runtime::Builder::new_multi_thread()// 创建一个多线程的异步运行时，启用运行时的所有特性
        .enable_all()
        .build()//构建运行时
        .unwrap();
    rt.block_on(async move {//启动异步任务
        let mut ctl = controller::controller::Controller::new(config)?;//创建控制器实例并传入解析
        let client = kube::Client::try_default().await?;//尝试创建一个 Kubernetes 客户端。这个客户端用于与 Kubernetes 集群交互。
        let (schin, _schdevin, schout, store) = ctl.spawn_kubeapi(client.clone(), true);//启动与 Kubernetes API 交互的相关功能
        ctl.spawn_webserver(schin, store);//启动web服务器 接受脚本发送器 reflector结构体
        ctl.spawn_grpc(client, schout);
        ctl.run().await?;//调用 Controller::run 方法来启动控制器的主要运行循环。这个循环可能会处理来自 Kubernetes 的事件、响应 Web 或 gRPC 请求等。
        Ok::<_, Report>(())
    })?;
    Ok(())
}

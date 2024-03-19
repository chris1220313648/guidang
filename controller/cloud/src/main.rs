use clap::Parser;
use color_eyre::{Report, Result};
use tracing::Level;
use tracing_subscriber::{filter, prelude::*};

#[derive(Parser)]
struct Args {
    #[clap(short, default_value = "0.0.0.0:8000")]
    web: String,
    #[clap(short, default_value = "0.0.0.0:8001")]
    grpc: String,
    #[clap(short, default_value = "127.0.0.1:1883")]
    mqtt: String,
}

fn main() -> Result<()> {
    let opt = Args::parse();
    color_eyre::install()?;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_thread_names(true),
        )
        .with(
            filter::Targets::new()
                .with_default(Level::INFO)
                .with_target("controller", Level::TRACE),
        )
        .init();
    let config = controller::controller::Config {
        webaddr: opt.web.parse()?,
        grpcaddr: opt.grpc.parse()?,
        mqttaddr: opt.mqtt.parse()?,
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async move {
        let mut ctl = controller::controller::Controller::new(config)?;
        let client = kube::Client::try_default().await?;
        let (schin, _schdevin, schout, store) = ctl.spawn_kubeapi(client.clone(), true);
        ctl.spawn_webserver(schin, store);
        ctl.spawn_grpc(client, schout);
        ctl.run().await?;
        Ok::<_, Report>(())
    })?;
    Ok(())
}

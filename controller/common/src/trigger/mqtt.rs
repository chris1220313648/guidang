use std::marker::PhantomData;
use std::sync::Arc;

use crate::api::{
    mqtt::{DEVICE_ETPREFIX, TWIN_ETUPDATE_RESULT_SUFFIX},
    Device,
};
use crate::scheduler::ResourceIndex;
use color_eyre::{eyre::eyre, Result};
use flume::Sender;
use once_cell::sync::Lazy;
use regex::Regex;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, Publish, QoS};
use tracing::{error, info, log::trace};

pub type AsyncHook = Sender<Arc<Publish>>;
pub type SyncHook = Box<dyn FnMut(&Publish) -> Result<()> + Sync + Send + 'static>;

#[tracing::instrument(skip_all)]
pub async fn mqtt_client(
    host: String,
    port: u16,
    mut async_hooks: Vec<AsyncHook>,
    mut sync_hooks: Vec<SyncHook>,
) -> Result<()> {
    info!("Connect to MQTT broker {}:{}", host, port);
    let options = MqttOptions::new("ruleengine", host, port);
    let (client, mut eventloop) = AsyncClient::new(options, 20);
    // The only possible first package is
    let connack = eventloop.poll().await?;
    assert!(matches!(connack, Event::Incoming(Packet::ConnAck(_))));
    // We should never use Qos 2: ExactlyOnce
    client
        .subscribe(
            format!("{}+{}", DEVICE_ETPREFIX, TWIN_ETUPDATE_RESULT_SUFFIX),
            QoS::AtMostOnce,
        )
        .await?;
    loop {
        if let Event::Incoming(p) = eventloop.poll().await? {
            let publish = match p {
                Packet::Publish(i) => i,
                Packet::PingResp
                | Packet::PubAck(_)
                | Packet::PubRec(_)
                | Packet::PubRel(_)
                | Packet::PubComp(_)
                | Packet::SubAck(_) => {
                    trace!("{p:?}");
                    continue;
                }
                _ => return Err(eyre!("Unexpected MQTT packet {:?}", p)),
            };
            let ae = Arc::new(publish.clone());
            for tx in &mut async_hooks {
                if let Err(e) = tx.send(ae.clone()) {
                    error!(error =? e, "MQTTWatcher async hook throw a error")
                }
            }
            for hook in &mut sync_hooks {
                if let Err(e) = hook(&publish) {
                    error!(error =? e, "MQTTWatcher sync hook throw a error")
                }
            }
        }
    }
}

const DEVICE_UPDATE_RESULT_REGEX: Lazy<Regex> = Lazy::new(|| {
    // !!! \\$ !!!
    Regex::new(&format!(
        "\\{}([a-zA-Z0-9-_]+){}",
        DEVICE_ETPREFIX, TWIN_ETUPDATE_RESULT_SUFFIX
    ))
    .unwrap()
});

pub fn trigger_hook(scheduler: Sender<ResourceIndex<Device>>) -> SyncHook {
    let triger = move |msg: &Publish| {
        let name = match DEVICE_UPDATE_RESULT_REGEX.captures(&msg.topic) {
            Some(cap) => cap[1].to_owned(),
            None => return Ok::<_, color_eyre::Report>(()),
        };
        scheduler
            .send(ResourceIndex {
                namespace: "default".to_owned(), // FIXME: where is namespace ???
                name,
                api: PhantomData,
            })
            .map_err(|_| eyre!("Scheduler is down!"))
    };
    Box::new(triger)
}

pub fn logger_hook() -> SyncHook {
    let logger = |msg: &Publish| {
        tracing::info!(msg =?msg, "MQTT Publish");
        Ok(())
    };
    Box::new(logger)
}

#[cfg(test)]
mod test {
    use super::*;

    const HOST: &str = "127.0.0.1";
    const PORT: u16 = 1883;

    #[tokio::test]
    async fn test_mqtt() {
        const DEVICE_NAME: &str = "test_name";
        const DEVICE_NAMESPACE: &str = "default";

        let (tx, rx) = flume::bounded(3);
        let sync_hooks = vec![trigger_hook(tx), logger_hook()];
        let async_hooks = Vec::new();
        tokio::spawn(
            async move { mqtt_client(HOST.to_owned(), PORT, async_hooks, sync_hooks).await },
        );

        let options = MqttOptions::new("test_mqtt", HOST, PORT);
        let (client, mut eventloop) = AsyncClient::new(options, 3);
        tokio::spawn(async move {
            loop {
                eventloop.poll().await.unwrap();
            }
        });
        client
            .publish(
                format!("{DEVICE_ETPREFIX}{DEVICE_NAME}{TWIN_ETUPDATE_RESULT_SUFFIX}"),
                QoS::AtMostOnce,
                false,
                "",
            )
            .await
            .unwrap();

        let ri = rx.recv_async().await.unwrap();
        assert_eq!(ri.name, DEVICE_NAME);
        assert_eq!(ri.namespace, DEVICE_NAMESPACE);
    }
}

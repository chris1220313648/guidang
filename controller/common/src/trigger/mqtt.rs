use std::marker::PhantomData;
use std::sync::Arc;//Arc 是 Rust 的一个原子引用计数类型，允许多个所有者拥有同一个数据，实现数据的线程安全共享

use crate::api::{//这行代码从当前包的api模块中引入了Device类型以及mqtt子模块中的DEVICE_ETPREFIX和TWIN_ETUPDATE_RESULT_SUFFIX常量。
    mqtt::{DEVICE_ETPREFIX, TWIN_ETUPDATE_RESULT_SUFFIX},
    Device,
};
use crate::scheduler::ResourceIndex;
use color_eyre::{eyre::eyre, Result};
use flume::Sender;
use once_cell::sync::Lazy;//Lazy用于静态或动态变量的延迟初始化，保证线程安全
use regex::Regex;//Regex类型用于创建和操作正则表达式。
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, Publish, QoS};
//引入了rumqttc库中的AsyncClient（一个异步MQTT客户端）、Event（事件类型）、MqttOptions（MQTT配置选项）、Packet（MQTT数据包）、Publish（发布消息的类型）和QoS（消息服务质量等级）
use tracing::{error, info, log::trace};

pub type AsyncHook = Sender<Arc<Publish>>;//定义了一个类型别名AsyncHook，它是一个发送端，可以发送包裹在Arc（原子引用计数智能指针）中的Publish类型的消息。
pub type SyncHook = Box<dyn FnMut(&Publish) -> Result<()> + Sync + Send + 'static>;
//定义了一个类型别名SyncHook，它是一个装箱的动态闭包，这个闭包可以修改地接收一个Publish类型的引用，并返回一个Result<()>类型的结果。该闭包可以跨线程安全地发送（Send）、同步访问（Sync），并且具有'static生命周期。
#[tracing::instrument(skip_all)]
pub async fn mqtt_client(
    host: String,
    port: u16,
    mut async_hooks: Vec<AsyncHook>,
    mut sync_hooks: Vec<SyncHook>,
) -> Result<()> {
    info!("Connect to MQTT broker {}:{}", host, port);
    let options = MqttOptions::new("ruleengine", host, port);//创建客户端连接选项
    let (client, mut eventloop) = AsyncClient::new(options, 20);//创建客户端和队列容量
    // The only possible first package is
    let connack = eventloop.poll().await?;//获取第一个事件包
    assert!(matches!(connack, Event::Incoming(Packet::ConnAck(_))));//检查连接确认包
    // We should never use Qos 2: ExactlyOnce
    client
        .subscribe(
            format!("{}+{}", DEVICE_ETPREFIX, TWIN_ETUPDATE_RESULT_SUFFIX),
            QoS::AtMostOnce,
        )
        .await?;//订阅主题
    loop {
        if let Event::Incoming(p) = eventloop.poll().await? {
            let publish = match p {//匹配包类型
                Packet::Publish(i) => i,//发布类型的包
                Packet::PingResp
                | Packet::PubAck(_)
                | Packet::PubRec(_)
                | Packet::PubRel(_)
                | Packet::PubComp(_)
                | Packet::SubAck(_) => {
                    trace!("{p:?}");
                    continue;//记录日志 继续循环
                }
                _ => return Err(eyre!("Unexpected MQTT packet {:?}", p)),
            };
            let ae = Arc::new(publish.clone());
            for tx in &mut async_hooks {
                if let Err(e) = tx.send(ae.clone()) {//异步钩子发送所以包
                    error!(error =? e, "MQTTWatcher async hook throw a error")
                }
            }
            for hook in &mut sync_hooks {
                if let Err(e) = hook(&publish) {//同步钩子处理发布消息
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
    let triger = move |msg: &Publish| {//move: 将闭包中的所有权从外部捕获到闭包内。这个闭包需要移动捕获的变量，以便在闭包内使用。
        let name = match DEVICE_UPDATE_RESULT_REGEX.captures(&msg.topic) {
            Some(cap) => cap[1].to_owned(),//匹配成功 提取第一个捕获组中的设备名称
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

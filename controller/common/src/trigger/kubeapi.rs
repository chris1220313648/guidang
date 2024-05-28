use crate::{
    api::Device,
    api::Script,
    scheduler::{Reflector, ResourceIndex},
};
use color_eyre::{eyre::eyre, Report, Result};
use flume::{Receiver, Sender};
use futures::StreamExt;
use kube::{api::ListParams, Api, Resource};
use kube_runtime::watcher::{watcher, Event};
use serde::de::DeserializeOwned;
use std::{fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};
use tracing::error;

pub type AsyncHook<K> = Sender<Arc<Event<K>>>;//异步地发送包含某种类型事件的消息。
pub type SyncHook<K> = Box<dyn FnMut(&Event<K>) -> Result<()> + Sync + Send + 'static>;
//类型别名用于同步事件处理，允许你同步地处理某种类型的事件，并且能够在闭包内部进行状态的修改。
#[tracing::instrument(skip_all)]
pub async fn reflector<K>(
    api: Api<K>,
    list_params: ListParams,//获取资源列表的参数
    mut async_hooks: Vec<AsyncHook<K>>,//异步狗子
    mut sync_hooks: Vec<SyncHook<K>>,//同步
) -> Result<()>
where
    K: Resource + 'static + Clone + Debug + Send + DeserializeOwned,//定义了K的约束条件
    K::DynamicType: Eq + Hash + Clone + Default,
{
    let mut watcher = watcher(api, list_params).boxed();//使用watcher异步监听资源变化
    loop {
        if let Some(ev) = watcher.next().await {//检查是否有新的事件
            match ev {
                Ok(e) => {
                    if !async_hooks.is_empty() {
                        let ae = Arc::new(e.clone());//创建事件的引用计数副本
                        for tx in &mut async_hooks {//并将其发送到所有异步钩子。之后，遍历同步钩子，直接将事件传递给它们处理
                            if let Err(e) = tx.send(ae.clone()) {
                                error!(error =% e, "Reflector async hook throw a error")
                            }//错误时（Err(e)），记录一条错误日志并通过break Err(Report::from(e))退出循环，返回错误。
                        }
                    }
                    for hook in &mut sync_hooks {
                        if let Err(e) = hook(&e) {
                            error!(error =? e, "Reflector sync hook throw a error")
                        }
                    }
                }
                Err(e) => {
                    error!(error =? e, "Reflector throw a fatal error");
                    break Err(Report::from(e));
                }
            }
        } else {
            break Err(eyre!("Reflector stop unexpected!"));
        }
    }
}

pub fn logger_hook<K>() -> SyncHook<K>
where
    K: Resource + 'static + Clone + Debug + Send + Sync + DeserializeOwned,
    K::DynamicType: Eq + Hash + Clone + Default,
{
    let logger = |ev: &Event<K>| {//闭包logger接受一个对Event<K>的引用，并根据事件类型执行不同的日志记录操作。
        use kube::ResourceExt;
        match ev {
            Event::Applied(a) => {//对于应用（创建或更新）的事件，记录资源的名称和命名空间，并标记为"KubeAPI applied"
                tracing::trace!(name =? a.name(), ns =? a.namespace(), "KubeAPI applied")
            }
            Event::Deleted(d) => {
                tracing::trace!(name =? d.name(), ns =? d.namespace(), "KubeAPI deleted")
            }
            Event::Restarted(_) => tracing::trace!("Restart"),
        }
        Ok(())
    };
    Box::new(logger)
}

#[tracing::instrument(skip_all)]
pub async fn trigger_hook(
    rx: Receiver<Arc<Event<Device>>>,//接受设备事件
    scheduler: Sender<ResourceIndex<Device>>,//设备索引发送到调度器
) -> Result<()> {
    let dev_to_idx = |dev: &Device| ResourceIndex {
        //定义了一个闭包dev_to_idx，它接受一个&Device引用作为参数，并返回一个ResourceIndex<Device>结构
        name: dev.meta().name.clone().unwrap(),
        namespace: dev.meta().namespace.clone().unwrap(),
        api: PhantomData,//用于表明这个结构体泛型地依赖于Device类
    };
    loop {
        let ev = rx.recv_async().await?;//通过调用rx.recv_async().await?，它异步等待并接收新的事件。
        match ev.as_ref() {
            Event::Applied(dev) => {
                let idx = dev_to_idx(dev);//Device转换为ResourceIndex，并通过scheduler异步发送这个索引。
                tracing::trace!(dev = ?dev, "Got new device applied");
                scheduler.send_async(idx).await?
            }
            Event::Restarted(devs) => {
                for dev in devs {
                    tracing::trace!(dev = ?dev, "Got new device restarted");
                    scheduler.send_async(dev_to_idx(dev)).await?
                }
            }
            Event::Deleted(_) => {}
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn device_hook(//同步设备
    rx: Receiver<Arc<Event<Device>>>,
    reflector: Arc<Reflector>,//一个Reflector对象的原子引用计数（Arc）指针，Reflector是一个假定存在的类型，负责管理设备的状态和生命周期。
) -> Result<()> {
    loop {
        let dev = rx.recv_async().await?;
        match dev.as_ref() {
            Event::Applied(dev) => reflector.add_device(dev),
            Event::Restarted(devs) => reflector.restart_device(devs),
            Event::Deleted(dev) => reflector.remove_device(dev),
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn script_hook(//同步脚本
    rx: Receiver<Arc<Event<Script>>>,
    reflector: Arc<Reflector>,//一个Reflector对象的原子引用计数（Arc）指针，Reflector是一个假定存在的类型，负责管理设备的状态和生命周期。
) -> Result<()> {
    loop {
        let dev = rx.recv_async().await?;
        match dev.as_ref() {
            Event::Applied(script) => reflector.add_script(script),
            Event::Restarted(scripts) => reflector.restart_script(scripts),
            Event::Deleted(script) => reflector.remove_script(script),
        }
    }
}

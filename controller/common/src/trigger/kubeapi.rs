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

pub type AsyncHook<K> = Sender<Arc<Event<K>>>;
pub type SyncHook<K> = Box<dyn FnMut(&Event<K>) -> Result<()> + Sync + Send + 'static>;

#[tracing::instrument(skip_all)]
pub async fn reflector<K>(
    api: Api<K>,
    list_params: ListParams,
    mut async_hooks: Vec<AsyncHook<K>>,
    mut sync_hooks: Vec<SyncHook<K>>,
) -> Result<()>
where
    K: Resource + 'static + Clone + Debug + Send + DeserializeOwned,
    K::DynamicType: Eq + Hash + Clone + Default,
{
    let mut watcher = watcher(api, list_params).boxed();
    loop {
        if let Some(ev) = watcher.next().await {
            match ev {
                Ok(e) => {
                    if !async_hooks.is_empty() {
                        let ae = Arc::new(e.clone());
                        for tx in &mut async_hooks {
                            if let Err(e) = tx.send(ae.clone()) {
                                error!(error =% e, "Reflector async hook throw a error")
                            }
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
    let logger = |ev: &Event<K>| {
        use kube::ResourceExt;
        match ev {
            Event::Applied(a) => {
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
    rx: Receiver<Arc<Event<Device>>>,
    scheduler: Sender<ResourceIndex<Device>>,
) -> Result<()> {
    let dev_to_idx = |dev: &Device| ResourceIndex {
        name: dev.meta().name.clone().unwrap(),
        namespace: dev.meta().namespace.clone().unwrap(),
        api: PhantomData,
    };
    loop {
        let ev = rx.recv_async().await?;
        match ev.as_ref() {
            Event::Applied(dev) => {
                let idx = dev_to_idx(dev);
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
pub async fn device_hook(
    rx: Receiver<Arc<Event<Device>>>,
    reflector: Arc<Reflector>,
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
pub async fn script_hook(
    rx: Receiver<Arc<Event<Script>>>,
    reflector: Arc<Reflector>,
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

use crate::api::{Device, Script};
use crate::scheduler::{Reflector, ResourceIndex};
use color_eyre::{eyre::eyre, Report, Result};
use flume::{Receiver, Sender};
use rusqlite::{Connection, Result as SqlResult};
use serde::de::DeserializeOwned;
use std::{fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};
use tracing::error;
use tokio::time::{interval, Duration};

// 定义数据库事件类型
#[derive(Debug, Clone)]
pub enum DbEvent<K> {
    Inserted(K),
    Updated(K),
    Deleted(K),
}

pub type AsyncHook<K> = Sender<Arc<DbEvent<K>>>; // 异步地发送包含某种类型事件的消息
pub type SyncHook<K> = Box<dyn FnMut(&DbEvent<K>) -> Result<()> + Sync + Send + 'static>; // 同步事件处理
#[tracing::instrument(skip_all)]
pub async fn reflector<K>(
    conn: Connection,
    query: &str, // 获取资源列表的查询
    mut async_hooks: Vec<AsyncHook<K>>,
    mut sync_hooks: Vec<SyncHook<K>>,
    extract_key: fn(&K) -> String,
) -> Result<()>
where
    K: 'static + Clone + Debug + Send + DeserializeOwned,
{
    let mut previous_data: HashMap<String, K> = HashMap::new();

    loop {
        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([], |row| {
            Ok(K::from_row(row)?)
        })?;

        let mut current_data: HashMap<String, K> = HashMap::new();
        for result in rows {
            let item = result?;
            let key = extract_key(&item);
            current_data.insert(key.clone(), item.clone());

            // 检查插入和更新事件
            if let Some(old_item) = previous_data.get(&key) {
                if old_item != &item {
                    // 生成更新事件
                    let event = DbEvent::Updated(item.clone());
                    handle_event(&event, &mut async_hooks, &mut sync_hooks).await?;
                }
            } else {
                // 生成插入事件
                let event = DbEvent::Inserted(item.clone());
                handle_event(&event, &mut async_hooks, &mut sync_hooks).await?;
            }
        }

        // 检查删除事件
        for (key, old_item) in previous_data.iter() {
            if !current_data.contains_key(key) {
                // 生成删除事件
                let event = DbEvent::Deleted(old_item.clone());
                handle_event(&event, &mut async_hooks, &mut sync_hooks).await?;
            }
        }

        previous_data = current_data;
        interval(Duration::from_secs(10)).await;
    }
}

async fn handle_event<K>(
    event: &DbEvent<K>,
    async_hooks: &mut Vec<AsyncHook<K>>,
    sync_hooks: &mut Vec<SyncHook<K>>,
) -> Result<()>
where
    K: 'static + Clone + Debug + Send + DeserializeOwned,
{
    if !async_hooks.is_empty() {
        let ae = Arc::new(event.clone());
        for tx in async_hooks.iter_mut() {
            if let Err(e) = tx.send(ae.clone()) {
                error!(error =% e, "Reflector async hook throw a error")
            }
        }
    }

    for hook in sync_hooks.iter_mut() {
        if let Err(e) = hook(event) {
            error!(error =? e, "Reflector sync hook throw a error")
        }
    }

    Ok(())
}
pub fn logger_hook<K>() -> SyncHook<K>
where
    K: 'static + Clone + Debug + Send + Sync + DeserializeOwned,
{
    let logger = |ev: &DbEvent<K>| {
        match ev {
            DbEvent::Inserted(a) => {
                tracing::trace!(item =? a, "DB item inserted")
            }
            DbEvent::Updated(a) => {
                tracing::trace!(item =? a, "DB item updated")
            }
            DbEvent::Deleted(a) => {
                tracing::trace!(item =? a, "DB item deleted")
            }
        }
        Ok(())
    };
    Box::new(logger)
}
#[tracing::instrument(skip_all)]
pub async fn trigger_hook(
    rx: Receiver<Arc<DbEvent<Device>>>,
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
            DbEvent::Inserted(dev) | DbEvent::Updated(dev) => {
                let idx = dev_to_idx(dev);
                tracing::trace!(dev = ?dev, "Got new device inserted/updated");
                scheduler.send_async(idx).await?
            }
            DbEvent::Deleted(_) => {}
        }
    }
}
#[tracing::instrument(skip_all)]
pub async fn device_hook(
    rx: Receiver<Arc<DbEvent<Device>>>,
    reflector: Arc<Reflector>,
) -> Result<()> {
    loop {
        let dev = rx.recv_async().await?;
        match dev.as_ref() {
            DbEvent::Inserted(dev) => reflector.add_device(dev),
            DbEvent::Updated(dev) => reflector.update_device(dev),
            DbEvent::Deleted(dev) => reflector.remove_device(dev),
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn script_hook(
    rx: Receiver<Arc<DbEvent<Script>>>,
    reflector: Arc<Reflector>,
) -> Result<()> {
    loop {
        let script = rx.recv_async().await?;
        match script.as_ref() {
            DbEvent::Inserted(script) => reflector.add_script(script),
            DbEvent::Updated(script) => reflector.update_script(script),
            DbEvent::Deleted(script) => reflector.remove_script(script),
        }
    }
}


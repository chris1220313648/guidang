use crate::api::{Device, Script};
use crate::id::ScriptIDGenerator;
use color_eyre::{eyre::eyre, Result};
use dashmap::{DashMap, DashSet};
use flume::{Receiver, Sender};
use kube::Resource;
use proto::server_message::{
    run_script::{Manifest as ProtoManifest, ReadDevice, WriteDevice},
    RunScript,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;
use tracing::{debug, info, trace};

#[derive(Deserialize, Clone)]
pub struct ResourceIndex<K> {
    pub namespace: String,
    pub name: String,
    #[serde(skip)]
    pub api: PhantomData<K>,
}

impl<K> Hash for ResourceIndex<K> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.namespace.hash(state);
        self.name.hash(state);
    }
}

impl<K> PartialEq for ResourceIndex<K> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.namespace == other.namespace
    }
}

impl<K> Eq for ResourceIndex<K> {}

impl<K> Debug for ResourceIndex<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceIndex")
            .field("namespace", &self.namespace)
            .field("name", &self.name)
            .finish()
    }
}

impl<K> From<&K> for ResourceIndex<K>
where
    K: Resource,
{
    fn from(res: &K) -> Self {
        ResourceIndex {
            namespace: res.meta().namespace.clone().unwrap(),
            name: res.meta().name.clone().unwrap(),
            api: PhantomData,
        }
    }
}

pub trait RunScriptLookup {
    fn lookup_script(&mut self, index: &ResourceIndex<Script>) -> Result<Script>;
    fn lookup_device(&mut self, index: &ResourceIndex<Device>) -> Result<Device>;
    fn map_device_to_script(
        &mut self,
        device: &ResourceIndex<Device>,
    ) -> Result<Vec<ResourceIndex<Script>>>;
    fn lookup_readable(&mut self, script: &Script) -> Result<HashMap<String, ReadDevice>>;
    fn lookup_writable(&mut self, script: &Script) -> Result<HashMap<String, WriteDevice>>;
}

pub struct ManagerMsg {
    pub run: RunScript,
    pub name: String,
    pub namespace: String,
}

pub struct Scheduler<T: RunScriptLookup + Send> {
    lookup_impl: T,
    script_idgen: ScriptIDGenerator,
}

impl<T: RunScriptLookup + Send + 'static> Scheduler<T> {
    pub fn new(lookup: T) -> Self {
        Scheduler {
            lookup_impl: lookup,
            script_idgen: ScriptIDGenerator::default(),
        }
    }
    pub fn lookup(&mut self, index: ResourceIndex<Script>) -> Result<ManagerMsg> {
        trace!(script =? index, "lookup new script");
        let script = self.lookup_impl.lookup_script(&index)?;
        let readable = self.lookup_impl.lookup_readable(&script)?;
        let writable = self.lookup_impl.lookup_writable(&script)?;
        let name = script.meta().name.clone().unwrap();
        let namespace = script.meta().namespace.clone().unwrap();
        let env = script.spec.env;
        let run = RunScript {
            script_id: self.script_idgen.gen().into(),
            manifest: Some(ProtoManifest {
                script_type: script.spec.manifest.script_type as i32,
                package_name: script.spec.manifest.name.clone(),
                package_version: script.spec.manifest.version.clone(),
                register: script.spec.manifest.register.clone().unwrap_or_default(),
            }),
            readable,
            writable,
            env,
            default_qos: script.spec.execute_policy.qos as i32,
        };
        trace!(run =? run, "lookup result");
        Ok(ManagerMsg {
            run,
            name,
            namespace,
        })
    }
}

/// Map of Device index to Script Set
pub type SelectorMap = DashMap<ResourceIndex<Device>, DashSet<ResourceIndex<Script>>>;
pub type Store<K> = DashMap<ResourceIndex<K>, K>;

#[derive(Debug, Clone, Default)]
pub struct Reflector {
    pub selector_map: SelectorMap,
    pub device_store: Store<Device>,
    pub script_store: Store<Script>,
}

impl Reflector {
    pub fn add_device(&self, dev: &Device) {
        let idx = dev.into();
        self.device_store.insert(idx, dev.clone());
    }
    pub fn remove_device(&self, dev: &Device) {
        let idx = dev.into();
        if self.device_store.remove(&idx).is_none() {
            tracing::warn!(device =? dev, "Reflector want to remove nonexsit Device")
        }
    }
    pub fn restart_device(&self, dev: &[Device]) {
        for d in dev {
            self.add_device(d);
        }
    }
    pub fn add_script(&self, script: &Script) {
        let idx: ResourceIndex<Script> = script.into();
        self.script_store.insert(idx.clone(), script.clone());
        let devices = self.get_selected_by_name(script);
        debug!(script =? idx, devices =? devices);
        for dev in devices {
            let set = self.selector_map.get(&dev);
            match set {
                None => {
                    let new_set = DashSet::new();
                    new_set.insert(script.into());
                    self.selector_map.insert(dev, new_set);
                }
                Some(set) => {
                    set.insert(script.into());
                }
            }
        }
    }
    pub fn remove_script(&self, script: &Script) {
        let idx = script.into();
        for set in self.selector_map.iter() {
            set.remove(&idx);
        }
        if self.script_store.remove(&idx).is_none() {
            tracing::warn!(script =? script, "Reflector want to remove nonexsit Script")
        }
    }
    pub fn restart_script(&self, scripts: &[Script]) {
        for s in scripts {
            self.add_script(s);
        }
    }
    fn get_selected_by_name(&self, script: &Script) -> Vec<ResourceIndex<Device>> {
        let mut result = Vec::new();
        let mut idx = ResourceIndex {
            name: String::new(),
            namespace: script.meta().namespace.clone().unwrap(),
            api: PhantomData,
        };
        if let Some(map) = &script.spec.read_selector.match_names {
            for name in map.values() {
                idx.name = name.clone();
                result.push(idx.clone())
            }
        }
        result
    }
}

impl RunScriptLookup for Arc<Reflector> {
    fn lookup_script(&mut self, index: &ResourceIndex<Script>) -> Result<Script> {
        self.script_store
            .get(index)
            .map(|s| s.to_owned())
            .ok_or_else(|| eyre!("Script: {:?} not found in store", index))
    }

    fn lookup_device(&mut self, index: &ResourceIndex<Device>) -> Result<Device> {
        self.device_store
            .get(index)
            .map(|s| s.to_owned())
            .ok_or_else(|| eyre!("Device: {:?} not found in store", index))
    }

    fn map_device_to_script(
        &mut self,
        device: &ResourceIndex<Device>,
    ) -> Result<Vec<ResourceIndex<Script>>> {
        let set = self
            .selector_map
            .get(device)
            .ok_or_else(|| eyre!("Device: {:?} not found in store", device))?
            .iter()
            .map(|m| m.clone())
            .collect();
        Ok(set)
    }

    fn lookup_readable(&mut self, script: &Script) -> Result<HashMap<String, ReadDevice>> {
        let mut result = HashMap::new();
        if let Some(map) = &script.spec.read_selector.match_names {
            for (k, v) in map.iter() {
                let idx = ResourceIndex {
                    namespace: script.meta().namespace.to_owned().unwrap(),
                    name: v.to_string(),
                    api: PhantomData,
                };
                let dev = if let Some(dev) = self.device_store.get(&idx) {
                    dev
                } else {
                    continue;
                };
                let mut status = HashMap::new();
                if let Some(s) = &dev.status {
                    trace!(s=?s);
                    for twin in &s.twins {
                        if let Some(val) = &twin.reported {
                            status.insert(twin.property_name.to_owned(), val.value.to_owned());
                        }
                    }
                }
                result.insert(
                    k.to_owned(),
                    ReadDevice {
                        name: v.to_owned(),
                        status,
                    },
                );
            }
        }
        Ok(result)
    }
    fn lookup_writable(&mut self, script: &Script) -> Result<HashMap<String, WriteDevice>> {
        let mut result = HashMap::new();
        if let Some(map) = &script.spec.write_selector.match_names {
            for (k, v) in map.iter() {
                result.insert(k.to_owned(), WriteDevice { name: v.to_owned() });
            }
        }
        Ok(result)
    }
}

#[tracing::instrument(skip_all)]
pub async fn trigger(
    store: Arc<Reflector>,
    device: Receiver<ResourceIndex<Device>>,
    script: Sender<ResourceIndex<Script>>,
) -> Result<()> {
    loop {
        let idx = device.recv_async().await?;
        info!(device =? idx, "map trigger got new device");
        if let Some(scripts) = store.selector_map.get(&idx) {
            for s in scripts.iter() {
                info!(script =? *s, "map trigger new script");
                script.send_async(s.clone()).await?;
            }
        }
    }
}

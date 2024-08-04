use crate::api::Device;
use crate::api::Script;
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
    pub api: PhantomData<K>,//PhantomData用来表示ResourceIndex与类型K有关，但不会实际存储K的值。
}

impl<K> Hash for ResourceIndex<K> {//实现哈希特征
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.namespace.hash(state);
        self.name.hash(state);
    }
}

impl<K> PartialEq for ResourceIndex<K> {//比较两个实例是否相等
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.namespace == other.namespace
    }
}

impl<K> Eq for ResourceIndex<K> {}

impl<K> Debug for ResourceIndex<K> {//实现Debug特征以提供格式化的调试输出，包含资源的namespace和name字段
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceIndex")
            .field("namespace", &self.namespace)
            .field("name", &self.name)
            .finish()
    }
}

impl<K> From<&K> for ResourceIndex<K>//ResourceIndex<K>实现从资源类型K到ResourceIndex<K>的转换
where
    K: Resource,
{
    fn from(res: &K) -> Self {//为ResourceIndex<K>实现从资源类型K到ResourceIndex<K>的转换，其中K必须实现Resource特征
        ResourceIndex {
            namespace: res.meta().namespace.clone().unwrap(),
            name: res.meta().name.clone().unwrap(),
            api: PhantomData,
        }
    }
}
// 针对自定义的Script类型实现From特征
impl From<&Script> for ResourceIndex<Script> {
    fn from(script: &Script) -> Self {
        ResourceIndex {
            namespace: "default".to_string(),
            name: script.spec.manifest.name.clone(),
            api: PhantomData,
        }
    }
}
impl From<&Device> for ResourceIndex<Device> {
    fn from(device: &Device) -> Self {
        ResourceIndex {
            namespace: "default".to_string(),
            name: device.clone(),
            api: PhantomData,
        }
    }
}

//这个RunScriptLookup特征（trait）定义了一组方法，用于在执行脚本的上下文中查找脚本、设备，以及映射设备到脚本和查询脚本的可读写设备。
pub trait RunScriptLookup {//这个RunScriptLookup特征（trait）定义了一组方法，用于在执行脚本的上下文中查找脚本、设备，以及映射设备到脚本和查询脚本的可读写设备
    //该方法接受一个指向Script资源索引的引用，并返回相应的Script实例。这个方法可能会从数据库、文件系统或内存中检索指定的脚本
    fn lookup_script(&mut self, index: &ResourceIndex<Script>) -> Result<Script>;
    fn lookup_device(&mut self, index: &ResourceIndex<Device>) -> Result<Device>;
    fn map_device_to_script(//接受一个设备的资源索引，并返回一个包含可能与该设备相关联的所有脚本索引的向量。这允许应用程序找出哪些脚本需要在特定设备上执行。
        &mut self,
        device: &ResourceIndex<Device>,
    ) -> Result<Vec<ResourceIndex<Script>>>;
    fn lookup_readable(&mut self, script: &Script) -> Result<HashMap<String, ReadDevice>>;
    //lookup_readable返回一个映射，键是可读设备的标识符，值是对应的ReadDevice实例；
    fn lookup_writable(&mut self, script: &Script) -> Result<HashMap<String, WriteDevice>>;
}

pub struct ManagerMsg {
    pub run: RunScript,
    pub name: String,
    pub namespace: String,
}

pub struct Scheduler<T: RunScriptLookup + Send> {//Scheduler是一个泛型结构体，它负责调度脚本的执行。
    lookup_impl: T,//是一个实现了 RunScriptLookup trait 的实例，负责具体的查找脚本和设备的操作。
    script_idgen: ScriptIDGenerator,//用于生成唯一的脚本标识符
}
//这个impl块为Scheduler<T>结构体提供了方法的实现，其中T是满足RunScriptLookup + Send + 'static约束的任意类型。这意味着T可以用于查找脚本和设备，可以跨线程发送，且具有'Static'生命周期。
impl<T: RunScriptLookup + Send + 'static> Scheduler<T> {//这个实现块内部，你可以定义 Scheduler<T> 的方法，这些方法将能利用 T 提供的 RunScriptLookup 功能，
    pub fn new(lookup: T) -> Self {//T类型
        Scheduler {
            lookup_impl: lookup,//其实就是Reflector
            script_idgen: ScriptIDGenerator::default(),
        }
    }//lookup方法根据给定的脚本资源索引index查找并准备执行相关的信息，最后封装到ManagerMsg中返回。
    pub fn lookup(&mut self, index: ResourceIndex<Script>) -> Result<ManagerMsg> {
        trace!(script =? index, "lookup new script");
        let script = self.lookup_impl.lookup_script(&index)?;
        let readable = self.lookup_impl.lookup_readable(&script)?;
        let writable = self.lookup_impl.lookup_writable(&script)?;
        let name = script.spec.manifest.name.clone();
        let namespace = "default".to_string();
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
//用于映射从Device到一组Script的关系。这里的DashMap是一个线程安全的高性能哈希表，提供了并发读写访问。
#[derive(Debug, Clone, Default)]
pub struct Reflector {
    pub selector_map: SelectorMap,//有时候会存在关系和脚本，但是不存在设备
    pub device_store: Store<Device>,
    pub script_store: Store<Script>,
}

impl Reflector {//结构体函数
    pub fn add_device(&self, dev: &Device) {
        let idx = dev.into();
        self.device_store.insert(idx, dev.clone());
    }//将给定的Device实例添加到device_store中。首先，将Device转换为其资源索引，然后将设备及其索引插入到device_store中
    pub fn remove_device(&self, dev: &Device) {
        let idx = dev.into();
        if self.device_store.remove(&idx).is_none() {
            tracing::warn!(device =? dev, "Reflector want to remove nonexsit Device")
        }
    }//从device_store中移除给定的Device实例
    pub fn restart_device(&self, dev: &[Device]) {
        for d in dev {
            self.add_device(d);
        }
    }
    pub fn add_script(&self, script: &Script) {//添加脚本，添加关系
        let idx: ResourceIndex<Script> = script.into();
        self.script_store.insert(idx.clone(), script.clone());//哈希表插入
        let devices = self.get_selected_by_name(script);
        debug!(script =? idx, devices =? devices);
        for dev in devices {//这段代码遍历devices列表，为每个设备更新与之关联的脚本集合
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
        info!("Reflector add Script Sucesseful!")
    }//
    pub fn remove_script(&self, script: &Script) {
        let idx = script.into();//设备索引
        for set in self.selector_map.iter() {//每个set代表与特定设备关联的脚本集合
            set.value().remove(&idx);//移除索引 断开脚本与这些设备的关联。
        }
        if self.script_store.remove(&idx).is_none() {//脚本存储器中移除脚本
            tracing::warn!(script =? script, "Reflector want to remove nonexsit Script")
        }
        info!("Reflector remove Script Sucesseful!")
    }//estart_script方法接收一个脚本数组的引用，并对每个脚本调用add_script方法。这个方法的目的是重新添加（或"重启"）一系列脚本
    pub fn restart_script(&self, scripts: &[Script]) {
        for s in scripts {
            self.add_script(s);
        }
        info!("Reflector restart Script Sucesseful!")
    }//get_selected_by_name方法基于脚本的选择器逻辑，特别是read_selector.match_names部分，来识别和构建与脚本相关联的设备列表。
    
    //这个方法返回一个ResourceIndex<Device>类型的向量，其中每个元素代表一个与该脚本相关联的设备。
    fn get_selected_by_name(&self, script: &Script) -> Vec<ResourceIndex<Device>> {
        let mut result = Vec::new();
        let mut idx = ResourceIndex {
            name: String::new(),
            namespace: "default".to_string(),
            api: PhantomData,
        };
        if let Some(map) = &script.spec.read_selector.match_names {
            for name in map.values() {
                idx.name = name.clone();//循环体中，将每个设备名称分别赋值给idx.name，其中idx是ResourceIndex<Device>类型的一个实例。
                result.push(idx.clone())//通过克隆，确保每个元素都有一个唯一的资源索引副本，把相关的设备索引都加入result。
            }
        }
        result
    }
}

impl RunScriptLookup for Arc<Reflector> {
    fn lookup_script(&mut self, index: &ResourceIndex<Script>) -> Result<Script> {
        self.script_store
            .get(index)//返回一个 Option 类型，如果找到则是 Some，否则是 None。
            .map(|s| s.to_owned())//.map方法应用于这个Option值，如果值是Some，则执行括号内的闭包函数，这里是克隆脚本|s| s.to_owned()
            .ok_or_else(|| eyre!("Script: {:?} not found in store", index))//深拷贝
    }

    fn lookup_device(&mut self, index: &ResourceIndex<Device>) -> Result<Device> {
        self.device_store
            .get(index)
            .map(|s| s.to_owned())
            .ok_or_else(|| eyre!("Device: {:?} not found in store", index))
    }

    fn map_device_to_script(//获取和设备关联的脚本
        &mut self,
        device: &ResourceIndex<Device>,
    ) -> Result<Vec<ResourceIndex<Script>>> {
        let set = self
            .selector_map
            .get(device)
            .ok_or_else(|| eyre!("Device: {:?} not found in store", device))?
            .iter()
            .map(|m| m.clone())
            .collect();//首先通过.iter()获取脚本集合的迭代器，然后使用.map(|m| m.clone())对每个元素（脚本资源索引）进行克隆操作。最后，使用.collect()将这些克隆的脚本资源索引收集到一个向量中。这个过程创建了一个包含所有相关联脚本资源索引副本的新向量。
        Ok(set)
    }

    fn lookup_readable(&mut self, script: &Script) -> Result<HashMap<String, ReadDevice>> {
        let mut result = HashMap::new();
        if let Some(map) = &script.spec.read_selector.match_names {//查找到可读设备设备类型和名字
            for (k, v) in map.iter() {
                let idx = ResourceIndex {//设备索引  k是脚本里用的名字，v是脚本资源名字
                    namespace: "default".to_string(),
                    name: v.to_string(),
                    api: PhantomData,
                };//对于match_names中的每一对(k, v)，创建一个ResourceIndex<Device>实例idx，其namespace从脚本元数据中获取，name设置为设备名称(v)。
                let dev = if let Some(dev) = self.device_store.get(&idx) {
                    dev
                } else {//使用idx在device_store中查询对应的设备。如果设备不存在，则跳过当前迭代。
                    continue;
                };
                let mut status = HashMap::new();//建立 设备状态名称和对应的状态值
                if let Some(s) = &dev.status {
                    trace!(s=?s);
                    for twin in &s.twins {//遍历设备的状态孪生值 比如温度 开关 
                        if let Some(val) = &twin.reported {
                            status.insert(twin.property_name.to_owned(), val.value.to_owned());//插入状态名 状态值
                        }
                    }
                }//遍历设备的状态信息，特别是查找reported状态（这里假设设备状态包含twins，其中每个twin有一个reported字段），并将这些状态信息添加到status映射中。
                result.insert(//对于每个处理过的设备，使用设备的标识符(k)和一个新的ReadDevice实例（包含设备名称和状态映射）更新结果哈希图result。
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
    //lookup_writable方法使得可以根据脚本的声明动态确定哪些设备需要被脚本写入
    fn lookup_writable(&mut self, script: &Script) -> Result<HashMap<String, WriteDevice>> {
        let mut result = HashMap::new();
        if let Some(map) = &script.spec.write_selector.match_names {
            for (k, v) in map.iter() {
                result.insert(k.to_owned(), WriteDevice { name: v.to_owned() });
            }//对于match_names中的每一对(k, v)，使用键值对创建WriteDevice实例，
        }
        Ok(result)
    }
}

#[tracing::instrument(skip_all)]
pub async fn trigger(//段trigger异步函数的设计是在一个事件驱动的架构中，根据设备的变化自动触发与之相关联的脚本执行
    store: Arc<Reflector>,
    device: Receiver<ResourceIndex<Device>>,
    script: Sender<ResourceIndex<Script>>,
) -> Result<()> {
    loop {
        let idx = device.recv_async().await?;
        info!(device =? idx, "map trigger got new device");
        if let Some(scripts) = store.selector_map.get(&idx) {
            for s in scripts.iter() {//遍历脚本迭代器
                info!(script =? *s, "map trigger new script");
                script.send_async(s.clone()).await?;//异步发送脚本索引到另一个通道以触发脚本执行
            }
        }
    }
}

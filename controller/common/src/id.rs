use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
//定义了一个ScriptID结构体，并为它实现了From特征，以允许ScriptID和u32之间的相互转换
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ScriptID(ID);//ScriptID是一个包含单一字段的结构体，这个字段是另一个结构体ID的实例

impl ScriptID {
    pub fn to_u32(&self) -> u32 {
        self.0 .0
    }
}
impl From<ScriptID> for u32 {//这个impl块为ScriptID类型实现了From特征，允许将ScriptID实例转换成u32类型
    fn from(val: ScriptID) -> u32 {
        val.0.into()
    }
}

impl From<u32> for ScriptID {//
    fn from(val: u32) -> Self {
        ScriptID(ID(val))
    }
}

#[derive(Debug, Default)]
pub struct ScriptIDGenerator {
    inner: IDGenerator,
}

impl ScriptIDGenerator {
    pub fn gen(&self) -> ScriptID {
        ScriptID(self.inner.gen())//利用IDGenerator生成一个ID
    }
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ExecutorID(ID);

impl From<ExecutorID> for u32 {
    fn from(val: ExecutorID) -> u32 {
        val.0.into()
    }
}

#[derive(Debug, Default)]
pub struct ExecutorIDGenerator {
    inner: IDGenerator,
}

impl ExecutorIDGenerator {
    pub fn gen(&self) -> ExecutorID {
        ExecutorID(self.inner.gen())
    }
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
struct ID(u32);

impl Into<u32> for ID {
    fn into(self) -> u32 {
        self.0
    }
}

#[derive(Debug)]
struct IDGenerator(AtomicU32);

impl Default for IDGenerator {
    fn default() -> Self {
        IDGenerator(AtomicU32::new(1))
    }
}

impl IDGenerator {
    fn gen(&self) -> ID {
        ID(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

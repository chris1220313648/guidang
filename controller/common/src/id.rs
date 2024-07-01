use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ScriptID(ID);

impl From<ScriptID> for u32 {
    fn from(val: ScriptID) -> u32 {
        val.0.into()
    }
}

impl From<u32> for ScriptID {
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
        ScriptID(self.inner.gen())
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

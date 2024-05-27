use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::str::FromStr;
// use kube::api::ObjectMeta;

//对应原来k8s的数据
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptSpec {
    pub read_selector: DeviceSelectorSet,
    pub write_selector: DeviceSelectorSet,
    pub env: HashMap<String, String>,
    pub manifest: Manifest,
    pub execute_policy: Policy,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptStatus {
    pub last_run: i64,
    pub elapsed_time: u32,
    pub status: i32,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSelectorSet {
    pub match_names: Option<HashMap<String, String>>,
    pub match_abilities: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub script_type: ScriptType,
    pub name: String,
    pub version: String,
    pub register: Option<String>,
}

#[repr(i8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ScriptType {
    Wasm = 0,
    Js = 1,
    Native = 2,
    Standalone = 3,
}

impl std::fmt::Display for ScriptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptType::Js => write!(f, "js"),
            ScriptType::Wasm => write!(f, "wasm"),
            ScriptType::Native => write!(f, "so"),
            ScriptType::Standalone => write!(f, "exe"),
        }
    }
}

impl FromStr for ScriptType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "js" => Ok(ScriptType::Js),
            "Js" => Ok(ScriptType::Js),
            "wasm" => Ok(ScriptType::Wasm),
            "so" => Ok(ScriptType::Native),
            "exe" => Ok(ScriptType::Standalone),
            _ => Err("Unknown script type"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    pub read_change: bool,
    pub webhook: bool,
    pub cron: String,
    pub qos: QosPolicy,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub enum QosPolicy {
    OnlyOnce = 0,
    AtMostOnce = 1,
    AtLeastOnce = 2,
}

impl FromStr for QosPolicy {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OnlyOnce" => Ok(QosPolicy::OnlyOnce),
            "AtMostOnce" => Ok(QosPolicy::AtMostOnce),
            "AtLeastOnce" => Ok(QosPolicy::AtLeastOnce),
            _ => Err("Unknown QoS policy"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    pub spec: ScriptSpec,
    pub status: Option<ScriptStatus>,
}
// src/models.rs





// 数据库表的数据
#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptSqlite3 {
    pub name: String,
    pub script_type: String,
    pub version: String,
    pub elapsed_time: i32,
    pub last_run: i32,
    pub message: String,
    pub status: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub script_id: i32,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutePolicy {
    pub  script_id: i32,
    pub cron: String,
    pub qos: String,
    pub read_change: bool,
    pub webhook: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Selector {
    pub script_id: i32,
    pub selector_type: String,
    pub match_types: String,
    pub match_names: String,
}

use std::{collections::HashMap, str::FromStr};

use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Script spec defination
#[derive(Clone, Debug, Deserialize, Serialize, CustomResource, JsonSchema)]
#[kube(
    group = "hit.edu.cn",
    version = "v1alpha1",
    kind = "Script",
    namespaced,
    apiextensions = "v1",
    status = "ScriptStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct ScriptSpec {
    /// devices that the rule script can read.
    pub read_selector: DeviceSelectorSet,
    /// devices that the rule script can operate.
    pub write_selector: DeviceSelectorSet,
    /// Envirenment variables
    pub env: HashMap<String, String>,
    /// script manifest.
    pub manifest: Manifest,
    /// controller side policy of executing script.
    pub execute_policy: Policy,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptStatus {
    /// unix ms timestamp
    pub last_run: i64,
    /// time of last executing time in us
    pub elapsed_time: u32,
    /// executing status: map to controller.proto
    pub status: i32,
    /// executing message
    pub message: String,
}

/// A device selector set is a map from names of device or device set used in rule script,
/// to acutal device resources in kubernetes.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSelectorSet {
    /// This is a map from name of device used in rule script
    /// to a single device resource in kubnernetes.
    /// key: name of device used in rule script
    /// value: name of device resource
    pub match_names: Option<HashMap<String, String>>,

    /// This is a map from name of a set of devices used in rule scirpt
    /// to a Ability resource in kubernetes.
    /// The Ability resource resolve to a set of devices.
    pub match_abilities: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// script type
    pub script_type: ScriptType,
    /// package name
    pub name: String,
    /// version number
    pub version: String,
    /// override the default script package register
    pub register: Option<String>,
}

#[repr(i8)]
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
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
            "wasm" => Ok(ScriptType::Wasm),
            "so" => Ok(ScriptType::Native),
            "exe" => Ok(ScriptType::Standalone),
            _ => Err("Unknown rule script"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    /// When to execute the script
    /// Execute when state of devices in read_selector changed
    pub read_change: bool,
    /// Execute when webhook is triggerd
    pub webhook: bool,
    /// Same format of crontab
    pub cron: String,
    /// default Qos of submission
    pub qos: QosPolicy,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub enum QosPolicy {
    OnlyOnce = 0,
    AtMostOnce = 1,
    AtLeastOnce = 2,
}

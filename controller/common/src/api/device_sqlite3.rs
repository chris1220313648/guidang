
use schemars::JsonSchema;//用于生成 JSON Schema，这是定义 Kubernetes 资源 schema 的
use serde::{Deserialize, Serialize};// 则用于序列化和反序列化 Rust 结构体
use std::fmt;//提供了格式化功能
use std::{collections::BTreeMap, fmt::Display};
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub spec: DeviceSpec,
    pub status: Option<DeviceStatus>,
}
/// DeviceSpec represents a single device instance. It is an instantiation of a device model.
#[derive(Clone, Debug, Deserialize, Serialize,JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSpec {
    pub name: String,
    /// DeviceModelRef is reference to the device model used as a template
    /// to create the device instance.
    pub device_model_ref: LocalObjectReference,

    /// The protocol configuration used to connect to the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<ProtocolConfig>,

    /// List of property visitors which describe how to access the device properties.
    /// PropertyVisitors must unique by propertyVisitor.propertyName.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub property_visitors: Vec<DevicePropertyVisitor>,

    /// Data section describe a list of time-series properties which should be processed
    /// on edge node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DeviceData>,

    /// NodeSelector indicates the binding preferences between devices and nodes.
    pub node_selector: NodeSelector,
}

impl fmt::Display for DeviceSpec {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        writeln!(
            fmt,
            "Model: {}",
            self.device_model_ref.name.as_deref().unwrap_or("")
        )?;
        writeln!(fmt, "Node selector: {:?}", self.node_selector)?;
        Ok(())
    }
}

/// DeviceStatus reports the device state and the desired/reported values of twin attributes.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone,JsonSchema)]
pub struct DeviceStatus {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub twins: Vec<Twin>,
}

/// Twin provides a logical representation of control properties (writable properties in the
/// device model). The properties can have a Desired state and a Reported state.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone,JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Twin {
    pub property_name: String,
    pub desired: TwinProperty,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported: Option<TwinProperty>,
}

impl fmt::Display for Twin {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(fmt, "{}: {}", self.property_name, self.desired)?;
        if let Some(ref reported) = self.reported {
            write!(fmt, " (reported: {})", reported)
        } else {
            Ok(())
        }
    }
}

/// TwinProperty represents the state of a device property.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone,JsonSchema)]
pub struct TwinProperty {
    pub value: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl TwinProperty {
    pub fn new(value: String) -> TwinProperty {
        TwinProperty {
            value,
            metadata: BTreeMap::new(),
        }
    }
}

impl fmt::Display for TwinProperty {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(fmt, "{}", self.value)
    }
}

/// LocalObjectReference represents a reference to another object in the same namespace.
#[derive(Clone, Debug, Deserialize, Serialize,JsonSchema)]
pub struct LocalObjectReference {
    pub name: Option<String>,
}

/// NodeSelector represents node selector requirements.
#[derive(Clone, Debug, Deserialize, Serialize,JsonSchema)]
pub struct NodeSelector {
    pub node_selector_terms: Vec<NodeSelectorTerm>,
}

/// NodeSelectorTerm represents a requirement for selecting nodes.
#[derive(Clone, Debug, Deserialize, Serialize,JsonSchema)]
pub struct NodeSelectorTerm {
    pub match_expressions: Vec<NodeSelectorRequirement>,
}

/// NodeSelectorRequirement represents a node selector requirement.
#[derive(Clone, Debug, Deserialize, Serialize,JsonSchema)]
pub struct NodeSelectorRequirement {
    pub key: String,
    pub operator: String,
    pub values: Vec<String>,
}

/// ProtocolConfig represents the protocol configuration used to connect to the device.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone,JsonSchema)]
pub struct ProtocolConfig {}

/// DevicePropertyVisitor describes how to access device properties.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone,JsonSchema)]
pub struct DevicePropertyVisitor {}

/// DeviceData describes a list of time-series properties which should be processed on edge node.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone,JsonSchema)]
pub struct DeviceData {}



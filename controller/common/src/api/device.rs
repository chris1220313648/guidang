use k8s_openapi::api::core::v1;
use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::{collections::BTreeMap, fmt::Display};

// TODO: add printcolumn: https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#additional-printer-columns
/// DeviceSpec represents a single device instance. It is an instantation of a device model.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "devices.kubeedge.io",
    version = "v1alpha2",
    kind = "Device",
    namespaced,
    apiextensions = "v1",
    status = "DeviceStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSpec {
    /// DeviceModelRef is reference to the device model used as a template
    /// to create the device instance.
    pub device_model_ref: v1::LocalObjectReference,

    /// The protocol configuration used to connect to the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    protocol: Option<ProtocolConfig>,

    /// List of property visitors which describe how to access the device properties.
    /// PropertyVisitors must unique by propertyVisitor.propertyName.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    property_visitors: Vec<DevicePropertyVisitor>,

    /// Data section describe a list of time-series properties which should be processed
    /// on edge node.
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<DeviceData>,

    /// NodeSelector indicates the binding preferences between devices and nodes.
    /// Refer to k8s.io/kubernetes/pkg/apis/core NodeSelector for more details
    pub node_selector: v1::NodeSelector,
}

impl Display for Device {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(
            fmt,
            "Model: {}",
            self.spec.device_model_ref.name.as_deref().unwrap_or("")
        )?;
        writeln!(fmt, "Node selector: {:?}", self.spec.node_selector)?;
        if let Some(ref status) = self.status {
            writeln!(fmt, "Status:")?;
            for item in &status.twins {
                writeln!(fmt, "{}", item)?;
            }
        }
        Ok(())
    }
}

/// DeviceStatus reports the device state and the desired/reported values of twin attributes.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct DeviceStatus {
    /// A list of device twins containing desired/reported desired/reported values of twin properties..
    /// A passive device won't have twin properties and this list could be empty.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub twins: Vec<Twin>,
}

/// Twin provides a logical representation of control properties (writable properties in the
/// device model). The properties can have a Desired state and a Reported state. The cloud configures
/// the `Desired`state of a device property and this configuration update is pushed to the edge node.
/// The mapper sends a command to the device to change this property value as per the desired state .
/// It receives the `Reported` state of the property once the previous operation is complete and sends
/// the reported state to the cloud. Offline device interaction in the edge is possible via twin
/// properties for control/command operations.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Twin {
    /// The property name for which the desired/reported values are specified.
    /// This property should be present in the device model.
    pub property_name: String,
    /// the desired property value.
    pub desired: TwinProperty,
    /// the reported property value.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported: Option<TwinProperty>,
}

impl Display for Twin {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(fmt, "{}: {}", self.property_name, self.desired)?;
        if let Some(ref reported) = self.reported {
            write!(fmt, " (reported: {})", reported)
        } else {
            Ok(())
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct TwinProperty {
    /// Required: The value for this property.
    pub value: String,
    /// Additional metadata like timestamp when the value was reported etc.
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

impl Display for TwinProperty {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(fmt, "{}", self.value)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatusPatch {
    pub status: DeviceStatus,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
struct ProtocolConfig {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
struct DevicePropertyVisitor {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
struct DeviceData {}

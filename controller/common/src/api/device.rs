// use k8s_openapi::api::core::v1;
// use kube_derive::CustomResource;//简化自定义资源的定义过程，它允许你通过定义一个 Rust 结构体来定义一个 Kubernetes 的自定义资源
// use schemars::JsonSchema;//用于生成 JSON Schema，这是定义 Kubernetes 资源 schema 的
// use serde::{Deserialize, Serialize};// 则用于序列化和反序列化 Rust 结构体
// use std::fmt;//提供了格式化功能
// use std::{collections::BTreeMap, fmt::Display};

// // TODO: add printcolumn: https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#additional-printer-columns
// /// DeviceSpec represents a single device instance. It is an instantation of a device model.
// #[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
// #[kube(//#[kube] 属性用于指定 Kubernetes 自定义资源的详细信息
//     group = "devices.kubeedge.io",
//     version = "v1alpha2",
//     kind = "Device",
//     namespaced,
//     apiextensions = "v1",
//     status = "DeviceStatus"
// )]
// #[serde(rename_all = "camelCase")]//这个属性是 serde 的一部分，它指示在序列化和反序列化时，所有字段名应该使用 camelCase 风格
// pub struct DeviceSpec {
//     /// DeviceModelRef is reference to the device model used as a template
//     /// to create the device instance.
//     pub device_model_ref: v1::LocalObjectReference,//设备模型引用，用作创建设备实例的模板

//     /// The protocol configuration used to connect to the device.
//     #[serde(skip_serializing_if = "Option::is_none")]//如果值为 None，则在序列化时跳过这个字段
//     protocol: Option<ProtocolConfig>,//设备协议配置

//     /// List of property visitors which describe how to access the device properties.
//     /// PropertyVisitors must unique by propertyVisitor.propertyName.
//     #[serde(default)]//如果向量为空，则在序列化时跳过此字段
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     property_visitors: Vec<DevicePropertyVisitor>,

//     /// Data section describe a list of time-series properties which should be processed
//     /// on edge node.
//     #[serde(skip_serializing_if = "Option::is_none")]
//     data: Option<DeviceData>,//描述一系列时序属性，这些属性应该在边缘节点上处理

//     /// NodeSelector indicates the binding preferences between devices and nodes.
//     /// Refer to k8s.io/kubernetes/pkg/apis/core NodeSelector for more details
//     pub node_selector: v1::NodeSelector,//指示设备与节点之间的绑定偏好
// }

// impl Display for Device {
//     //Formatter 是一个用于执行格式化操作的结构体。它提供了多种方法来构建最终的字符串。这里，它被借用为可变引用，因为构建字符串的过程可能会改变 Formatter 的内部状态
//     fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
//         writeln!(//打印模型名称
//             fmt,
//             "Model: {}",
//             self.spec.device_model_ref.name.as_deref().unwrap_or("")
//         )?;
//         //接着打印出节点选择器的信息。这里使用 {:?} 格式说明符，意味着打印的是 Debug 格式的输出
//         writeln!(fmt, "Node selector: {:?}", self.spec.node_selector)?;
//         //打印设备状态：如果 Device 的 status 字段为 Some，则遍历 status.twins 中的每一项，并打印。
//         if let Some(ref status) = self.status {
//             writeln!(fmt, "Status:")?;
//             for item in &status.twins {
//                 writeln!(fmt, "{}", item)?;
//             }
//         }
//         Ok(())
//     }
// }

// /// DeviceStatus reports the device state and the desired/reported values of twin attributes.
// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
// pub struct DeviceStatus {
//     /// A list of device twins containing desired/reported desired/reported values of twin properties..
//     /// A passive device won't have twin properties and this list could be empty.
//     #[serde(default)]
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     pub twins: Vec<Twin>,//这个字段定义了一个名为 twins 的公共成员变量，类型为 Vec<Twin>。这是一个向量，存储的元素类型为 Twin，表示设备的双胞胎属性，用于描述设备的期望状态和报告状态
// }

// /// Twin provides a logical representation of control properties (writable properties in the
// /// device model). The properties can have a Desired state and a Reported state. The cloud configures
// /// the `Desired`state of a device property and this configuration update is pushed to the edge node.
// /// The mapper sends a command to the device to change this property value as per the desired state .
// /// It receives the `Reported` state of the property once the previous operation is complete and sends
// /// the reported state to the cloud. Offline device interaction in the edge is possible via twin
// /// properties for control/command operations.
// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
// #[serde(rename_all = "camelCase")]
// pub struct Twin {
//     /// The property name for which the desired/reported values are specified.
//     /// This property should be present in the device model.
//     pub property_name: String,
//     /// the desired property value.
//     pub desired: TwinProperty,
//     /// the reported property value.
//     #[serde(default)]
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub reported: Option<TwinProperty>,//段表示设备报告的实际属性值
// }

// impl Display for Twin {
//     fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
//         write!(fmt, "{}: {}", self.property_name, self.desired)?;
//         if let Some(ref reported) = self.reported {
//             write!(fmt, " (reported: {})", reported)
//         } else {
//             Ok(())
//         }
//     }
// }

// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
// pub struct TwinProperty {
//     /// Required: The value for this property.
//     pub value: String,
//     /// Additional metadata like timestamp when the value was reported etc.
//     #[serde(default)]
//     #[serde(skip_serializing_if = "BTreeMap::is_empty")]
//     pub metadata: BTreeMap<String, String>,//存储与属性值相关的额外元数据，如时间戳等
// }

// impl TwinProperty {
//     pub fn new(value: String) -> TwinProperty {//为 TwinProperty 结构体提供了一个 new 关联函数，这是一个构造器方法，用于方便地创建 TwinProperty 实例
//         TwinProperty {
//             value,
//             metadata: BTreeMap::new(),
//         }
//     }
// }

// impl Display for TwinProperty {//通过为 TwinProperty 实现 Display trait，使得 TwinProperty 的实例可以使用 {} 格式说明符被格式化为字符串
//     fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
//         write!(fmt, "{}", self.value)
//     }
// }

// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
// #[serde(rename_all = "camelCase")]
// pub struct DeviceStatusPatch {
//     pub status: DeviceStatus,
// }

// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
// struct ProtocolConfig {}

// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
// struct DevicePropertyVisitor {}

// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
// struct DeviceData {}

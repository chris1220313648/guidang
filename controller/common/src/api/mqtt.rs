#![allow(dead_code)]
use serde::Deserialize;
use std::collections::HashMap;

/// the topic prefix for device event
pub const DEVICE_ETPREFIX: &str = "$hw/events/device/";// MQTT 主题的前缀，用于设备事件
/// the topic suffix for twin update event
pub const TWIN_ETUPDATE_SUFFIX: &str = "/twin/update";//设备双胞胎更新事件的主题后缀
/// the topic suffix for twin update result event
pub const TWIN_ETUPDATE_RESULT_SUFFIX: &str = "/twin/update/result";//表示设备双胞胎更新结果事件的主题后缀

/// the struct of device twin update
/// https://github.com/kubeedge/kubeedge/blob/master/edge/pkg/devicetwin/dttype/types.go#L232
#[derive(Clone, Debug, Deserialize)]
pub struct DeviceTwinUpdate {
    event_id: String,
    timestamp: i64,
    twin: HashMap<String, MsgTwin>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MsgTwin {//包含特定设备双胞胎属性的更新详情
    expected: Option<TwinValue>,
    actual: Option<TwinValue>,
    optional: Option<bool>,
    metadata: Option<TypeMetadata>,
    expected_version: Option<TwinVersion>,
    actual_version: Option<TwinVersion>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TwinValue {//表示双胞胎属性的值及其元数据
    value: Option<String>,
    metadata: Option<ValueMetadata>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TypeMetadata {//存储关于双胞胎属性类型的信息
    r#type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TwinVersion {
    cloud: i64,
    edge: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ValueMetadata {
    timestamp: i64,
}

#[cfg(test)]
#[test]
fn mqtt_device_twin_update_type() {
    let msg = r#"
    {
        "event_id": "",
        "timestamp": 1592129718158,
        "twin": {
            "temperature": {
                "expected": {
                    "value": "",
                    "metadata": {
                        "timestamp": 1592129378826
                    }
                },
                "actual": {
                    "value": "42",
                    "metadata": {
                        "timestamp": 1592129378809
                    }
                },
                "optional": false,
                "metadata": {
                    "type": "string"
                }
            }
        }
    }"#;

    let _: DeviceTwinUpdate = serde_json::from_str(&msg).unwrap();
}

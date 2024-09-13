use schemars::JsonSchema;//用于生成 JSON Schema，这是定义 Kubernetes 资源 schema 的
use serde::{Deserialize, Serialize};// 则用于序列化和反序列化 Rust 结构体
use std::fmt;//提供了格式化功能
use std::{collections::BTreeMap, fmt::Display};
use std::collections::HashMap;
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Ability {
    pub name: String,
    pub http_url: String,
    pub attributes: HashMap<String, Item>,//设备名-属性表

}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct Item {
    pub properties: HashMap<String, i32>,
    
}



pub mod ability;
pub mod bootstrap;
pub mod devices;
pub mod log;

use deno_core::{url::Url, Extension};
use proto::{server_message::run_script::ReadDevice, QosPolicy};
use std::collections::HashMap;
use time::OffsetDateTime;
//个用于Deno运行时环境的模块，它定义了一些核心结构和初始化函数，用于创建和配置Deno扩展。
/// All inmutable information of this rule and session
#[derive(Debug, Clone)]
pub struct Rule {// 表示规则和会话的不变信息，如脚本ID、开始时间、名称、版本、注册URL和QoS（Quality of Service）策略。
    pub script_id: u32,
    pub start_time: OffsetDateTime,
    pub name: String,
    pub version: String,
    pub register: String,
    pub qos: QosPolicy,
}

#[derive(Debug)]
pub struct ReadableDevices {
    pub devices: HashMap<String, ReadDevice>,
}

#[derive(Debug)]
pub struct WritableDevices {
    pub devices: HashMap<String, DeviceSnapshot>,
}

pub struct Envvar {//包含环境变量的映射。
    pub env: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct DeviceSnapshot {//表示设备的快照，包含设备名称和待提交更改的映射。
    pub name: String,
    pub commits: HashMap<String, String>,
}

impl Rule {
    pub fn url(&self) -> Url {//方法用于构建访问规则脚本的URL，这个URL是基于规则的注册URL、名称和版本动态生成的。
        Url::parse(&format!(
            "{}/{}/{}.js",
            self.register, self.name, self.version
        ))
        .expect("Bad register url")
    }
}

pub fn extensions() -> Vec<Extension> {//扩展初始化函数
    vec![
        deno_webidl::init(),//提供WebIDL支持
        log::init(),
        deno_url::init(),//提供URL处理功能
        deno_web::init::<WebPermission>(deno_web::BlobStore::default(), None),
        devices::init(),
        ability::init(),
        bootstrap::init(),
    ]
}

struct WebPermission;

impl deno_web::TimersPermission for WebPermission {
    fn allow_hrtime(&mut self) -> bool {
        false
    }
    fn check_unstable(&self, _state: &deno_core::OpState, _api_name: &'static str) {
        // None
    }
}

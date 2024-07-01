pub mod ability;
pub mod bootstrap;
pub mod devices;
pub mod log;

use deno_core::{url::Url, Extension};
use proto::{server_message::run_script::ReadDevice, QosPolicy};
use std::collections::HashMap;
use time::OffsetDateTime;

/// All inmutable information of this rule and session
#[derive(Debug, Clone)]
pub struct Rule {
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

pub struct Envvar {
    pub env: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct DeviceSnapshot {
    pub name: String,
    pub commits: HashMap<String, String>,
}

impl Rule {
    pub fn url(&self) -> Url {
        Url::parse(&format!(
            "{}/{}/{}.js",
            self.register, self.name, self.version
        ))
        .expect("Bad register url")
    }
}

pub fn extensions() -> Vec<Extension> {
    vec![
        deno_webidl::init(),
        log::init(),
        deno_url::init(),
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

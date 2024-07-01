use anyhow::anyhow;
use anyhow::Result;
use deno_core::{located_script_name, JsRuntime, ModuleLoader, RuntimeOptions, Snapshot};
use executor_ops as ops;
use prost_types::{Duration, Timestamp};
use proto::{
    controller_service_client::ControllerServiceClient,
    script_status::ScriptStatusCode,
    server_message::{run_script::ReadDevice, RunScript},
    QosPolicy, ScriptStatus,
};
use reqwest::{Client, ClientBuilder};
use std::{collections::HashMap, rc::Rc};
use time::OffsetDateTime;
use tonic::transport::Channel;
use tracing::warn;
use tracing::{error, info};

pub static SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/SNAPSHOT.bin"));

pub struct DenoWorker {
    pub rt: JsRuntime,
}

#[derive(Clone)]
pub struct GlobalOption<M: ModuleLoader> {
    pub default_register: String,
    pub module_loader: M,
}

impl DenoWorker {
    pub fn new<M: ModuleLoader + 'static>(
        run: RunScript,
        global: GlobalOption<M>,
        client: ControllerServiceClient<Channel>,
    ) -> DenoWorker {
        let GlobalOption {
            default_register,
            module_loader,
        } = global;
        let qos = run.default_qos();
        let manifest = run.manifest.unwrap();
        let mut register = manifest.register;
        if register.is_empty() {
            register = default_register.clone()
        }
        let state = Rc::new(ops::Rule {
            script_id: run.script_id,
            start_time: OffsetDateTime::now_utc(),
            name: manifest.package_name,
            version: manifest.package_version,
            register,
            qos,
        });
        let readable = ops::ReadableDevices {
            devices: run.readable,
        };
        let writeable = {
            let mut devices = HashMap::new();
            for (k, v) in run.writable {
                devices.insert(
                    k,
                    ops::DeviceSnapshot {
                        name: v.name,
                        commits: HashMap::new(),
                    },
                );
            }
            ops::WritableDevices { devices }
        };
        let envvar = ops::Envvar { env: run.env };
        let http_client = ClientBuilder::new()
            .gzip(true)
            .brotli(true)
            .build()
            .unwrap();
        let startup_snapshot = Some(Snapshot::Static(SNAPSHOT));
        let mut rt = JsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(module_loader)),
            extensions: ops::extensions(),
            startup_snapshot,
            will_snapshot: false,
            ..Default::default()
        });
        let op_state = rt.op_state();
        let mut op_state = op_state.borrow_mut();
        op_state.put(state);
        op_state.put(readable);
        op_state.put(writeable);
        op_state.put(envvar);
        op_state.put(client);
        op_state.put(http_client);
        DenoWorker { rt }
    }

    pub async fn run(mut self) {
        let res = self.run_inner().await;
        let op_state = self.rt.op_state();
        let mut op_state = op_state.borrow_mut();
        let state: &Rc<ops::Rule> = op_state.borrow();
        let (code, message) = if let Err(e) = res {
            error!(
                "Script {}({}) crashed: {:?}",
                state.name, state.script_id, e
            );
            (ScriptStatusCode::Crash, format!("{:?}", e))
        } else {
            (ScriptStatusCode::Ok, String::new())
        };
        let start = Some(Timestamp {
            seconds: state.start_time.unix_timestamp(),
            nanos: state.start_time.nanosecond() as i32,
        });
        let now = OffsetDateTime::now_utc();
        let duration = now - state.start_time;
        let duration = Some(Duration {
            seconds: duration.whole_seconds(),
            nanos: duration.subsec_nanoseconds(),
        });
        let request = ScriptStatus {
            script_id: state.script_id,
            start,
            duration,
            code: code as i32,
            message,
        };
        let client: &mut ControllerServiceClient<Channel> = op_state.borrow_mut();
        if let Err(e) = client.update_script_status(request.clone()).await {
            error!(error =? e, "Failed to update script status");
        }
        info!(status =? request, "Script exit");
    }

    async fn run_inner(&mut self) -> Result<()> {
        self.bootstrap();
        let res = {
            let op_state = self.rt.op_state();
            let op_state = op_state.borrow_mut();
            let http: &Client = op_state.borrow();
            let state: &Rc<ops::Rule> = op_state.borrow();
            let url = state.url();
            http.get(url).send().await?
        };
        if !res.status().is_success() {
            return Err(anyhow!(
                "Download script failed with status: {}",
                res.status()
            ));
        }
        let code = res.text().await?;
        self.rt.execute_script(&located_script_name!(), &code)?;
        let result = self.rt.execute_script(&located_script_name!(), "main()")?;
        let result = self.rt.resolve_value(result).await?;
        let result = result.open(self.rt.v8_isolate());
        if !result.is_null_or_undefined() {
            let res = result.to_rust_string_lossy(&mut self.rt.handle_scope());
            warn!("Script's main() return a value which is ignored: {:?}", res)
        }
        Ok(())
    }

    // TODO: use this when add back module import support
    #[deprecated(note = "Use run_inner instead")]
    async fn run_inner_old(&mut self) -> Result<()> {
        self.bootstrap();
        let specifier = {
            let op_state = self.rt.op_state();
            let op_state = op_state.borrow_mut();
            let state: &Rc<ops::Rule> = op_state.borrow();
            state.url()
        };
        let id = self.rt.load_main_module(&specifier, None).await?;
        tracing::debug!("load_main_module");
        let mut receiver = self.rt.mod_evaluate(id);
        tracing::debug!("mod_evaluate");
        tokio::select! {
          maybe_result = &mut receiver => {
            tracing::debug!("received module evaluate {:?}", maybe_result);
            maybe_result.expect("Module evaluation result not provided.")?;
          }

          event_loop_result = self.rt.run_event_loop(false) => {
            event_loop_result?;
            let maybe_result = receiver.await;
            maybe_result.expect("Module evaluation result not provided.")?;
          }
        }
        tracing::debug!("mod_evaluate done");
        self.rt.run_event_loop(false).await?;
        tracing::debug!("run_event_loop done");
        self.rt.execute_script(&located_script_name!(), "main()")?;
        tracing::debug!("run main() done");
        Ok(())
    }

    pub fn bootstrap(&mut self) {
        let op_state = self.rt.op_state();
        let op_state = op_state.borrow();
        let env: &ops::Envvar = op_state.borrow();
        let arg = serde_json::json!({
            "noColor": false,
            "env": env.env
        });
        let script = format!("bootstrap({})", arg);
        self.rt
            .execute_script(&located_script_name!(), &script)
            .expect("Failed to execute bootstrap script");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::loader::{FsLoader, RegisterLoader};
    use proto::server_message::run_script::{manifest::ScriptType, Manifest, WriteDevice};
    use std::path::PathBuf;
    use time::OffsetDateTime;

    fn test_localhost_opt() -> GlobalOption<RegisterLoader> {
        GlobalOption {
            default_register: "http://127.0.0.1:8080".to_string(),
            module_loader: RegisterLoader::new(),
        }
    }

    fn test_local_opt() -> GlobalOption<FsLoader> {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("config")
            .join("new_register")
            .canonicalize()
            .unwrap();
        let url = reqwest::Url::from_directory_path(base).unwrap();
        let default_register = url.to_string();
        GlobalOption {
            default_register,
            module_loader: FsLoader,
        }
    }

    fn empty_run() -> RunScript {
        RunScript {
            script_id: 0,
            manifest: Some(Manifest {
                package_name: "test".to_string(),
                package_version: "0.1_beta1".to_string(),
                register: "".to_owned(),
                script_type: ScriptType::Js as i32,
            }),
            readable: HashMap::new(),
            writable: HashMap::new(),
            env: HashMap::new(),
            default_qos: 0,
        }
    }

    fn init_log() {
        tracing_subscriber::fmt()
            .pretty()
            .with_thread_names(true)
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    fn get_tokio() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .thread_name("test_worker")
            .build()
            .unwrap()
    }

    #[test]
    fn test_url() {
        const REGISTER: &str = "http://register.default.svc.cluster.local:8080";

        let rule = ops::Rule {
            script_id: 0,
            start_time: OffsetDateTime::now_utc(),
            name: "my_script".to_string(),
            version: "0.1_beta1".to_string(),
            register: REGISTER.to_string(),
            qos: QosPolicy::AtMostOnce,
        };

        let url = rule.url();

        assert_eq!(url.scheme(), "http");
        assert_eq!(url.host_str(), Some("register.default.svc.cluster.local"));
        assert_eq!(url.port(), Some(8080));

        let path: Vec<_> = url.path_segments().unwrap().collect();
        assert_eq!(&path, &["my_script", "0.1_beta1.js"])
    }

    fn detailed_run() -> RunScript {
        let mut s = empty_run();
        s.readable.insert(
            "temp".into(),
            ReadDevice {
                name: "dht11-sensor-1".into(),
                status: HashMap::from([("temperature".into(), "27.5".into())]),
            },
        );
        s.writable.insert(
            "switch".into(),
            WriteDevice {
                name: "kdh12".into(),
            },
        );
        s
    }
}

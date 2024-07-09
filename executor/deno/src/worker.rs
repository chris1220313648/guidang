use anyhow::anyhow;
use anyhow::Result;
//deno_core库的关键组件，包括用于管理JavaScript运行时的JsRuntime，以及其他与模块加载和运行时配置相关的类型
use deno_core::{located_script_name, JsRuntime, ModuleLoader, RuntimeOptions, Snapshot};
use executor_ops as ops;
use prost_types::{Duration, Timestamp};//prost是一个Rust库，用于处理Protocol Buffers（
use proto::{//引用了一个名为proto的模块或包，它可能包含了由.proto文件自动生成的Rust代码，用于gRPC通信。
    controller_service_client::ControllerServiceClient,
    script_status::ScriptStatusCode,
    server_message::{run_script::ReadDevice, RunScript},
    QosPolicy, ScriptStatus,
};
use reqwest::{Client, ClientBuilder};//reqwest是一个Rust的HTTP客户端库，用于发起HTTP请求
use std::{collections::HashMap, rc::Rc};//标准库中的类型，HashMap用于存储键值对集合，Rc是一个引用计数类型，用于共享数据。
use time::OffsetDateTime;
use tonic::transport::Channel;
use tracing::warn;
use tracing::{error, info};

pub static SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/SNAPSHOT.bin"));//concat!宏用于在编译时将字符串字面量拼接在一起，而env!("OUT_DIR")是一个环境变量，指向构建脚本输出的目录，通常用于存放由构建脚本生成的文件。
//这行代码定义了一个名为SNAPSHOT的静态变量，它包含了一个编译时包含的二进制快照文件的字节。这个快照可能是预编译的JavaScript代码或Deno运行时的某个状态，可以被JsRuntime直接使用，以加快初始化速度。
pub struct DenoWorker {
    pub rt: JsRuntime,//DenoWorker是一个包含JsRuntime的结构体。JsRuntime是deno_core库中的一个类型，代表了一个JavaScript运行时环境，其中可以执行JavaScript代码。DenoWorker可能被设计用来封装和管理这个运行时环境，执行具体的JavaScript任务或脚本
}

#[derive(Clone)]
pub struct GlobalOption<M: ModuleLoader> {//<M: ModuleLoader>：这部分定义了一个泛型参数M，并约束M必须实现ModuleLoader trait。
    pub default_register: String,
    pub module_loader: M,//ModuleLoader是deno_core中的一个trait，定义了模块加载的接口。这表明GlobalOption可以与任何实现了ModuleLoader接口的模块加载器一起工作
}//包含全局配置选项

impl DenoWorker {//封装和管理运行时环境
    pub fn new<M: ModuleLoader + 'static>(
        run: RunScript,
        global: GlobalOption<M>,
        client: ControllerServiceClient<Channel>,
    ) -> DenoWorker {//构造函数接收三个参数：run、global和client。run是一个RunScript结构体，包含了执行脚本所需的信息；global是GlobalOption<M>的实例，包含全局配置；client是ControllerServiceClient<Channel>的实例，可能用于与外部服务进行gRPC通信。
        let GlobalOption {
            default_register,
            module_loader,
        } = global;//函数内部首先从global中解构出default_register和module_loader。
        let qos = run.default_qos();//默认的服务质量（Quality of Service）设置。
        let manifest = run.manifest.unwrap();// 从manifest获取注册信息。
        let mut register = manifest.register;//
        if register.is_empty() {
            register = default_register.clone();
            info!("use local default register value:{}",register);
        }
        let state = Rc::new(ops::Rule {//创建一个新的规则状态对象，并存储有关脚本执行的信息
            script_id: run.script_id,
            start_time: OffsetDateTime::now_utc(),
            name: manifest.package_name,
            version: manifest.package_version,
            register,
            qos,
        });
        let readable = ops::ReadableDevices {
            devices: run.readable,
        };//创建一个包含可读设备信息的对象。run.readable可能是一个设备列表。
        let writeable = {
            let mut devices = HashMap::new();
            for (k, v) in run.writable {
                devices.insert(//每个设备用其名称作为键，值是ops::DeviceSnapshot对象，后者包含设备的名称和一个空的提交HashMa
                    k,
                    ops::DeviceSnapshot {
                        name: v.name,
                        commits: HashMap::new(),
                    },
                );
            }
            ops::WritableDevices { devices }
        };
        let envvar = ops::Envvar { env: run.env };//创建一个包含环境变量的结构体实例。run.env很可能是一个从外部传入的、包含环境变量键值对的HashMap。
        let http_client = ClientBuilder::new()//使用reqwest库的ClientBuilder来构建一个HTTP客户端实例
            .gzip(true)
            .brotli(true)
            .build()
            .unwrap();
        let startup_snapshot = Some(Snapshot::Static(SNAPSHOT));//设置一个启动快照，SNAPSHOT可能是预编译的JavaScript代码的二进制表示，用于加速JsRuntime的启动
        let mut rt = JsRuntime::new(RuntimeOptions {//
            module_loader: Some(Rc::new(module_loader)),
            extensions: ops::extensions(),//通过JsRuntime::new方法创建一个新的JsRuntime实例。RuntimeOptions用于配置JsRuntime，包括模块加载器、扩展、启动快照等。
            startup_snapshot,
            will_snapshot: false,
            ..Default::default()
        });
        let op_state = rt.op_state();//获取JsRuntime的操作状态（OpState），这是一个包含运行时各种状态和数据的容器。
        let mut op_state = op_state.borrow_mut();
        op_state.put(state);//方法将先前创建的各种状态和对象放入操作状态中。这包括脚本执行规则、可读/可写设备、环境变量、gRPC客户端和HTTP客户端等。
        op_state.put(readable);
        op_state.put(writeable);
        op_state.put(envvar);
        op_state.put(client);
        op_state.put(http_client);
        DenoWorker { rt }
    }

    pub async fn run(mut self) {
        let res = self.run_inner().await;//这行代码调用run_inner方法来异步执行脚本，并等待执行结果
        let op_state = self.rt.op_state();//获取JsRuntime的操作状态（OpState），这是一个存储运行时状态和数据的容器。
        let mut op_state = op_state.borrow_mut();//过调用borrow_mut获得一个可变引用，这允许修改操作状态中的数据。
        let state: &Rc<ops::Rule> = op_state.borrow();
        let (code, message) = if let Err(e) = res {
            error!(//这部分尝试匹配res是否为Err变体。如果是，那么将Err中的值绑定到变量e上，并执行大括号内的代码。这意味着如果脚本执行失败，执行内部的逻辑。
                "Script {}({}) crashed: {:?}",
                state.name, state.script_id, e
            );
            (ScriptStatusCode::Crash, format!("{:?}", e))
        } else {
            (ScriptStatusCode::Ok, String::new())
        };
        let start = Some(Timestamp {//脚本开始时间
            seconds: state.start_time.unix_timestamp(),
            nanos: state.start_time.nanosecond() as i32,
        });
        let now = OffsetDateTime::now_utc();
        let duration = now - state.start_time;//持续时间
        let duration = Some(Duration {
            seconds: duration.whole_seconds(),
            nanos: duration.subsec_nanoseconds(),
        });
        let request = ScriptStatus {//创建一个ScriptStatus对象，包含脚本的ID、开始时间、持续时间、状态代码和状态消息。这个对象包含了脚本执行的所有结果信息，将被用于更新服务端的脚本状态。
            script_id: state.script_id,
            start,
            duration,
            code: code as i32,
            message,
        };
        let client: &mut ControllerServiceClient<Channel> = op_state.borrow_mut();
        if let Err(e) = client.update_script_status(request.clone()).await {
            error!(error =? e, "Failed to update script status");//将ScriptStatus请求发送到服务端以更新脚本状态。如果调用失败（返回Err），则记录一条错误日志。
        }
        info!(status =? request, "Script exit");
    }

    async fn run_inner(&mut self) -> Result<()> {
        self.bootstrap();//初始化
        let res = {
            let op_state = self.rt.op_state();
            let op_state = op_state.borrow_mut();
            let http: &Client = op_state.borrow();//获取操作状态（OpState）的可变引用，并从中借用HTTP客户端和脚本运行规则的状态。
            let state: &Rc<ops::Rule> = op_state.borrow();
            let url = state.url();//文件服务器地址+名字+版本
            http.get(url).send().await?//构造请求URL，并异步发送HTTP GET请求以下载脚本。如果响应状态不是成功（200-299），则返回错误。
        };
        if !res.status().is_success() {//检查响应状态
            return Err(anyhow!(
                "Download script failed with status: {}",
                res.status()
            ));
        }
        let code = res.text().await?;//异步获取响应体中的文本内容，即JavaScript代码
        self.rt.execute_script(&located_script_name!(), &code)?;//执行下载的脚本
        let result = self.rt.execute_script(&located_script_name!(), "main()")?;//再次调用execute_script执行名为main的函数。这假设下载的脚本定义了一个名为main的顶层函数。
        let result = self.rt.resolve_value(result).await?;//处理execute_script返回的结果。这可能是处理JavaScript Promise的步骤，确保异步操作完成
        let result = result.open(self.rt.v8_isolate());//尝试访问返回值的具体内容。
        if !result.is_null_or_undefined() {
            let res = result.to_rust_string_lossy(&mut self.rt.handle_scope());//将结果转换为Rust字符串
            warn!("Script's main() return a value which is ignored: {:?}", res)
        }
        Ok(())
    }

    // TODO: use this when add back module import support
    #[deprecated(note = "Use run_inner instead")]
    async fn run_inner_old(&mut self) -> Result<()> {
        self.bootstrap();
        let specifier = {//获取操作状态，然后从中借用state来获取模块的URL，即要加载的JavaScript模块的位置。
            let op_state = self.rt.op_state();
            let op_state = op_state.borrow_mut();
            let state: &Rc<ops::Rule> = op_state.borrow();
            state.url()
        };
        let id = self.rt.load_main_module(&specifier, None).await?;//来异步加载主模块。
        tracing::debug!("load_main_module");
        let mut receiver = self.rt.mod_evaluate(id);
        tracing::debug!("mod_evaluate");//调用mod_evaluate开始评估加载的模块，并获取一个结果接收器。
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
        let op_state = self.rt.op_state();//获取JsRuntime的操作状态（OpState），这是存储运行时状态和数据的容器。
        let op_state = op_state.borrow();//通过调用borrow方法获得OpState的引用。这允许从操作状态中读取数据。
        let env: &ops::Envvar = op_state.borrow();//从操作状态中借用环境变量的状态。
        let arg = serde_json::json!({
            "noColor": false,//这个对象设置"noColor": false（这可能影响脚本的日志或输出格式），并传递之前从操作状态中获取的环境变量。
            "env": env.env
        });
        let script = format!("bootstrap({})", arg);//将配置的JSON对象格式化为一个字符串，形成一个调用bootstrap函数的脚本
        self.rt
            .execute_script(&located_script_name!(), &script)
            .expect("Failed to execute bootstrap script");//执行构建的初始化脚本
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::loader::{FsLoader, RegisterLoader};
    use proto::server_message::run_script::{manifest::ScriptType, Manifest, WriteDevice};
    use std::path::PathBuf;
    use time::OffsetDateTime;

    fn test_localhost_opt() -> GlobalOption<RegisterLoader> {//test_localhost_opt函数创建并返回一个GlobalOption<RegisterLoader>实例，这个实例配置了一个指向本地服务器http://127.0.0.1:8080的默认注册路径，并使用RegisterLoader作为模块加载器
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
            .join("config")//然后附加"config/new_register"到路径上，指向配置文件或目录的位置。
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
            }),//它表示一个空的脚本运行请求。这里的脚本没有可读写的设备、没有环境变量，且服务质量（QoS）设置为0。这个函数可以用于测试脚本执行流程，不涉及实际的设备操作或环境变量。
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
            .unwrap()//创建并返回一个配置为当前线程的Tokio运行时实例，这个运行时启用了所有特性，并设置了线程名称为"test_worker"
    }

    #[test]
    fn test_url() {//这个测试验证了ops::Rule结构体实例化后，其url方法生成的URL是否符合预期：
        const REGISTER: &str = "http://register.default.svc.cluster.local:8080";

        let rule = ops::Rule {
            script_id: 0,
            start_time: OffsetDateTime::now_utc(),
            name: "my_script".to_string(),
            version: "0.1_beta1".to_string(),
            register: REGISTER.to_string(),
            qos: QosPolicy::AtMostOnce,
        };

        let url = rule.url();//url方法生成的URL

        assert_eq!(url.scheme(), "http");
        assert_eq!(url.host_str(), Some("register.default.svc.cluster.local"));
        assert_eq!(url.port(), Some(8080));

        let path: Vec<_> = url.path_segments().unwrap().collect();
        assert_eq!(&path, &["my_script", "0.1_beta1.js"])
    }//路径 正确地映射到了脚本的名称和版本，路径应该是["my_script", "0.1_beta1.js"]。

    fn detailed_run() -> RunScript {
        let mut s = empty_run();//创建一个基本的RunScript实例，没有额外的设备或环境变量
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

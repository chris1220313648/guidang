use std::{cell::RefCell, rc::Rc};

use deno_core::{
    error::AnyError,
    error::{generic_error, range_error, resource_unavailable},
    include_js_files, op, Extension, OpState,
};
use proto::{controller_service_client::ControllerServiceClient, QosPolicy, UpdateDevice};//用于与控制器服务进行通信的客户端和相关数据结构。
use serde::Deserialize;
use tonic::transport::Channel;
use tracing::debug;

use crate::{ReadableDevices, Rule, WritableDevices};

pub fn init() -> Extension {
    Extension::builder()
        .js(include_js_files!(
            prefix "executor/deno:",
            "devices/01_device.js",
        ))
        .ops(vec![//注册一系列操作
            op_list_readable_devices::decl(),
            op_list_writable_devices::decl(),
            op_get_device_status::decl(),
            op_update_device_desired::decl(),
            op_commit_device::decl(),
        ])
        .build()
}

#[op]
pub fn op_list_readable_devices(
    state: &mut OpState,
    _: (),
    _: (),
) -> Result<Vec<String>, AnyError> {
    let readable: &ReadableDevices = state.borrow();//通过state.borrow()借用ReadableDevices
    let list = readable.devices.keys().map(|v| v.clone()).collect();//获取所有可读设备的名称，并收集到一个 Vec<String> 中
    Ok(list)
}

#[op]
pub fn op_list_writable_devices(
    state: &mut OpState,
    _: (),
    _: (),
) -> Result<Vec<String>, AnyError> {
    let writable: &WritableDevices = state.borrow();
    let list = writable.devices.keys().map(|v| v.clone()).collect();
    Ok(list)
}

#[op]
pub fn op_get_device_status(
    state: &mut OpState,
    name: String,
    property: String,
) -> Result<Option<String>, AnyError> {
    let readable: &ReadableDevices = state.borrow();//借用可读设备集合
    debug!("{:?}", readable);
    let value = readable
        .devices
        .get(&name)//尝试从设备映射中找到指定名称的设备，并查询该设备的状态中是否存在指定的属性。如果找到，返回该属性的值；否则，返回None
        .and_then(|d| d.status.get(&property))
        .map(|v| v.to_owned());
    debug!(name = ?name, property = ?property, value = ?value);
    Ok(value)
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateDeviceDesired {
    name: String,
    property: String,
    value: String,
}

#[op]
pub fn op_update_device_desired(
    state: &mut OpState,
    arg: UpdateDeviceDesired,
    _: (),
) -> Result<(), AnyError> {
    let writable: &mut WritableDevices = state.borrow_mut();
    let r = if let Some(d) = writable.devices.get_mut(&arg.name) {
        debug!(arg =? arg, "update device desired");//使用if let结构尝试从writable.devices中找到匹配arg.name的设备。
        d.commits.insert(arg.property, arg.value);//如果找到了设备，就在该设备的commits映射中插入或更新arg.property对应的值为arg.value
        Ok(())
    } else {
        Err(generic_error("Device not found"))
    };
    debug!(writable =? writable, "after update");
    r
}

#[op]
pub async fn op_commit_device(//用于提交对某个设备的所有待定更改
    state: Rc<RefCell<OpState>>,
    name: String,
    qos: Option<i32>,
) -> Result<(), AnyError> {
    let (commits, resource_name) = {
        let mut op_state = state.try_borrow_mut().map_err(|_| resource_unavailable())?;
        let writable: &mut WritableDevices = op_state.borrow_mut();
        if let Some(d) = writable.devices.get_mut(&name) {
            (std::mem::take(&mut d.commits), d.name.clone())//先尝试从Deno的运行时状态中获取指定设备的待提交更改列表
        } else {
            return Err(generic_error("Device not found"));
        }
    };
    debug!(commits =? commits, name =? name, qos =? qos, "commit device");
    let (mut client, request) = {//构建请求
        let op_state = state.try_borrow().map_err(|_| resource_unavailable())?;
        let rule: &Rc<Rule> = op_state.borrow();
        let qos = match qos {
            Some(qos) => {
                QosPolicy::from_i32(qos).ok_or_else(|| range_error("Invalid Qos value"))? as i32
            }
            None => rule.qos as i32,
        };
        let request = UpdateDevice {//，构建一个向设备管理服务发送的请求。如果提供了QoS参数，则使用该参数值；否则，使用默认值。
            script_id: rule.script_id,
            name: resource_name,
            desired: commits,
            qos,
        };
        let client: &ControllerServiceClient<Channel> = op_state.borrow();
        (client.clone(), request)
    };
    debug!(request =? request, "commit device requset");
    // FIXME: Qos handle
    client.update_device_desired(request).await?;//通过之前获取的gRPC客户端，异步发送更新设备状态的请求。这一步可能涉及与外部服务的网络通信。
    debug!("requset finished");
    Ok(())
}

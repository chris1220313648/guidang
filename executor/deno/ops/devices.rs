use std::{cell::RefCell, rc::Rc};

use deno_core::{
    error::AnyError,
    error::{generic_error, range_error, resource_unavailable},
    include_js_files, op, Extension, OpState,
};
use proto::{controller_service_client::ControllerServiceClient, QosPolicy, UpdateDevice};
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
        .ops(vec![
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
    let readable: &ReadableDevices = state.borrow();
    let list = readable.devices.keys().map(|v| v.clone()).collect();
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
    let readable: &ReadableDevices = state.borrow();
    debug!("{:?}", readable);
    let value = readable
        .devices
        .get(&name)
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
        debug!(arg =? arg, "update device desired");
        d.commits.insert(arg.property, arg.value);
        Ok(())
    } else {
        Err(generic_error("Device not found"))
    };
    debug!(writable =? writable, "after update");
    r
}

#[op]
pub async fn op_commit_device(
    state: Rc<RefCell<OpState>>,
    name: String,
    qos: Option<i32>,
) -> Result<(), AnyError> {
    let (commits, resource_name) = {
        let mut op_state = state.try_borrow_mut().map_err(|_| resource_unavailable())?;
        let writable: &mut WritableDevices = op_state.borrow_mut();
        if let Some(d) = writable.devices.get_mut(&name) {
            (std::mem::take(&mut d.commits), d.name.clone())
        } else {
            return Err(generic_error("Device not found"));
        }
    };
    debug!(commits =? commits, name =? name, qos =? qos, "commit device");
    let (mut client, request) = {
        let op_state = state.try_borrow().map_err(|_| resource_unavailable())?;
        let rule: &Rc<Rule> = op_state.borrow();
        let qos = match qos {
            Some(qos) => {
                QosPolicy::from_i32(qos).ok_or_else(|| range_error("Invalid Qos value"))? as i32
            }
            None => rule.qos as i32,
        };
        let request = UpdateDevice {
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
    client.update_device_desired(request).await?;
    debug!("requset finished");
    Ok(())
}

// Copyright 2020-2022 Han puyu. All rights reserved.

"use strict";

((window) => {
    const core = window.Deno.core;

    function listReadableDevices() {
        return core.opSync("op_list_readable_devices")
    }

    function listWritableDevices() {
        return core.opSync("op_list_writable_devices")
    }

    function getDeviceStatus(device, property) {
        return core.opSync("op_get_device_status", device, property)
    }

    function setDeviceStatus(device, property, value) {
        return core.opSync("op_update_device_desired", {
            "name": device,
            "property": property,
            "value": value
        })
    }

    async function commitDevice(device, qos) {
        return await core.opAsync("op_commit_device", device, qos)
    }

    window.__bootstrap.devices = {
        listReadableDevices,
        listWritableDevices,
        getDeviceStatus,
        setDeviceStatus,
        commitDevice
    };
})(this);
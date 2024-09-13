async function filterService(value) {
    let url = Deno.env["filter_service_url"];
    url += `?value=${value}`;
    console.log(url);
    return await Deno.httpGet(url, "");
}

async function main() {
    const temp = parseFloat(Device.getDeviceStatus("temp-sensor-name", "temperature"));
    console.log("Value of temperature from sensor is ", temp);

    const filterValue = parseFloat(await filterService(temp));
    console.log("Value of temperature after filter is ", filterValue);

    if (filterValue < parseFloat(Deno.env["threshold-value"])) {
        Device.setDeviceStatus("target-device-name", "status", "1")
    } else {
        Device.setDeviceStatus("target-device-name", "status", "0")
    }
    await Device.commitDevice("target-device-name")
}
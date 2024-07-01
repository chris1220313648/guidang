async function filterService(value) {
    const url = Deno.env["filter_service_url"];
    console.log(url);
    return await Deno.httpGet(`${url}?value=${value}`, "");
}

export async function main() {
    const illumination = parseFloat(Device.getDeviceStatus("illumination", "illumination"))
    console.log("Value of illumination from sensor is ", illumination)

    const filterValue = await filterService(illumination);
    console.log("Value of illumination after filter is ", filterValue)

    let value;
    if (parseFloat(filterValue) < parseFloat(Deno.env["threshold"])) {
        value = "768"
    } else {
        value = "1024"
    }
    Device.setDeviceStatus("motor", "control-state", value)
    await Device.commitDevice("motor")
    console.info("Script Exit!")
}
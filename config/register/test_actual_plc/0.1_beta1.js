
async function main() {
    const [axis_actualVelocityStr, http_url] = Deno.getAbilityStatus("ability-sensor-name", "axis_actualVelocity");
    console.log("Value of plc from http is ", axis_actualVelocityStr);
    const axis_actualVelocity = parseFloat(axis_actualVelocityStr);
    if (axis_actualVelocity < 5) {
        // 获取目标设备的 HTTP URL（假设只包含 IP 和端口）
        
    
        // 检查并确保 URL 包含协议（假设是 HTTP）
        const full_url =  "http://127.0.0.1:5200/log";
    
        // 调用 Deno.httpPost，将 Content-Type 设置为 application/json
        await Deno.httpPost(full_url, "", JSON.stringify({
            'Content-Type': 'application/json' // 设置正确的 Content-Type
        }));
    }
}

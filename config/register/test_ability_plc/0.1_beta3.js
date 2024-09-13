async function main() {
    const [isonline, http_url] = Deno.getAbilityStatus("ability-sensor-name", "isonline");
    console.log("Value of plc from sensor is ", isonline);
    if (isonline == "0") {
        // 获取目标设备的 HTTP URL（假设只包含 IP 和端口）
        const http_url = await Deno.getAbilityUrl("ability-target-name");
    
        // 检查并确保 URL 包含协议（假设是 HTTP）
        const full_url = http_url.startsWith("http://") || http_url.startsWith("https://") 
            ? `${http_url}/ability/change_status/alertforplc`
            : `http://${http_url}/ability/change_status/alertforplc`;
    
        // 构造请求体（status 为 "1"）
        const body = JSON.stringify({ "status": "1" });
    
        // 调用 Deno.httpPost，将 Content-Type 设置为 application/json
        await Deno.httpPost(full_url, body, JSON.stringify({
            'Content-Type': 'application/json' // 设置正确的 Content-Type
        }));
    }
    
    
}
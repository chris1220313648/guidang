async function main() {
    //获取环境变量
    const thresholdValue = parseFloat(Deno.env.get("threshold-value"));
    // 获取可写能力url的输出路由，可读能力url的触发属性
    const [valueStr, http_url_sensor] = Deno.getAbilityStatus("ability-sensor-name", "axis_actualPosition");
    const [router, http_url_target] = Deno.getAbilityStatus("ability-target-name", "router");
    console.log("Value of plc from http is ", valueStr);
    const value = parseFloat(valueStr);
    // 获取当前时间，并转换为北京时间
    const date = new Date();
    const options = {
        timeZone: 'Asia/Shanghai',
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
    };
    // 使用 toLocaleString 格式化时间为 YYYY/MM/DD, HH:MM:SS
    const beijingTime = date.toLocaleString('zh-CN', options);
    // 使用正则表达式和模板字符串进行格式化
    const formattedTime = beijingTime.replace(
        /(\d{4})\/(\d{2})\/(\d{2}), (\d{2}):(\d{2}):(\d{2})/,
        '$1年$2月$3日 $4时$5分$6秒'
    );
    // 能力框架的报警路由
    let http_fullurl_target = http_url_target + router;
    if (value > thresholdValue) {
        // 当超过阈值时，发送报警信息
        const message = {
            timestamp: formattedTime,
            message: "PLC position is too far.",
        };
        // 调用 Deno.httpPost，将 Content-Type 设置为 application/json
        await Deno.httpPost(
            http_fullurl_target,
            JSON.stringify(message), // 直接将 message 对象转换为 JSON 字符串
            JSON.stringify({'Content-Type': 'application/json'})
        );
    }
}
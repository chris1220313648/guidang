async function main() {
    // 获取速度值和 HTTP URL
    const full_url = "http://192.168.1.204:5000/alert";
    const thresholdValue = 60;
    const [axis_actualVelocityStr, http_url] = Deno.getAbilityStatus("ability-sensor-name", "axis_actualVelocity");
    console.log("Value of plc from http is ", axis_actualVelocityStr);
    
    const axis_actualVelocity = parseFloat(axis_actualVelocityStr);
    
    // 设置报警阈值
    

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

    // 目标设备的完整 HTTP URL
    

    if (axis_actualVelocity >=thresholdValue) {
        // 当速度超过阈值时，发送报警信息
        const message = {
            timestamp: formattedTime,
            message: `Alarm:PLC motors velocity is too fast,current velocity is ${axis_actualVelocity}`,
            // velocity: axis_actualVelocity
        };

        // 调用 Deno.httpPost，将 Content-Type 设置为 application/json
        await Deno.httpPost(
            full_url, 
            JSON.stringify(message), // 直接将 message 对象转换为 JSON 字符串
            JSON.stringify({ 'Content-Type': 'application/json' })
        );
    } 
    // else {
    //     // 当速度未超过阈值时，发送正常信息
    //     const message = {
    //         timestamp: formattedTime,
    //         message: "test：PLC velocity is normal",
    //         velocity: axis_actualVelocity
    //     };

    //     // 调用 Deno.httpPost，将 Content-Type 设置为 application/json
    //     await Deno.httpPost(
    //         full_url, 
    //         JSON.stringify(message), // 直接将 message 对象转换为 JSON 字符串
    //         JSON.stringify({ 'Content-Type': 'application/json' })
    //     );
    // }
}
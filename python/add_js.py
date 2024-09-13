import os

def create_ability_scripts(base_path):
    # 确保 base_path 存在
    if not os.path.exists(base_path):
        os.makedirs(base_path)
    
    for i in range(1, 11):
        file_name = f"test_ability_{i}/0.1_beta{i+3}.js"
        file_path = os.path.join(base_path, file_name)
        os.makedirs(os.path.dirname(file_path), exist_ok=True)
        
        # JS 脚本内容
        js_content = f"""
async function main() {{
    const [isonline, http_url] = Deno.getAbilityStatus("ability-sensor-name", "isonline");
    console.log("Value of ability {i} from sensor is ", isonline);
    if (isonline == "0") {{
        // 获取目标设备的 HTTP URL（假设只包含 IP 和端口）
        const http_url = await Deno.getAbilityUrl("ability-target-name");
    
        // 检查并确保 URL 包含协议（假设是 HTTP）
        const full_url = http_url.startsWith("http://") || http_url.startsWith("https://") 
            ? `${{http_url}}/ability/change_status/alertforability{i}`
            : `http://${{http_url}}/ability/change_status/alertforability{i}`;
    
        // 构造请求体（status 为 "1"）
        const body = JSON.stringify({{ "status": "1" }});
    
        // 调用 Deno.httpPost，将 Content-Type 设置为 application/json
        await Deno.httpPost(full_url, body, JSON.stringify({{
            'Content-Type': 'application/json' // 设置正确的 Content-Type
        }}));
    }}
}}
"""
        
        # 将内容写入文件
        with open(file_path, "w") as file:
            file.write(js_content)
        print(f"Created: {file_path}")

if __name__ == "__main__":
    base_directory = '../config/register'  # 根据您的路径调整
    create_ability_scripts(base_directory)

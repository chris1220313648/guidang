import requests
import os
import yaml  # 用于解析 YAML 内容
from add_script_from_yaml import insert_or_update_script_ability  # 导入 insert_script_ability 函数

def send_command_to_api(command):
    url = "http://10.5.80.249:10980/code-generate/api/rule-engine"
    headers = {
        "Content-Type": "application/json"
    }
    payload = {
        "question": command
    }
    
    try:
        # 发送 POST 请求
        response = requests.post(url, json=payload, headers=headers, timeout=10)
        
        # 检查请求是否成功
        if response.status_code == 200:
            # 返回API响应的内容
            return response.json()
        else:
            return f"请求失败，状态码: {response.status_code}, 信息: {response.text}"
    
    except Exception as e:
        return f"请求失败，错误信息: {str(e)}"

def display_result(result, custom_ability_name=None):
    # 检查响应内容是否包含 'answer' 键
    if isinstance(result, dict) and 'answer' in result:
        answer = result['answer']
        
        # 尝试从 YAML 内容中提取属性
        if 'yaml' in answer:
            yaml_content = answer['yaml']
            yaml_data = yaml.safe_load(yaml_content)  # 解析 YAML 内容
            
            # 使用用户提供的脚本名字，如果有的话；否则从 YAML 数据中获取
            ability_name = custom_ability_name if custom_ability_name else yaml_data.get("ability_name", "default_name")
            
            # 更新 ability_name 字段
            yaml_data["ability_name"] = ability_name
            
            # 确保 `execute_policy` 中的子项正确嵌套
            if "execute_policy" in yaml_data:
                execute_policy = yaml_data["execute_policy"]
                yaml_data["execute_policy"] = {
                    "cron": yaml_data.get("cron", ""),
                    "qos": yaml_data.get("qos", "AtMostOnce"),
                    "read_change": yaml_data.get("read_change", True),
                    "webhook": yaml_data.get("webhook", True)
                }
            print(yaml_data)

            # 将 YAML 数据插入数据库
            insert_or_update_script_ability(yaml_data)
            
            # 创建目录
            os.makedirs(ability_name, exist_ok=True)
            
            # 保存 YAML 文件
            file_name_yaml = f"{ability_name}.yaml"
            yaml_path = os.path.join(ability_name, file_name_yaml)
            with open(yaml_path, "w", encoding="utf-8") as yaml_file:
                yaml.dump(yaml_data, yaml_file, allow_unicode=True)
            print(f"\nYAML 部分已保存到 {yaml_path} 文件中。\n")

        # 保存 JavaScript 内容
        if 'javascript' in answer:
            javascript_content = answer['javascript']
            script_version = yaml_data.get("script_version", "0.1_beta1")
            file_name_js = f"{script_version}.js"
            
            # 构建路径并创建目录
            js_directory = os.path.join("../config/register", ability_name)
            os.makedirs(js_directory, exist_ok=True)
            
            # 保存 JavaScript 文件
            js_path = os.path.join(js_directory, file_name_js) 
            with open(js_path, "w", encoding="utf-8") as js_file:
                js_file.write(javascript_content)
            print(f"\nJavaScript 部分已保存到 {js_path} 文件中。\n")

if __name__ == "__main__":
    while True:
        # 接受用户输入
        command_input = input("请输入指令（格式'脚本名字:指令'，或仅输入指令，输入'exit'退出）: ")
        
        # 判断是否退出
        if command_input.lower() == "exit":
            print("程序已退出")
            break
        
        # 解析输入，分离脚本名字和指令
        if ":" in command_input:
            custom_ability_name, command = command_input.split(":", 1)
        else:
            custom_ability_name, command = None, command_input
        
        # 发送请求并格式化输出结果
        result = send_command_to_api(command)
        display_result(result, custom_ability_name)

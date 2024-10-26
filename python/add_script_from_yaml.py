import sqlite3
import yaml

def insert_or_update_script_ability(yaml_data):
    # 从 YAML 数据中提取字段
    ability_name = yaml_data.get("ability_name", "default_name")
    script_type = yaml_data.get("script_type", "Js")
    script_version = yaml_data.get("script_version", "0.1")
    elapsed_time = yaml_data.get("elapsed_time", 0)
    last_run = yaml_data.get("last_run", 0)
    message = yaml_data.get("message", "")
    status = yaml_data.get("status", 0)
    register = yaml_data.get("register", "")

    # 连接到数据库
    conn = sqlite3.connect('../test.db')
    cursor = conn.cursor()
    
    # 检查是否存在同名脚本
    cursor.execute("SELECT ScriptID FROM Script WHERE Name = ?", (ability_name,))
    existing_record = cursor.fetchone()
    
    if existing_record:
        # 如果存在，则执行 UPDATE 操作
        script_id = existing_record[0]
        cursor.execute("""
            UPDATE Script 
            SET ScriptType = ?, Version = ?, ElapsedTime = ?, LastRun = ?, Message = ?, Status = ?, Register = ? 
            WHERE ScriptID = ?
        """, (script_type, script_version, elapsed_time, last_run, message, status, register, script_id))
        print(f"Data updated for {ability_name} with ScriptID: {script_id}")
    else:
        # 如果不存在，则执行 INSERT 操作
        cursor.execute("""
            INSERT INTO Script (Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status, Register) 
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """, (ability_name, script_type, script_version, elapsed_time, last_run, message, status, register))
        
        # 获取插入的 ScriptID
        script_id = cursor.lastrowid
        print(f"Data inserted successfully for {ability_name} with ScriptID: {script_id}")
    
    # 更新或插入 ExecutePolicies 表数据
    execute_policy = yaml_data.get("execute_policy", {}) or {}  # 使用空字典作为默认值
    print(execute_policy)
    if existing_record:
        cursor.execute("DELETE FROM ExecutePolicies WHERE ScriptID = ?", (script_id,))
    cursor.execute("""
        INSERT OR REPLACE INTO ExecutePolicies (ScriptID, Cron, QoS, ReadChange, Webhook) 
        VALUES (?, ?, ?, ?, ?)
    """, (script_id, execute_policy.get("cron", ""), execute_policy.get("qos", "AtMostOnce"),
          execute_policy.get("read_change", True), execute_policy.get("webhook", True)))
    
    # 更新或插入 Selector 表数据
    selectors = yaml_data.get("selectors", [])
    if existing_record:
        cursor.execute("DELETE FROM Selector WHERE ScriptID = ?", (script_id,))
    for selector in selectors:
        type_ = selector.get("type", "")
        match_ability = selector.get("match_ability", "")
        match_name = selector.get("match_name", "")
        cursor.execute("""
            INSERT INTO Selector (ScriptID, Type, MatchAbility, MatchNames) 
            VALUES (?, ?, ?, ?)
        """, (script_id, type_, match_ability, match_name))

    # 更新或插入 EnvironmentVariables 表数据
    envs = yaml_data.get('env', {})
    if existing_record:
        cursor.execute("DELETE FROM EnvironmentVariables WHERE ScriptID = ?", (script_id,))
    for key, value in envs.items():
        cursor.execute("""
            INSERT INTO EnvironmentVariables (ScriptID, Key, Value) 
            VALUES (?, ?, ?)
        """, (script_id, key, value))

    # 提交事务
    conn.commit()
    
    # 关闭连接
    conn.close()

def load_yaml_and_insert_or_update(yaml_content):
    # 解析整个 YAML 文件内容
    yaml_data = yaml.safe_load(yaml_content)
    
    # 插入或更新数据库
    insert_or_update_script_ability(yaml_data)

if __name__ == "__main__":
    # 假设从 API 获得的 YAML 内容
    yaml_content = """
    ability_name: test_actual_plc
    script_type: Js
    script_version: 0.1_beta1
    env:
      threshold-value: "1500"
    selectors:
      - type: readSelector
        match_ability: "ability-sensor-name:plc"
        match_name: ""
      - type: writeSelector
        match_ability: "ability-target-name:alert_log"
        match_name: ""
    elapsed_time: 0
    last_run: 0
    message: ""
    status: 0
    register: ""
    execute_policy:
      cron: ""
      qos: AtMostOnce
      read_change: true
      webhook: true
    """
    
    # 加载 YAML 内容并插入或更新数据库
    load_yaml_and_insert_or_update(yaml_content)

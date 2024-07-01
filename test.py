import sqlite3

def insert_script_data():
    # 连接到数据库
    conn = sqlite3.connect('./test.db')
    cursor = conn.cursor()
    
    # 插入 Script 表数据
    cursor.execute("""
        INSERT INTO Script (Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status) 
        VALUES (?, ?, ?, ?, ?, ?, ?)
    """, ('test', 'Js', '0.1_beta1', 0, 0, '', 0))
    
    # 获取插入的 ScriptID
    script_id = cursor.lastrowid
    
    # 插入 EnvironmentVariables 表数据
    env_vars = [
        ('filter_service_url', 'http://127.0.0.1:8003/api/v1alpha1/filter'),
        ('threshold-value', '40')
    ]
    
    for key, value in env_vars:
        cursor.execute("""
            INSERT INTO EnvironmentVariables (ScriptID, Key, Value) 
            VALUES (?, ?, ?)
        """, (script_id, key, value))
    
    # 插入 ExecutePolicies 表数据
    cursor.execute("""
        INSERT INTO ExecutePolicies (ScriptID, Cron, QoS, ReadChange, Webhook) 
        VALUES (?, ?, ?, ?, ?)
    """, (script_id, '', 'AtMostOnce', True, True))
    
    # 插入 Selector 表数据
    selectors = [
        ('readSelector', 'matchNames', 'temp-sensor-name:dht11'),
        ('writeSelector', 'matchNames', 'target-device-name:switch')
    ]
    
    for type_, match_types, match_names in selectors:
        cursor.execute("""
            INSERT INTO Selector (ScriptID, Type, MatchTypes, MatchNames) 
            VALUES (?, ?, ?, ?)
        """, (script_id, type_, match_types, match_names))
    
    # 提交事务
    conn.commit()
    
    # 关闭连接
    conn.close()
    print(f"Data inserted successfully with ScriptID: {script_id}")

if __name__ == "__main__":
    insert_script_data()

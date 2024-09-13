import sqlite3

def insert_script_device():
    # 连接到数据库
    conn = sqlite3.connect('../test.db')
    cursor = conn.cursor()
    
    # 插入 Script 表数据
    cursor.execute("""
        INSERT INTO Script (Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status,Register) 
        VALUES (?, ?, ?, ?, ?, ?, ?,?)
    """, ('test', 'Js', '0.1_beta1', 0, 0, '', 0,''))
    
    # 获取插入的 ScriptID
    script_id = cursor.lastrowid
    
    # # 插入 EnvironmentVariables 表数据
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
        ('readSelector', '','temp-sensor-name: dht11' ),
        ('writeSelector', '','target-device-name: switch' )
    ]
    
    for type_, match_types, match_ability in selectors:
        cursor.execute("""
            INSERT INTO Selector (ScriptID, Type, MatchAbility, MatchNames) 
            VALUES (?, ?, ?, ?)
        """, (script_id, type_, match_types, match_ability))
    
    # 提交事务
    conn.commit()
    
    # 关闭连接
    conn.close()
    print(f"Data inserted successfully with ScriptID: {script_id}")

def insert_script_ability_camera():
    # 连接到数据库
    conn = sqlite3.connect('../test.db')
    cursor = conn.cursor()
    
    # 插入 Script 表数据
    cursor.execute("""
        INSERT INTO Script (Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status,Register) 
        VALUES (?, ?, ?, ?, ?, ?, ?,?)
    """, ('test_ability', 'Js', '0.1_beta2', 0, 0, '', 0,''))
    
    # 获取插入的 ScriptID
    script_id = cursor.lastrowid
    
    # 插入 ExecutePolicies 表数据
    cursor.execute("""
        INSERT INTO ExecutePolicies (ScriptID, Cron, QoS, ReadChange, Webhook) 
        VALUES (?, ?, ?, ?, ?)
    """, (script_id, '', 'AtMostOnce', True, True))
    
    # 插入 Selector 表数据
    selectors = [
        ('readSelector', 'ability-sensor-name:sensor','' ),
        ('writeSelector', 'ability-target-name:executor','' )
    ]
    
    for type_, match_ability, match_name in selectors:
        cursor.execute("""
            INSERT INTO Selector (ScriptID, Type, MatchAbility, MatchNames) 
            VALUES (?, ?, ?, ?)
        """, (script_id, type_, match_ability, match_name))
    
    # 提交事务
    conn.commit()
    
    # 关闭连接
    conn.close()
    print(f"Data inserted successfully with ScriptID: {script_id}")
def insert_script_ability_plc():
    # 连接到数据库
    conn = sqlite3.connect('../test.db')
    cursor = conn.cursor()
    
    # 插入 Script 表数据
    cursor.execute("""
        INSERT INTO Script (Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status,Register) 
        VALUES (?, ?, ?, ?, ?, ?, ?,?)
    """, ('test_ability_plc', 'Js', '0.1_beta3', 0, 0, '', 0,''))
    
    # 获取插入的 ScriptID
    script_id = cursor.lastrowid
    
    # 插入 ExecutePolicies 表数据
    cursor.execute("""
        INSERT INTO ExecutePolicies (ScriptID, Cron, QoS, ReadChange, Webhook) 
        VALUES (?, ?, ?, ?, ?)
    """, (script_id, '', 'AtMostOnce', True, True))
    
    # 插入 Selector 表数据
    selectors = [
        ('readSelector', 'ability-sensor-name:plc_sensor','' ),
        ('writeSelector', 'ability-target-name:plc_executor','' )
    ]
    
    for type_, match_ability, match_name in selectors:
        cursor.execute("""
            INSERT INTO Selector (ScriptID, Type, MatchAbility, MatchNames) 
            VALUES (?, ?, ?, ?)
        """, (script_id, type_, match_ability, match_name))
    
    # 提交事务
    conn.commit()
    
    # 关闭连接
    conn.close()
    print(f"Data inserted successfully with ScriptID: {script_id}")
if __name__ == "__main__":
    insert_script_device()
    insert_script_ability_camera()
    insert_script_ability_plc()

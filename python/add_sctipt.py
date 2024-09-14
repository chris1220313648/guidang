import sqlite3

def insert_script_ability(ability_name, sensor_name, target_name, script_version):
    # 连接到数据库
    conn = sqlite3.connect('../test.db')
    cursor = conn.cursor()
    
    # 插入 Script 表数据
    cursor.execute("""
        INSERT INTO Script (Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status, Register) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    """, (ability_name, 'Js', script_version, 0, 0, '', 0, ''))
    
    # 获取插入的 ScriptID
    script_id = cursor.lastrowid
    
    # 插入 ExecutePolicies 表数据
    cursor.execute("""
        INSERT INTO ExecutePolicies (ScriptID, Cron, QoS, ReadChange, Webhook) 
        VALUES (?, ?, ?, ?, ?)
    """, (script_id, '', 'AtMostOnce', True, True))
    
    # 插入 Selector 表数据
    selectors = [
        ('readSelector', f'ability-sensor-name:{sensor_name}', ''),
        ('writeSelector', f'ability-target-name:{target_name}', '')
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
    print(f"Data inserted successfully for {ability_name} with ScriptID: {script_id}")

if __name__ == "__main__":
    # 插入设备脚本
    # insert_script_ability('test_ability_camera', 'sensor', 'executor', '0.1_beta2')
    # insert_script_ability('test_ability_plc', 'plc_sensor', 'plc_executor', '0.1_beta3')

    # 插入10个新能力的脚本
    for i in range(11, 31):
        insert_script_ability(f'test_ability_{i}', f'ability{i}_sensor', f'ability{i}_executor', f'0.1_beta{i+3}')

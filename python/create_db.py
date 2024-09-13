import sqlite3

# 创建数据库连接
conn = sqlite3.connect('../test.db')
cursor = conn.cursor()

# SQL 创建表和触发器的语句
create_tables_and_triggers_sql = """
-- 创建脚本表
CREATE TABLE IF NOT EXISTS Script (
    ScriptID INTEGER PRIMARY KEY AUTOINCREMENT,
    Name TEXT,
    ScriptType TEXT,
    Version TEXT,
    ElapsedTime INTEGER,
    LastRun INTEGER,
    Message TEXT,
    Status INTEGER,
    Register TEXT
);

-- 创建环境变量表
CREATE TABLE IF NOT EXISTS EnvironmentVariables (
    EnvID INTEGER PRIMARY KEY AUTOINCREMENT,
    ScriptID INTEGER,
    Key TEXT,
    Value TEXT,
    FOREIGN KEY (ScriptID) REFERENCES Script(ScriptID) ON DELETE CASCADE
);

-- 创建执行策略表
CREATE TABLE IF NOT EXISTS ExecutePolicies (
    PolicyID INTEGER PRIMARY KEY AUTOINCREMENT,
    ScriptID INTEGER,
    Cron TEXT,
    QoS TEXT,
    ReadChange BOOLEAN,
    Webhook BOOLEAN,
    FOREIGN KEY (ScriptID) REFERENCES Script(ScriptID) ON DELETE CASCADE
);

-- 创建选择器表
CREATE TABLE IF NOT EXISTS Selector (
    SelectorID INTEGER PRIMARY KEY AUTOINCREMENT,
    ScriptID INTEGER,
    Type TEXT,
    MatchAbility TEXT,
    MatchNames TEXT,
    FOREIGN KEY (ScriptID) REFERENCES Script(ScriptID) ON DELETE CASCADE
);

-- 创建事件日志表
CREATE TABLE IF NOT EXISTS EventLog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    script_id INTEGER,
    event_type TEXT,
    event_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(script_id) REFERENCES Script(ScriptID)
);

-- 创建设备日志表
CREATE TABLE IF NOT EXISTS DeviceLog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建触发器
-- 插入脚本事件日志触发器
CREATE TRIGGER IF NOT EXISTS trg_script_insert
AFTER INSERT ON Script
BEGIN
    INSERT INTO EventLog (script_id, event_type)
    VALUES (new.ScriptID, 'Inserted');
END;

-- 更新脚本事件日志触发器
CREATE TRIGGER IF NOT EXISTS trg_script_update
AFTER UPDATE ON Script
BEGIN
    INSERT INTO EventLog (script_id, event_type)
    VALUES (new.ScriptID, 'Updated');
END;

-- 删除脚本事件日志触发器
CREATE TRIGGER IF NOT EXISTS trg_script_delete
AFTER DELETE ON Script
BEGIN
    INSERT INTO EventLog (script_id, event_type)
    VALUES (old.ScriptID, 'Deleted');
END;

-- 插入设备事件日志触发器
CREATE TRIGGER IF NOT EXISTS trg_device_insert
AFTER INSERT ON Device
BEGIN
    INSERT INTO DeviceLog (device_id, event_type)
    VALUES (new.id, 'Inserted');
END;

-- 更新设备事件日志触发器
CREATE TRIGGER IF NOT EXISTS trg_device_update
AFTER UPDATE ON Device
BEGIN
    INSERT INTO DeviceLog (device_id, event_type)
    VALUES (new.id, 'Updated');
END;

-- 删除设备事件日志触发器
CREATE TRIGGER IF NOT EXISTS trg_device_delete
AFTER DELETE ON Device
BEGIN
    INSERT INTO DeviceLog (device_id, event_type)
    VALUES (old.id, 'Deleted');
END;

-- 插入Twins事件日志触发器
CREATE TRIGGER IF NOT EXISTS trg_twins_insert
AFTER INSERT ON Twins
BEGIN
    INSERT INTO DeviceLog (device_id, event_type)
    VALUES (new.device_id, 'Inserted');
END;

-- 更新Twins事件日志触发器
CREATE TRIGGER IF NOT EXISTS trg_twins_update
AFTER UPDATE ON Twins
BEGIN
    INSERT INTO DeviceLog (device_id, event_type)
    VALUES (new.device_id, 'Updated');
END;

-- 删除Twins事件日志触发器
CREATE TRIGGER IF NOT EXISTS trg_twins_delete
AFTER DELETE ON Twins
BEGIN
    INSERT INTO DeviceLog (device_id, event_type)
    VALUES (old.device_id, 'Deleted');
END;
"""

# 执行SQL语句
cursor.executescript(create_tables_and_triggers_sql)

# 提交更改并关闭连接
conn.commit()
conn.close()

print("Tables and triggers created successfully.")

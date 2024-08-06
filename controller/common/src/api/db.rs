use rusqlite::{params, Connection, Result};
use crate::api::script_sqlite3::{Script, EnvironmentVariable, ExecutePolicy, Selector};
use std::fs::File;
use std::io::Read;
use std::io::Write;
use serde_yaml::Value;
use serde_yaml::Mapping;
use std::error::Error;
use rusqlite::NO_PARAMS;

use super::script_sqlite3::ScriptSqlite3;

fn create_database_tables(conn: &Connection) -> Result<()> {
    let sql = "
        CREATE TABLE IF NOT EXISTS Script (
            ScriptID INTEGER PRIMARY KEY AUTOINCREMENT,
            Name TEXT,
            ScriptType TEXT,
            Version TEXT,
            ElapsedTime INTEGER,
            LastRun INTEGER,
            Message TEXT,
            Status INTEGER
        );
        CREATE TABLE IF NOT EXISTS EnvironmentVariables (
            EnvID INTEGER PRIMARY KEY AUTOINCREMENT,
            ScriptID INTEGER,
            Key TEXT,
            Value TEXT,
            FOREIGN KEY (ScriptID) REFERENCES Script(ScriptID) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS ExecutePolicies (
            PolicyID INTEGER PRIMARY KEY AUTOINCREMENT,
            ScriptID INTEGER,
            Cron TEXT,
            QoS TEXT,
            ReadChange BOOLEAN,
            Webhook BOOLEAN,
            FOREIGN KEY (ScriptID) REFERENCES Script(ScriptID) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS Selector (
            SelectorID INTEGER PRIMARY KEY AUTOINCREMENT,
            ScriptID INTEGER,
            Type TEXT,
            MatchTypes TEXT,
            MatchNames TEXT,
            FOREIGN KEY (ScriptID) REFERENCES Script(ScriptID) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS EventLog (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            script_id INTEGER,
            event_type TEXT,
            event_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(script_id) REFERENCES Script(ScriptID)
        );
        CREATE TRIGGER IF NOT EXISTS trg_script_insert
        AFTER INSERT ON Script
        BEGIN
            INSERT INTO EventLog (script_id, event_type)
            VALUES (new.ScriptID, 'Inserted');
        END;
        -- 更新事件
        CREATE TRIGGER IF NOT EXISTS trg_script_update
        AFTER UPDATE ON Script
        BEGIN
            INSERT INTO EventLog (script_id, event_type)
            VALUES (new.ScriptID, 'Updated');
        END;

        -- 删除事件
        CREATE TRIGGER IF NOT EXISTS trg_script_delete
        AFTER DELETE ON Script
        BEGIN
            INSERT INTO EventLog (script_id, event_type)
            VALUES (old.ScriptID, 'Deleted');
        END;
        -- 创建设备事件日志表
        CREATE TABLE IF NOT EXISTS DeviceLog (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            event_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );

        -- 插入事件触发器
        CREATE TRIGGER IF NOT EXISTS trg_device_insert
        AFTER INSERT ON Device
        BEGIN
            INSERT INTO DeviceLog (device_id, event_type)
            VALUES (new.id, 'Inserted');
        END;

        -- 更新事件触发器
        CREATE TRIGGER IF NOT EXISTS trg_device_update
        AFTER UPDATE ON Device
        BEGIN
            INSERT INTO DeviceLog (device_id, event_type)
            VALUES (new.id, 'Updated');
        END;

        -- 删除事件触发器
        CREATE TRIGGER IF NOT EXISTS trg_device_delete
        AFTER DELETE ON Device
        BEGIN
            INSERT INTO DeviceLog (device_id, event_type)
            VALUES (old.id, 'Deleted');
        END;
        -- 插入twin事件触发器
        CREATE TRIGGER IF NOT EXISTS trg_twins_insert
        AFTER INSERT ON Twins
        BEGIN
            INSERT INTO DeviceLog (device_id, event_type)
            VALUES (new.device_id, 'Inserted');
        END;

        -- 更新事件触发器
        CREATE TRIGGER IF NOT EXISTS trg_twins_update
        AFTER UPDATE ON Twins
        BEGIN
            INSERT INTO DeviceLog (device_id,  event_type)
            VALUES ( new.device_id, 'Updated');
        END;

        -- 删除事件触发器
        CREATE TRIGGER IF NOT EXISTS trg_twins_delete
        AFTER DELETE ON Twins
        BEGIN
            INSERT INTO DeviceLog ( device_id,  event_type)
            VALUES ( old.device_id, 'Deleted');
        END;

       
    ";
    conn.execute_batch(sql)?;
    Ok(())
}
fn insert_loop(conn: &Connection) -> Result<()> {
    // 插入Script表的基本信息
    conn.execute(
        "INSERT INTO Script (Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params!["test", "Js", "0.1_beta1", 0, 0, "", 0],
    )?;
    let script_id = conn.last_insert_rowid();

    // 插入EnvironmentVariables表
    let env_vars = [
        ("filter_service_url", "http://127.0.0.1:8003/api/v1alpha1/filter"),
        ("threshold-value", "40"),
    ];
    for (key, value) in &env_vars {
        conn.execute(
            "INSERT INTO EnvironmentVariables (ScriptID, Key, Value) 
             VALUES (?1, ?2, ?3)",
            params![script_id, *key, *value],
        )?;
    }

    // 插入ExecutePolicies表
    conn.execute(
        "INSERT INTO ExecutePolicies (ScriptID, Cron, QoS, ReadChange, Webhook) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![script_id, "", "AtMostOnce", true, true],
    )?;

    // 插入Selector表（ReadSelector）
    conn.execute(
        "INSERT INTO Selector (ScriptID, Type, MatchTypes, MatchNames) 
         VALUES (?1, ?2, ?3, ?4)",
        params![script_id, "ReadSelector", "matchNames", "temp-sensor-name:dht11"],
    )?;

    // 插入Selector表（WriteSelector）
    conn.execute(
        "INSERT INTO Selector (ScriptID, Type, MatchTypes, MatchNames) 
         VALUES (?1, ?2, ?3, ?4)",
        params![script_id, "WriteSelector", "matchNames", "target-device-name:switch"],
    )?;

    Ok(())
}
fn insert_script(conn: &Connection, script: &ScriptSqlite3) -> Result<i64> {
    let sql = "INSERT INTO Script (Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status) VALUES (?, ?, ?, ?, ?, ?, ?)";
    let mut stmt = conn.prepare(sql)?;

    stmt.execute(params![
        script.name,
        script.script_type,
        script.version,
        script.elapsed_time,
        script.last_run,
        script.message,
        script.status,
    ])?;

    let script_id = conn.last_insert_rowid();
    Ok(script_id)
}

fn insert_environment_variable(conn: &Connection, env_var: &EnvironmentVariable) -> Result<()> {
    let sql = "INSERT INTO EnvironmentVariables (ScriptID, Key, Value) VALUES (?, ?, ?)";
    let mut stmt = conn.prepare(sql)?;

    stmt.execute(params![
        env_var.script_id,
        env_var.key,
        env_var.value
    ])?;

    Ok(())
}

fn insert_execute_policy(conn: &Connection, exec_policy: &ExecutePolicy) -> Result<()> {
    conn.execute(
        "INSERT INTO ExecutePolicies (ScriptID, Cron, Qos, ReadChange, Webhook) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            exec_policy.script_id,
            exec_policy.cron,
            exec_policy.qos,
            exec_policy.read_change,
            exec_policy.webhook
        ],
    )?;
    Ok(())
}

fn insert_selector(conn: &Connection, selector: &Selector) -> Result<()> {
    conn.execute(
        "INSERT INTO Selector (ScriptID, Type, MatchTypes, MatchNames) VALUES (?1, ?2, ?3, ?4)",
        params![
            selector.script_id,
            selector.selector_type,
            selector.match_types,
            selector.match_names
        ],
    )?;
    Ok(())
}


fn import_script_from_yaml(conn: &Connection, yaml_file_path: &str) -> Result<(), Box<dyn Error>> {
    // 读取YAML文件
    let mut file = File::open(yaml_file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: serde_yaml::Value = serde_yaml::from_str(&contents)?;

    // 提取Script信息
    let script = ScriptSqlite3 {
        name: config["spec"]["manifest"]["name"].as_str().unwrap_or_default().to_string(),
        script_type: config["spec"]["manifest"]["scriptType"].as_str().unwrap_or_default().to_string(),
        version: config["spec"]["manifest"]["version"].as_str().unwrap_or_default().to_string(),
        elapsed_time: 0,  // 默认值
        last_run: 0,      // 默认值
        message: "".to_string(), // 默认值
        status: 0,        // 默认值
        register:config["spec"]["manifest"]["register"].as_str().unwrap_or_default().to_string(),
    };

    // 插入Script到数据库
    let script_id = insert_script(&conn, &script)?;

    // 提取环境变量
    if let Some(env_vars) = config["spec"]["env"].as_mapping() {
        for (key, value) in env_vars {
            let env_var = EnvironmentVariable {
                script_id: script_id as i32,
                key: key.as_str().unwrap_or_default().to_string(),
                value: value.as_str().unwrap_or_default().to_string(),
            };
            insert_environment_variable(&conn, &env_var)?;
        }
    }

    // 提取执行策略
    let exec_policy = ExecutePolicy {
        script_id: script_id as i32,
        cron: config["spec"]["executePolicy"]["cron"].as_str().unwrap_or_default().to_string(),
        qos: config["spec"]["executePolicy"]["qos"].as_str().unwrap_or_default().to_string(),
        read_change: config["spec"]["executePolicy"]["readChange"].as_bool().unwrap_or_default(),
        webhook: config["spec"]["executePolicy"]["webhook"].as_bool().unwrap_or_default(),
    };
    insert_execute_policy(&conn, &exec_policy)?;

    // 提取选择器
    let mut selector = Selector {
        script_id: script_id as i32,
        selector_type: "readSelector".to_string(),
        match_types: "".to_string(), // 假设match_types字段存在
        match_names: config["spec"]["readSelector"]["matchNames"]["temp-sensor-name"].as_str().unwrap_or_default().to_string(),
    };
    insert_selector(&conn, &selector)?;

    selector.selector_type = "writeSelector".to_string();
    selector.match_names = config["spec"]["writeSelector"]["matchNames"]["target-device-name"].as_str().unwrap_or_default().to_string();
    insert_selector(&conn, &selector)?;

    Ok(())
}


fn update_script(conn: &Connection, script: &ScriptSqlite3, script_id: i32) -> Result<()> {
    let sql = "UPDATE Script SET Name = ?, ScriptType = ?, Version = ?, ElapsedTime = ?, LastRun = ?, Message = ?, Status = ? WHERE ScriptID = ?";
    let mut stmt = conn.prepare(sql)?;

    stmt.execute(params![
        script.name,
        script.script_type,
        script.version,
        script.elapsed_time,
        script.last_run,
        script.message,
        script.status,
        script_id,
    ])?;

    Ok(())
}

fn update_environment_variable(conn: &Connection, env_var: &EnvironmentVariable, env_id: i32) -> Result<()> {
    let sql = "UPDATE EnvironmentVariables SET ScriptID = ?, Key = ?, Value = ? WHERE EnvID = ?";
    let mut stmt = conn.prepare(sql)?;

    stmt.execute(params![
        env_var.script_id,
        env_var.key,
        env_var.value,
        env_id,
    ])?;

    Ok(())
}
fn update_execute_policy(conn: &Connection, exec_policy: &ExecutePolicy, policy_id: i32) -> Result<()> {
    let sql = "UPDATE ExecutePolicies SET ScriptID = ?, Cron = ?, Qos = ?, ReadChange = ?, Webhook = ? WHERE PolicyID = ?";
    let mut stmt = conn.prepare(sql)?;

    stmt.execute(params![
        exec_policy.script_id,
        exec_policy.cron,
        exec_policy.qos,
        exec_policy.read_change as i32,
        exec_policy.webhook as i32,
        policy_id,
    ])?;

    Ok(())
}
fn update_selector(conn: &Connection, selector: &Selector, selector_id: i32) -> Result<()> {
    let sql = "UPDATE Selector SET ScriptID = ?, Type = ?, MatchTypes = ?, MatchNames = ? WHERE SelectorID = ?";
    let mut stmt = conn.prepare(sql)?;

    stmt.execute(params![
        selector.script_id,
        selector.selector_type,
        selector.match_types,
        selector.match_names,
        selector_id,
    ])?;

    Ok(())
}
fn delete_script(conn: &Connection, script_id: i32) -> Result<()> {
    let sql = "DELETE FROM Script WHERE ScriptID = ?";
    let mut stmt = conn.prepare(sql)?;

    stmt.execute(params![script_id])?;
    
    Ok(())
}
fn delete_environment_variable(conn: &Connection, env_id: i32) -> Result<()> {
    let sql = "DELETE FROM EnvironmentVariables WHERE EnvID = ?";
    let mut stmt = conn.prepare(sql)?;

    stmt.execute(params![env_id])?;
    
    Ok(())
}
fn delete_execute_policy(conn: &Connection, policy_id: i32) -> Result<()> {
    let sql = "DELETE FROM ExecutePolicies WHERE PolicyID = ?";
    let mut stmt = conn.prepare(sql)?;

    stmt.execute(params![policy_id])?;
    
    Ok(())
}
fn delete_selector(conn: &Connection, selector_id: i32) -> Result<()> {
    let sql = "DELETE FROM Selector WHERE SelectorID = ?";
    let mut stmt = conn.prepare(sql)?;

    stmt.execute(params![selector_id])?;
    
    Ok(())
}

pub fn query_script_by_id(conn: &Connection, script_id: i32) -> Result<(), Box<dyn Error>> {
    let sql = "
        SELECT s.ScriptID, s.Name, s.ScriptType, s.Version, s.ElapsedTime, s.LastRun, s.Message, s.Status, 
               e.EnvID, e.Key, e.Value, 
               p.PolicyID, p.Cron, p.QoS, p.ReadChange, p.Webhook, 
               sl.SelectorID, sl.Type, sl.MatchTypes, sl.MatchNames 
        FROM Script s 
        LEFT JOIN EnvironmentVariables e ON s.ScriptID = e.ScriptID 
        LEFT JOIN ExecutePolicies p ON s.ScriptID = p.ScriptID 
        LEFT JOIN Selector sl ON s.ScriptID = sl.ScriptID 
        WHERE s.ScriptID = ?";

    let mut stmt = conn.prepare(sql)?;

    // 在进入行处理之前获取列名
    let column_names: Vec<String> = (0..stmt.column_count())
        .map(|i| stmt.column_name(i).unwrap_or("").to_string())
        .collect();

    let mut rows = stmt.query(params![script_id])?;

    while let Some(row) = rows.next()? {
        for (i, col_name) in column_names.iter().enumerate() {
            let col_value = if let Ok(val) = row.get::<_, String>(i) {
                Some(val)
            } else if let Ok(val) = row.get::<_, i32>(i) {
                Some(val.to_string())
            } else if let Ok(val) = row.get::<_, bool>(i) {
                Some(val.to_string())
            } else {
                None
            };

            match col_value {
                Some(value) => print!("{}: {}, ", col_name, value),
                None => print!("{}: NULL, ", col_name),
            }
        }
        println!();
    }

    Ok(())
}

fn query_all(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let sql = "
        SELECT s.ScriptID, s.Name, s.ScriptType, s.Version, s.ElapsedTime, s.LastRun, s.Message, s.Status, 
               e.EnvID, e.Key, e.Value, 
               p.PolicyID, p.Cron, p.QoS, p.ReadChange, p.Webhook, 
               sl.SelectorID, sl.Type, sl.MatchTypes, sl.MatchNames 
        FROM Script s 
        LEFT JOIN EnvironmentVariables e ON s.ScriptID = e.ScriptID 
        LEFT JOIN ExecutePolicies p ON s.ScriptID = p.ScriptID 
        LEFT JOIN Selector sl ON s.ScriptID = sl.ScriptID";

    let mut stmt = conn.prepare(sql)?;

    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        println!(
            "ScriptID: {}, Name: {}, ScriptType: {}, Version: {}, ElapsedTime: {}, LastRun: {}, Message: {}, Status: {}, EnvID: {}, Key: {}, Value: {}, PolicyID: {}, Cron: {}, QoS: {}, ReadChange: {}, Webhook: {}, SelectorID: {}, Type: {}, MatchTypes: {}, MatchNames: {}",
            row.get::<_, i32>(0)?,  // ScriptID
            row.get::<_, String>(1)?,  // Name
            row.get::<_, String>(2)?,  // ScriptType
            row.get::<_, String>(3)?,  // Version
            row.get::<_, i32>(4)?,  // ElapsedTime
            row.get::<_, i32>(5)?,  // LastRun
            row.get::<_, String>(6)?,  // Message
            row.get::<_, i32>(7)?,  // Status
            row.get::<_, i32>(8)?,  // EnvID
            row.get::<_, String>(9)?,  // Key
            row.get::<_, String>(10)?,  // Value
            row.get::<_, i32>(11)?,  // PolicyID
            row.get::<_, String>(12)?,  // Cron
            row.get::<_, String>(13)?,  // QoS
            row.get::<_, i32>(14)?,  // ReadChange
            row.get::<_, i32>(15)?,  // Webhook
            row.get::<_, i32>(16)?,  // SelectorID
            row.get::<_, String>(17)?,  // Type
            row.get::<_, String>(18)?,  // MatchTypes
            row.get::<_, String>(19)?,  // MatchNames
        );
    }

    Ok(())
}

fn query_scripts(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let sql = "SELECT ScriptID, Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status FROM Script";
    let mut stmt = conn.prepare(sql)?;

    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let script_id: i32 = row.get(0)?;
        let name: String = row.get(1)?;
        let script_type: String = row.get(2)?;
        let version: String = row.get(3)?;
        let elapsed_time: i32 = row.get(4)?;
        let last_run: i32 = row.get(5)?;
        let message: String = row.get(6)?;
        let status: i32 = row.get(7)?;

        println!(
            "ScriptID: {}, Name: {}, ScriptType: {}, Version: {}, ElapsedTime: {}, LastRun: {}, Message: {}, Status: {}",
            script_id, name, script_type, version, elapsed_time, last_run, message, status
        );
    }

    Ok(())
}
fn query_environment_variables(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let sql = "SELECT EnvID, ScriptID, Key, Value FROM EnvironmentVariables";
    let mut stmt = conn.prepare(sql)?;

    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let env_id: i32 = row.get(0)?;
        let script_id: i32 = row.get(1)?;
        let key: String = row.get(2)?;
        let value: String = row.get(3)?;

        println!(
            "EnvID: {}, ScriptID: {}, Key: {}, Value: {}",
            env_id, script_id, key, value
        );
    }

    Ok(())
}

fn query_execute_policies(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let sql = "SELECT PolicyID, ScriptID, Cron, Qos, ReadChange, Webhook FROM ExecutePolicies";
    let mut stmt = conn.prepare(sql)?;

    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let policy_id: i32 = row.get(0)?;
        let script_id: i32 = row.get(1)?;
        let cron: String = row.get(2)?;
        let qos: String = row.get(3)?;
        let read_change: i32 = row.get(4)?;
        let webhook: i32 = row.get(5)?;

        println!(
            "PolicyID: {}, ScriptID: {}, Cron: {}, Qos: {}, ReadChange: {}, Webhook: {}",
            policy_id, script_id, cron, qos, if read_change != 0 { "Yes" } else { "No" }, if webhook != 0 { "Yes" } else { "No" }
        );
    }

    Ok(())
}

fn query_selectors(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let sql = "SELECT SelectorID, ScriptID, Type, MatchTypes, MatchNames FROM Selector";
    let mut stmt = conn.prepare(sql)?;

    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let selector_id: i32 = row.get(0)?;
        let script_id: i32 = row.get(1)?;
        let selector_type: String = row.get(2)?;
        let match_types: String = row.get(3)?;
        let match_names: String = row.get(4)?;

        println!(
            "SelectorID: {}, ScriptID: {}, Type: {}, MatchTypes: {}, MatchNames: {}",
            selector_id, script_id, selector_type, match_types, match_names
        );
    }

    Ok(())
}




fn get_triggered_environment_variables(conn: &Connection, changed_type: &str, changed_name: &str) -> Result<Vec<EnvironmentVariable>, Box<dyn Error>> {
    let mut triggered_scripts = Vec::new();
    let mut env_vars = Vec::new();

    // Step 1: 获取符合条件的ScriptID列表
    let sql_select_scripts = r#"
        SELECT DISTINCT ScriptID
        FROM ExecutePolicies
        WHERE ReadChange = 1 AND ScriptID IN (
            SELECT ScriptID
            FROM Selectors
            WHERE Type = 'Read' AND MatchTypes = ? AND MatchNames = ?
        );
    "#;

    let mut stmt = conn.prepare(sql_select_scripts)?;
    let script_rows = stmt.query_map(params![changed_type, changed_name], |row| {
        row.get(0)
    })?;

    for script_row in script_rows {
        triggered_scripts.push(script_row?);
    }

    // Step 2: 对于每个ScriptID，获取环境变量
    let sql_select_env_vars = "SELECT Key, Value FROM EnvironmentVariables WHERE ScriptID = ?";

    for script_id in triggered_scripts {
        let mut stmt = conn.prepare(sql_select_env_vars)?;
        let env_var_rows = stmt.query_map(params![script_id], |row| {
            Ok(EnvironmentVariable {
                key: row.get(0)?,
                value: row.get(1)?,
                script_id:script_id,
            })
        })?;

        for env_var_row in env_var_rows {
            env_vars.push(env_var_row?);
        }
    }

    Ok(env_vars)
}


fn fetch_data_for_script(conn: &Connection, table: &str, script_id: i32, query: &str) -> Result<Value, Box<dyn Error>> {
    let mut root = Value::Sequence(Vec::new());
    let mut stmt = conn.prepare(query)?;

    // 获取列数和列名
    let column_count = stmt.column_count();
    let column_names: Vec<String> = (0..column_count)
        .map(|i| stmt.column_name(i).unwrap().to_string())
        .collect();

    let rows = stmt.query_map(params![script_id], |row| {
        let mut node = Mapping::new();
        for (i, col_name) in column_names.iter().enumerate() {
            let col_value = if let Ok(val) = row.get::<_, String>(i) {
                Value::String(val)
            } else if let Ok(val) = row.get::<_, i64>(i) {
                Value::Number(val.into())
            } else {
                Value::Null
            };
            node.insert(Value::String(col_name.clone()), col_value);
        }
        Ok(node)
    })?;

    for row in rows {
        root.as_sequence_mut().unwrap().push(Value::Mapping(row?));
    }

    Ok(root)
}

fn export_script_data_to_yaml(conn: &Connection, script_id: i32, filename: &str) -> Result<(), Box<dyn Error>> {
    // Fetch and write details for each table related to the scriptID
    let script_data = fetch_data_for_script(&conn, "Script", script_id, "SELECT * FROM Script WHERE ScriptID = ?")?;
    let env_vars_data = fetch_data_for_script(&conn, "EnvironmentVariables", script_id, "SELECT * FROM EnvironmentVariables WHERE ScriptID = ?")?;
    let exec_policies_data = fetch_data_for_script(&conn, "ExecutePolicies", script_id, "SELECT * FROM ExecutePolicies WHERE ScriptID = ?")?;
    let selectors_data = fetch_data_for_script(&conn, "Selectors", script_id, "SELECT * FROM Selectors WHERE ScriptID = ?")?;

    // Start writing YAML content
    let mut data = Mapping::new();
    data.insert(Value::String("Script".to_string()), script_data);
    data.insert(Value::String("EnvironmentVariables".to_string()), env_vars_data);
    data.insert(Value::String("ExecutePolicies".to_string()), exec_policies_data);
    data.insert(Value::String("Selectors".to_string()), selectors_data);

    let yaml_data = Value::Mapping(data);
    let yaml_str = serde_yaml::to_string(&yaml_data)?;

    // Write to file
    let mut fout = File::create(filename)?;
    fout.write_all(yaml_str.as_bytes())?;
    println!("YAML file created for ScriptID {}: {}", script_id, filename);

    Ok(())
}
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use color_eyre::eyre::{Report, Result, WrapErr};
use tracing::info;
use crate::api::script_sqlite3::*;
use tokio::time::{interval, Duration};
use rusqlite::{params, Connection,Result as RusqliteResult};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::scheduler:: Reflector;

pub async fn reflector_sqlite3(conn: Arc<Mutex<Connection>>,reflector: Arc<Reflector>) -> Result<(), Report> {
    info!("start reflector_sqlite3");
    // 导入现有脚本信息
    import_existing_scripts(conn.clone(), reflector.clone())?;
    let _=poll_event_log_and_process_events(conn,reflector).await;
    Ok(())
    
    
}
fn import_existing_scripts(conn: Arc<Mutex<Connection>>, reflector: Arc<Reflector>) -> Result<(), Box<dyn Error>> {
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id FROM Script")?;
    
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let script_id: i32 = row.get(0)?;
        println!("Importing script: {:?}", script_id);
        let script = fetch_script_details(&conn, script_id)?;
        let env_vars = fetch_environment_variables(&conn, script_id)?;
        let execute_policy = fetch_execute_policy(&conn, script_id)?;
        let selectors = fetch_selectors(&conn, script_id)?;
        let script_struct = create_script_struct(script, env_vars, execute_policy, selectors)?;
        println!("{:?}", script_struct);
        reflector.add_script(&script_struct);
    }

    Ok(())
}
async fn poll_event_log_and_process_events(conn: Arc<Mutex<Connection>>,reflector: Arc<Reflector>) -> Result<(), Box<dyn Error>> {
    let mut last_polled = Utc::now() - chrono::Duration::seconds(30); // 记录上次轮询时间，假设10s前开始
    let poll_interval = Duration::from_secs(5); // 轮询间隔
    let mut interval = interval(poll_interval); // 定时器
    let mut count=0;

    loop {
        info!("lunxun:{}",count);
        count=count+1;
        interval.tick().await;
        let conn = conn.lock().unwrap();
        let mut table_stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='EventLog';")?;
        let table_exists: RusqliteResult<String> = table_stmt.query_row([], |row| row.get(0));
        match table_exists {
            Ok(name) => info!("Table exists: {}", name),
            Err(err) => {
                eprintln!("Table 'EventLog' does not exist: {}", err);
                continue; // 继续循环
            }
        }
        let mut stmt = match conn.prepare("
            SELECT script_id, event_type, event_time 
            FROM EventLog 
            WHERE event_time > ?
        ") {
            Ok(stmt) => stmt,
            Err(err) => {
                eprintln!("Failed to prepare statement: {}", err);
                continue; // 继续循环
            }
        };

        info!("presqlite:");
        // 确保时间格式正确
        let naive_last_polled = last_polled.naive_utc().format("%Y-%m-%d %H:%M:%S").to_string();
        info!("Using last_polled time:{}",naive_last_polled);
        info!(last_polled = %last_polled);
        println!("Using last_polled time: {}", naive_last_polled); // 调试信息
        let mut rows = stmt.query(params![naive_last_polled])?;
        let mut found = false;
        while let Some(row) = rows.next()? {
            found = true;
            let script_id: i32 = row.get(0)?;
            let event_type: String = row.get(1)?;
            let event_time: NaiveDateTime = row.get(2)?;
            println!("Event: {} for script_id: {} at {}", event_type, script_id, event_time);

            let script = fetch_script_details(&conn, script_id)?;
            let env_vars = fetch_environment_variables(&conn, script_id)?;
            let execute_policy = fetch_execute_policy(&conn, script_id)?;
            let selectors = fetch_selectors(&conn, script_id)?;
            let script_struct = create_script_struct(script, env_vars, execute_policy, selectors)?;
            println!("{:?}", script_struct);
            info!(event_type=%event_type);
            match event_type.as_str() {
                "Inserted" => {
                    // 处理创建事件的逻辑
                    println!("Handling create event for script_id: {}", script_id);
                    reflector.add_script(&script_struct)
                },
                "Updated" => {
                    // 处理更新事件的逻辑
                    println!("Handling update event for script_id: {}", script_id);
                    reflector.add_script(&script_struct)
                },
                "Deleted" => {
                    // 处理删除事件的逻辑
                    println!("Handling delete event for script_id: {}", script_id);
                    reflector.remove_script(&script_struct)
                },
                "error" => {
                    // 处理错误事件的逻辑
                    println!("Handling error event for script_id: {}", script_id);
                },
                _ => {
                    // 处理未知事件类型
                    println!("Unknown event type: {} for script_id: {}", event_type, script_id);
                }
            }      
        }
        
        if !found {
            println!("No new events found.");
        }

        last_polled = Utc::now(); // 更新上次查询时间
    }
}

fn fetch_script_details(conn: &Connection, script_id: i32) -> Result<ScriptSqlite3, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status FROM Script WHERE ScriptID = ?")?;
    let script = stmt.query_row(params![script_id], |row| {
        Ok(ScriptSqlite3 {
            name: row.get(0)?,
            script_type: row.get(1)?,
            version: row.get(2)?,
            elapsed_time: row.get(3)?,
            last_run: row.get(4)?,
            message: row.get(5)?,
            status: row.get(6)?,
        })
    })?;
    Ok(script)
}

fn fetch_environment_variables(conn: &Connection, script_id: i32) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT Key, Value FROM EnvironmentVariables WHERE ScriptID = ?")?;
    let mut rows = stmt.query(params![script_id])?;
    let mut env_vars = HashMap::new();
    while let Some(row) = rows.next()? {
        let key: String = row.get(0)?;
        let value: String = row.get(1)?;
        env_vars.insert(key, value);
    }
    Ok(env_vars)
}

fn fetch_execute_policy(conn: &Connection, script_id: i32) -> Result<Policy, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT Cron, QoS, ReadChange, Webhook FROM ExecutePolicies WHERE ScriptID = ?")?;
    let policy = stmt.query_row(params![script_id], |row| {
        let qos: String = row.get(1)?;
        let qos_policy = QosPolicy::from_str(&qos).map_err(|e| rusqlite::Error::InvalidQuery)?;
        Ok(Policy {
            cron: row.get(0)?,
            qos: qos_policy,
            read_change: row.get(2)?,
            webhook: row.get(3)?,
        })
    })?;
    Ok(policy)
}

fn fetch_selectors(conn: &Connection, script_id: i32) -> Result<(DeviceSelectorSet, DeviceSelectorSet), Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT Type, MatchTypes, MatchNames FROM Selector WHERE ScriptID = ?")?;
    let mut rows = stmt.query(params![script_id])?;
    let mut read_selector = DeviceSelectorSet {
        match_names: None,
        match_abilities: None,
    };
    let mut write_selector = DeviceSelectorSet {
        match_names: None,
        match_abilities: None,
    };
    while let Some(row) = rows.next()? {
        let selector_type: String = row.get(0)?;
        let match_types: String = row.get(1)?;
        let match_names: String = row.get(2)?;

        let match_types_map = parse_match_string(&match_types);
        let match_names_map = parse_match_string(&match_names);

        if selector_type == "readSelector" {
            read_selector.match_abilities = Some(match_types_map);
            read_selector.match_names = Some(match_names_map);
        } else if selector_type == "writeSelector" {
            write_selector.match_abilities = Some(match_types_map);
            write_selector.match_names = Some(match_names_map);
        }
    }
    Ok((read_selector, write_selector))
}
fn parse_match_string(s: &str) -> HashMap<String, String> {
    s.split(',').map(|kv| {
        let mut iter = kv.splitn(2, ':');
        let key = iter.next().unwrap_or("").to_string();
        let value = iter.next().unwrap_or("").to_string();
        (key, value)
    }).collect()
}
fn create_script_struct(
    script: ScriptSqlite3,
    env_vars: HashMap<String, String>,
    execute_policy: Policy,
    selectors: (DeviceSelectorSet, DeviceSelectorSet)
) -> Result<Script, Box<dyn Error>> {
    let script_type = ScriptType::from_str(&script.script_type)?;
    let manifest = Manifest {
        script_type,
        name: script.name.clone(),
        version: script.version.clone(),
        register: None,
    };

    let spec = ScriptSpec {
        read_selector: selectors.0,
        write_selector: selectors.1,
        env: env_vars,
        manifest,
        execute_policy,
    };

    let status = ScriptStatus {
        last_run: script.last_run as i64,
        elapsed_time: script.elapsed_time as u32,
        status: script.status,
        message: script.message,
    };

    Ok(Script {
        spec,
        status: Some(status),
    })
}

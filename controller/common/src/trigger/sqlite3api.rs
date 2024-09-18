use std::collections::HashMap;
use std::vec;
use serde_json::Value;
use std::error::Error;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::str::FromStr;
use color_eyre::eyre::{Report, Result, WrapErr};
use tracing::{info,error};
use crate::api::device_sqlite3::DeviceStatus;
use crate::api::script_sqlite3::*;
use crate::api::device_sqlite3::*;
use crate::api::ability::*;
use tokio::time::{interval, Duration};
use rusqlite::{params, Connection,Result as RusqliteResult};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::marker::PhantomData;
use flume::{Receiver, Sender};
use crate::scheduler::{Reflector, ResourceIndex};
use chrono::DateTime;
struct DeviceEvent {
    device_id: i32,
    event_type: String,
    event_time: DateTime<Utc>,
}
pub async fn reflector_sqlite3_device(conn: Arc<Mutex<Connection>>,reflector: Arc<Reflector>,scheduler: Sender<ResourceIndex<Device>>) -> Result<(), Report> {
    info!("start device_reflector");
    
    // 导入现有脚本信息
    let _=import_existing_devices(conn.clone(), reflector.clone()).await;
    let _=poll_device_event_and_process(conn,reflector,scheduler).await;
    Ok(())
    
    
}
pub async fn reflector_sqlite3(conn: Arc<Mutex<Connection>>,reflector: Arc<Reflector>) -> Result<(), Report> {
    info!("start reflector_sqlite3");
    // 导入现有脚本信息
    let _=import_existing_scripts(conn.clone(), reflector.clone()).await;
    let _=poll_script_event_and_process(conn,reflector).await;
    Ok(())
    
    
}
pub async fn reflector_ability(
    ability_flamework_url: &str,
    reflector: Arc<Reflector>,
    scheduler: Sender<ResourceIndex<Ability>>,
) -> Result<(), Report> {
    let poll_interval = Duration::from_secs(5); // 轮询间隔
    let mut interval = interval(poll_interval); // 定时器
    let running_url = format!("{}:8080/api/AbilityRunning", ability_flamework_url); // 获取正在运行的能力
    let mut count = 0;
    let abi_to_idx = |abi: &Ability| ResourceIndex {
        name: abi.name.clone(),
        namespace: "default".to_string(),
        api: PhantomData,
    };

    loop {
        info!("Polling ability count: {}", count);
        count += 1;
        interval.tick().await;

        // 发送 GET 请求到 AbilityRunning 服务器以获取正在运行的能力列表
        match reqwest::get(&running_url).await {
            Ok(response) => {
                match response.json::<Vec<Value>>().await {
                    Ok(running_abilities) => {
                        for ability_info in running_abilities {
                            // 提取能力的端口
                            if let Some(ability_port) = ability_info["abilityPort"].as_u64() {
                                let ability_name = ability_info["abilityName"].as_str().unwrap_or_default();
                                let ability_ip = ability_flamework_url; // 使用基础 URL
                                let full_url = format!("{}:{}/api/getAllAtomicInfo", ability_ip, ability_port);

                                // 查询该能力的详细信息
                                match reqwest::get(&full_url).await {
                                    Ok(detail_response) => {
                                        match detail_response.json::<Value>().await {
                                            Ok(ability_details) => {
                                                // 创建 Ability 对象并更新到 Reflector 中
                                                let mut attributes_map = HashMap::new();
                                        
                                                // 将 JSON 对象转换为 HashMap<String, serde_json::Value>
                                                if let Some(object) = ability_details.as_object() {
                                                    let mut properties = HashMap::new();
                                        
                                                    // 遍历 JSON 对象的每个键值对，并将其插入到 properties 中
                                                    for (key, value) in object {
                                                        properties.insert(key.clone(), value.clone());
                                                    }
                                        
                                                    // 创建一个 Item 并插入到 attributes_map 中
                                                    let item = Item { properties };
                                                    attributes_map.insert("Ability_attribute".to_string(), item);
                                                }
                                        
                                                // 创建 Ability 对象并更新到 Reflector 中
                                                let ability = Ability {
                                                    name: ability_name.to_string(),
                                                    http_url: format!("{}:{}", ability_ip, ability_port),
                                                    attributes: attributes_map,
                                                };
                                        
                                                info!("add ability: {:?}", ability);
                                                reflector.add_ability(&ability);
                                                let idx: ResourceIndex<Ability> = abi_to_idx(&ability);
                                                scheduler.send_async(idx).await?;
                                                info!("successfully sent ability");
                                            }
                                            Err(detail_json_err) => {
                                                eprintln!("Failed to parse ability details JSON: {}", detail_json_err);
                                            }
                                        }
                                    }
                                    Err(detail_request_err) => {
                                        eprintln!("Failed to get ability details: {}", detail_request_err);
                                    }
                                }
                            } else {
                                eprintln!("Ability port not found in running abilities.");
                            }
                        }
                    }
                    Err(json_err) => {
                        eprintln!("Failed to parse running abilities JSON: {}", json_err);
                    }
                }
            }
            Err(request_err) => {
                eprintln!("Failed to send request to AbilityRunning: {}", request_err);
            }
        }
    }

    Ok(())
}




async fn import_existing_scripts(conn: Arc<Mutex<Connection>>, reflector: Arc<Reflector>) -> Result<(), Box<dyn Error >> {
    let conn = conn.lock().await;
  
    let mut stmt = conn.prepare("SELECT ScriptID From Script;")?;

    let mut rows = stmt.query([])?;
   
    while let Some(row) = rows.next()? {
        let script_id: i32 = row.get(0)?;
        // info!("Importing script: {:?}", script_id);
        let script = fetch_script_details(&conn, script_id)?;
        // info!("get script: {:?}", script_id);
        let env_vars = fetch_environment_variables(&conn, script_id)?;
        // info!("get env script: {:?}", script_id);
        let execute_policy = fetch_execute_policy(&conn, script_id)?;
        // info!("get policy script: {:?}", script_id);
        let selectors = fetch_selectors(&conn, script_id)?;
        // info!("get selector script: {:?}", script_id);
        let script_struct = create_script_struct(script, env_vars, execute_policy, selectors)?;
        // info!("create script: {:?}", script_id);
        info!("{:?}", script_struct);
        reflector.add_script(&script_struct);
    }

    Ok(())
}
async fn poll_script_event_and_process(conn: Arc<Mutex<Connection>>,reflector: Arc<Reflector>) -> Result<(), Box<dyn Error>> {
    let mut last_polled = Utc::now() - chrono::Duration::seconds(30); // 记录上次轮询时间，假设10s前开始
    let poll_interval = Duration::from_secs(20); // 轮询间隔
    let mut interval = interval(poll_interval); // 定时器
    let mut count=0;

    loop {
        info!("Polling script count:{}",count);
        count=count+1;
        interval.tick().await;
        let conn = conn.lock().await;
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
        // 确保时间格式正确
        let naive_last_polled = last_polled.naive_utc().format("%Y-%m-%d %H:%M:%S").to_string();
        info!("Using last_polled time:{}",naive_last_polled);
        let mut rows = stmt.query(params![naive_last_polled])?;
        let mut found = false;
        while let Some(row) = rows.next()? {
            found = true;
            let script_id: i32 = row.get(0)?;
            let event_type: String = row.get(1)?;
            let event_time: NaiveDateTime = row.get(2)?;
            info!("Script Event: {} for script_id: {} at {}", event_type, script_id, event_time);

            let scriptsqlite3 = fetch_script_details(&conn, script_id)?;
            let env_vars = fetch_environment_variables(&conn, script_id)?;
            let execute_policy = fetch_execute_policy(&conn, script_id)?;
            let selectors = fetch_selectors(&conn, script_id)?;
            let script_struct = create_script_struct(scriptsqlite3, env_vars, execute_policy, selectors)?;
            info!("{:?}", script_struct);
            info!(event_type=%event_type);
            match event_type.as_str() {
                "Inserted" => {
                    // 处理创建事件的逻辑
                    info!("Handling create event for script_id: {}", script_id);
                    reflector.add_script(&script_struct)
                },
                "Updated" => {
                    // 处理更新事件的逻辑
                    info!("Handling update event for script_id: {}", script_id);
                    reflector.add_script(&script_struct)
                },
                "Deleted" => {
                    // 处理删除事件的逻辑
                    info!("Handling delete event for script_id: {}", script_id);
                    reflector.remove_script(&script_struct)
                },
                "error" => {
                    // 处理错误事件的逻辑
                    info!("Handling error event for script_id: {}", script_id);
                },
                _ => {
                    // 处理未知事件类型
                    info!("Unknown event type: {} for script_id: {}", event_type, script_id);
                }
            }      
        }
        
        if !found {
            info!("No new events found.");
        }

        last_polled = Utc::now(); // 更新上次查询时间
    }
}

fn fetch_script_details(conn: &Connection, script_id: i32) -> Result<ScriptSqlite3, Box<dyn Error>> {
    // 使用 match 处理 conn.prepare 的错误
    let mut stmt = match conn.prepare("SELECT Name, ScriptType, Version, ElapsedTime, LastRun, Message, Status, Register FROM Script WHERE ScriptID = ?") {
        Ok(statement) => statement,
        Err(e) => {
            eprintln!("Failed to prepare SQL statement: {}", e); // 打印错误
            return Err(Box::new(e)); // 返回错误
        }
    };

    // 使用 match 处理 query_row 的错误
    let script = match stmt.query_row(params![script_id], |row| {
        Ok(ScriptSqlite3 {
            name: row.get(0)?,
            script_type: row.get(1)?,
            version: row.get(2)?,
            elapsed_time: row.get(3)?,
            last_run: row.get(4)?,
            message: row.get(5)?,
            status: row.get(6)?,
            register: row.get(7)?,
        })
    }) {
        Ok(script) => script,
        Err(e) => {
            eprintln!("Failed to execute query: {}", e); // 打印错误
            return Err(Box::new(e)); // 返回错误
        }
    };

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
    let mut stmt = conn.prepare("SELECT Type, MatchAbility, MatchNames FROM Selector WHERE ScriptID = ?")?;
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
        let match_ability: String = row.get(1)?;
        let match_names: String = row.get(2)?;

        let match_anility_map = parse_match_string(&match_ability);
        let match_names_map = parse_match_string(&match_names);

        if selector_type == "readSelector" {
            read_selector.match_abilities = Some(match_anility_map);
            read_selector.match_names = Some(match_names_map);
        } else if selector_type == "writeSelector" {
            write_selector.match_abilities = Some(match_anility_map);
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
        register: Some(script.register.clone()),
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



async fn import_existing_devices(conn: Arc<Mutex<Connection>>,reflector: Arc<Reflector>) -> Result<(), Box<dyn Error>> {
    info!("import_existing_devices");
    let conn = conn.lock().await;
    let mut stmt = conn.prepare("SELECT id, name, labels, device_model_ref, node_selector FROM Device")?;
    
    let mut rows = stmt.query([])?;
    
    while let Some(row) = rows.next()? {
        let device_id: i32 = row.get(0)?;
        info!("Importing device: {:?}", device_id);
        let name: String = row.get(1)?;
        info!(" device name: {:?}", name);
        let labels: String = row.get(2)?;
        info!(" device lables: {:?}", labels);
        let device_model_name: String = row.get(3)?;
        info!(" device model_name: {:?}", device_model_name);
        let node_selector: String = row.get(4)?;
        info!(" device node_selector: {:?}", node_selector);
        
        let twins = fetch_twins_details(&conn, device_id)?;
        info!("twins:{:?}", twins);

        let device_model_ref: LocalObjectReference =LocalObjectReference{name:Some(device_model_name)} ;
        info!(" device model_name: {:?}", device_model_ref);
        
        
        let node_selector: NodeSelector = match serde_json::from_str(&node_selector) {
            Ok(ns) => {
                info!("Node selector parsed successfully: {:?}", ns);
                ns
            },
            Err(e) => {
                error!("Failed to parse node selector: {}", e);
                return Err(Box::new(e));
            }
        };
        

        let spec = DeviceSpec {
            name:name,
            device_model_ref:device_model_ref,
            property_visitors:vec![],
            node_selector:node_selector,
            data: None, 
            protocol: None,
        };
        let status:DeviceStatus=DeviceStatus{
            twins

        };
        let device = Device {
            spec,
            status: Some(status),
        };

        info!("add device:{:#?}", &device);
        reflector.add_device(&device);
        

    }
    Ok(())
 
}
async fn poll_device_event_and_process(
    conn: Arc<Mutex<Connection>>,
    reflector: Arc<Reflector>,
    scheduler: Sender<ResourceIndex<Device>>,
) -> Result<(), Box<dyn Error>> {
    let poll_interval = Duration::from_secs(10);
    let mut interval = interval(poll_interval);
    let mut last_polled = Utc::now() - chrono::Duration::seconds(30);
    let mut count=0;

    loop {
        info!("Polling device count:{}",count);
        interval.tick().await;
        let events = fetch_device_events(&conn, last_polled).await?;

        for event in events {
            process_device_event(event, &conn, &reflector, &scheduler).await?;
        }
        count=count+1;

        last_polled = Utc::now();
    }
}

async fn fetch_device_events(conn: &Arc<Mutex<Connection>>, last_polled: DateTime<Utc>) -> Result<Vec<DeviceEvent>, Box<dyn Error>> {
    let conn = conn.lock().await;
    let query = "
        SELECT device_id, event_type, event_time FROM DeviceLog WHERE event_time > ?";
    let mut stmt = conn.prepare(query)?;
    let mut rows = stmt.query(params![last_polled.naive_utc().format("%Y-%m-%d %H:%M:%S").to_string()])?;

    let mut events = Vec::new();
    while let Some(row) = rows.next()?{
        let event = DeviceEvent {
            device_id: row.get(0)?,
            event_type: row.get(1)?,
            event_time: row.get(2)?,
        };
        events.push(event);
    }

    Ok(events)
}

async fn process_device_event(
    event: DeviceEvent,
    conn: &Arc<Mutex<Connection>>,
    reflector: &Arc<Reflector>,
    scheduler: &Sender<ResourceIndex<Device>>,
) -> Result<(), Box<dyn Error>> {
    

    let device = fetch_device_details( conn,event.device_id).await?;
    let dev_to_idx = |dev: &Device| ResourceIndex {
        //定义了一个闭包dev_to_idx，它接受一个&Device引用作为参数，并返回一个ResourceIndex<Device>结构
        name: dev.spec.name.clone(),
        namespace: "default".to_string(),
        api: PhantomData,//用于表明这个结构体泛型地依赖于Device类
    };
    match event.event_type.as_str() {
        "Inserted" => {
            info!("Handling insert event for device_id: {}",event.device_id);
            reflector.add_device(&device);
            let idx = dev_to_idx(&device);
            scheduler.send_async(idx).await?;
        },
        "Updated" => {
            // 处理更新事件的逻辑
            info!("Handling update event for device_id: {}",event.device_id);
            reflector.add_device(&device);
            let idx = dev_to_idx(&device);
            scheduler.send_async(idx).await?;
        },
        "Deleted" => {
            // 处理删除事件的逻辑
            info!("Handling delete event for device_id: {}", event.device_id);
            reflector.remove_device(&device)
        },
        "error" => {
            // 处理错误事件的逻辑
            info!("Handling error event for device_id: {}", event.device_id);
        },
        _ => {
            // 处理未知事件类型
            info!("Unknown event type: {} for device_id: {}", event.event_type, event.device_id);
        }
    }

    Ok(())
}
async fn fetch_device_details(conn: &Arc<Mutex<Connection>>, device_id: i32) -> Result<Device, Box<dyn Error>> {
    let conn = conn.lock().await;
    let mut stmt = conn.prepare("SELECT id, name, labels, device_model_ref, node_selector FROM Device WHERE id=?")?;
    let mut rows = stmt.query(params![device_id])?;
    
    if let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        let labels: String = row.get(2)?; // 假设labels是一个JSON字符串
        let device_model_name: String = row.get(3)?;
        let node_selector: String = row.get(4)?;
        
        let twins = fetch_twins_details(&conn, device_id)?; // 假设这个函数已经定义好

        let device_model_ref = LocalObjectReference { name: Some(device_model_name) };
        let node_selector: NodeSelector = match serde_json::from_str(&node_selector) {
            Ok(ns) => {
                info!("Node selector parsed successfully: {:?}", ns);
                ns
            },
            Err(e) => {
                error!("Failed to parse node selector: {}", e);
                return Err(Box::new(e));
            }
        };

        let spec = DeviceSpec {
            name,
            device_model_ref,
            property_visitors: vec![],
            node_selector,
            data: None,
            protocol: None,
        };

        let status = DeviceStatus {
            twins,
        };

        Ok(Device {
            spec,
            status: Some(status),
        })
    } else {
        Err("No device found with the specified ID".into())
    }
}

fn fetch_twins_details(conn: &Connection, device_id: i32) -> Result<Vec<Twin>, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT id, device_id, property_name, desired, reported FROM Twins Where device_id=?")?;
    let mut rows = stmt.query(params![device_id])?;
    let mut twins = Vec::new();
    let mut errors = Vec::new();
    // info!("get twins");
    while let Some(row) = rows.next()? {
        let property_name: String = row.get(2)?;
        let desired: String = row.get(3)?;
        let reported: Option<String> = row.get(4)?;

        twins.push((property_name, desired, reported));
    }
    info!("get twins{:?}",twins);

    let mut result_twins = Vec::new();

    for (property_name, desired, reported) in twins {
        info!("twins_for");
        match serde_json::from_str::<TwinProperty>(&desired) {
            Ok(desired) => {
            // 判断 reported 字段是否为空，如果不为空则尝试解析
            let reported = if let Some(r) = reported {
                if  r.trim() == "{}"  { // 检查是否为空字符串
                    None
                } else {
                    match serde_json::from_str(&r) {
                        Ok(rep) => Some(rep),
                        Err(e) => {
                            info!("reported_parse_error: {:?}", e);
                            errors.push(e);
                            continue; // 发生错误时跳过当前循环
                        }
                    }
                }
            } else {
                None
            };
            
            // 没有错误，添加到结果列表
            info!("repored_ok_twins");
            result_twins.push(Twin {
                property_name,
                desired,
                reported,
            });
            }
            Err(e) => {
                info!("desired_error");
                errors.push(e);
            }
        }
    }

    if !errors.is_empty() {
        // Handle errors here, such as converting them to a Box<dyn Error>
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "JSON parsing failed")))
    } else {
        Ok(result_twins)
    }
}

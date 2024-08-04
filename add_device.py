import sqlite3
import yaml

# 定义数据库文件名
db_filename = 'test.db'



try:
    with open("./utils/mapper/temp.yaml", 'r') as file:
        data = yaml.safe_load(file)
    print("YAML data loaded successfully.")
   

    # 创建 SQLite 数据库连接
    conn = sqlite3.connect(db_filename)
    cursor = conn.cursor()
    print(f"Connected to SQLite database: {db_filename}")

    # 创建表
    cursor.execute('''
    CREATE TABLE IF NOT EXISTS Device (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT,
        labels TEXT,
        device_model_ref TEXT,
        node_selector TEXT
   
    )
    ''')
    cursor.execute('''
    CREATE TABLE IF NOT EXISTS Twins (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        device_id INTEGER,
        property_name TEXT,
        desired TEXT,
        reported TEXT,
        FOREIGN KEY(device_id) REFERENCES devices(id)
    )
    ''')
    print("Tables created successfully or already exist.")

    # 准备数据
    name = data['metadata']['name']
    labels = yaml.dump(data['metadata']['labels'])
    device_model_ref = data['spec']['deviceModelRef']['name']
    node_selector = yaml.dump(data['spec']['nodeSelector'])
   

    # 插入数据到 devices 表
    cursor.execute('''
    INSERT INTO Device (name, labels, device_model_ref, node_selector)
    VALUES (?, ?, ?, ?)
    ''', (name, labels, device_model_ref, node_selector))
    print("Device data inserted successfully.")

    device_id = cursor.lastrowid

    # 插入数据到 twins 表
    for twin in data['status']['twins']:
        property_name = twin['propertyName']
        desired = yaml.dump(twin['desired'])
        reported = yaml.dump(twin.get('reported'))
        cursor.execute('''
        INSERT INTO Twins (device_id, property_name, desired, reported)
        VALUES (?, ?, ?, ?)
        ''', (device_id, property_name, desired, reported))
    print("Twin data inserted successfully.")

    # 提交事务并关闭连接
    conn.commit()
    conn.close()
    print("Transaction committed and connection closed.")

except sqlite3.Error as e:
    print(f"SQLite error: {e}")
except yaml.YAMLError as e:
    print(f"YAML error: {e}")
except Exception as e:
    print(f"Unexpected error: {e}")

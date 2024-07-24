1.安装k8s1.22版本和kubeedge1.12版本，看原版本的readme

2.安装sqlite3

```
apt-get update && apt-get install -y \
    sqlite3 \
    libsqlite3-dev \
```
3.添加设备资源到k8s
utils/mapper/switch.model.yaml
utils/mapper/switch.yaml
utils/mapper/temp.model.yaml
utils/mapper/temp.yaml
4.启动规则引擎

```
cargo build --release
cd project_name
./target/release/cloud
./target/release/filter-server
./target/release/deno_executor  "http://127.0.0.1:8001" 
```

4.安装并启动mosquitto

```
sudo yum install epel-release
sudo yum install -y mosquitto
sudo systemctl start mosquitto
sudo systemctl enable mosquitto
mosquitto
```

5.clouid默认参数

```
cloud 

USAGE:
    cloud [OPTIONS]

OPTIONS:
    -g <GRPC>        [default: 0.0.0.0:8001]
    -h, --help       Print help information
    -m <MQTT>        [default: 127.0.0.1:1883]
    -w <WEB>         [default: 0.0.0.0:8000]
```

6.excurtor默认参数

```
controller的GRPC连接域名 默认为127.0.0.1：8001
```

7.运行添加脚本的python代码

```
cd guidang
python3 test.py
```

8.运行设备属性变更的python代码

```
python3 temp.py switch.py
```


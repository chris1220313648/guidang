# 规则引擎

<!--建议使用vscode 插件Markdown Preview Enhanced 和 Markdown All in One -->

## 系统设计文档

### 背景

规则引擎是一个基于Kubernetes的IoT Faas平台.

规则引擎是一个处理设备数据、控制设备的编程运行环境.

规则引擎基于serverless思想, 以**事件触发-> 规则执行**的范式执行用户编写的规则脚本.

规则引擎目前支持的功能见"软件安装和配置文档"的"编写新的脚本"一节.

#### 用户

```plantuml
!theme cyborg
rectangle 规则引擎 {
:脚本作者: as auther
:管理员: as admin
:用户: as user
:auther: --> (script register)
:admin: --> (script deploy)
:user: --> (script trigger)
(发布脚本) as (script register)
(部署脚本) as (script deploy)
(触发脚本) as (script trigger)
(script register) .> (script deploy)
(script deploy) .> (script trigger)
}
```

规则引擎具有3类不同的用户, 脚本作者, 集群管理员和用户.

脚本作者可以访问脚本registry, 并上传可供使用的脚本. 该类用户的管理依靠registry服务器的实现.

集群管理员可以访问Kubernetes API Server, 并部署新的脚本到集群中. 该类用户的管理依靠Kubernetes的用户、用户角色和角色绑定机制.

用户可以手动触发脚本的运行. 该类用户的管理依靠Kubernetes的用户、用户角色和角色绑定机制.

### 架构

正如其他基于Kubernetes开发的云原生平台, 规则引擎分为控制器和执行器两部分, 并使用Kubernetes的自定义资源定义(CRD) 管理部署到Kubernetes集群的规则.

其中控制器Controller具有Kubernetes分配的权限, 可以访问Kubernetes API Server下的资源(具体来说,是Script资源和Device资源), 并提供gRPC服务供控制器访问. 其中Script资源为用户使用规则引擎的主要接口, 其内容由管理员管理, 并被控制器读取, 控制器会修改其Status反应Script的运行状态. Device资源由KubeEdge的设备孪生组件管理, 其内容由管理员管理, 并被规则引擎控制器, KubeEdge的设备孪生组件读取, 规则引擎控制器会修改Device资源的Status以实现对设备的控制, KubeEdge的设备孪生组件会实现由Status的内容操作实际硬件的功能.

执行器没有Kubernetes的权限, 需要访问控制器的gRPC服务, 等待控制器的调度器分配任务, 执行器在接收到任务后会启动运行时执行任务, 并通过gRPC服务完成各个功能.

控制器和执行器之间的协议定义在`proto/controller.proto`中

### 功能逻辑简介

Device资源是KubeEdge提供的资源, 包括了设备的不可变属性(Spec)和状态信息(Status), 其中状态信息包括了设备的可读/可写的属性. 部分设备的状态不可写, 例如传感器类设备, 部分设备的状态可写, 例如开关, 可写的设备一定有可读状态, 反应最后一次成功写入的状态. KubeEdge以及底层的设备驱动会确保对状态的修改会反映到物理世界的IoT设备上.

规则引擎的一个"规则"会预设一个可读的设备列表和可写的设备列表, 并通过代码(如JavaScript或WebAssembly)进行运算, 最终的结果是在某一条件下对可写设备的状态进行修改. 管理员要部署的Script资源会将规则预设的可读/可写设备的名称(字符串)映射到实际Device资源的name上(要求Script和Device资源必须在同一个namespace才能相互访问), 并指定规则代码的名称, 版本号等信息以供规则引擎执行. 除此之外Script资源还可以通过环境变量的方式指定参数.

当一个Script资源所管理的可读设备的Device资源状态发生变化时, Script会被"触发", 由执行器执行规则代码. 一个良好的规则代码应当在多次运行中对同样的输入产生同样的输出. 不过这并非绝对, 因为规则引擎代码允许访问集群上部署的HTTP服务, 例如将可读设备的状态交给深度学习服务进行推理, 并获得返回结果, 再对可写设备的状态进行改变.

除了因设备状态变化产生的触发事件, 用户还可以通过向Web服务器发送Get请求的方式直接触发一个Script资源的执行.

#### 执行器

执行器的结构如下

```plantuml
!theme cyborg
() gRPC -> run
() gRPC -> update
run --> [main] 
[main] --> [worker 1]
[main] --> [worker 2]
[main] --> [worker ...]
update <-- [worker 1]
update <-- [worker 2]
update <-- [worker ...]
[worker 1] --> [Deno]
[worker 1] --> [module loader]
[Deno] --> [v8]
[Deno] --> [ops]
```

##### 主线程

执行器的主线程通过gRPC链接调用run函数. 在建立连接后, 主线程会收到控制器发来的ServerMessage::RunScript消息, 执行器会建立Worker线程处理该消息.

```plantuml
!theme bluegray
Executor --> Controller: TCP connect/HTTP Header (validate Header)
Executor -> Controller: ClientMessage { code = Connect, info = ClientInfo }
Controller -> Executor: ServerMessage::Connected { executor_id }
Executor -> Controller: ClientMessage { code = Continue }
Controller -> Executor: ServerMessage::RunScript { script_id, manifest, ... }
Executor -> Controller: ClientMessage { code = Continue }
== loop... ==
Controller -> Executor: ServerMessage::Disconnect { reason = ServerExit }
== or ==
Executor -> Controller: ClientMessage { code = Disconnect }
Controller -> Executor: ServerMessage::Disconnect { reason = ClientExit }
```

连接的状态变化:

```plantuml
!theme bluegray
start
:Connect;
repeat
:Continue;
repeat while(stop?)
:Disconnect;
stop
```

##### Worker线程

Worker线程进行如下工作:

1. 创建v8运行时
2. 初始化(bootstrap)上下文
3. 加载要运行的规则作为主模块
4. 运行其中的main函数, 直到所有Promise都resolve.
5. 记录运行时间等状态信息, 并通过gRPC的update_script_status调用更新Script资源的Status

##### ops

ops是Deno底层提供的向v8添加native函数的方法, 该模块实现了ECMAScript之外的诸多扩展, 例如日志功能(console.log)或访问设备的Device API功能.

#### 控制器

执行器结构如下

```plantuml
!theme bluegray
() "KubeAPI/Script" as KubeAPI_Script
() "KubeAPI/Device" as KubeAPI_Device
() gRPC
() HTTP
package "Reflector" {
[reflector<Device>] as DeviceReflector
[reflector<Script>] as ScriptReflector
KubeAPI_Device --> [DeviceReflector]
KubeAPI_Script --> [ScriptReflector]
[DeviceReflector] --> [trigger_hook]
[trigger_hook] --> [trigger]
[trigger] --> [Scheduler]
[Reflector] --> [trigger]
[DeviceReflector] --> [device_hook]
[ScriptReflector] --> [script_hook]
[device_hook] --> [Reflector]
[script_hook] --> [Reflector]
}
[Reflector] --> [Scheduler]: RunScriptLookup
[Scheduler] --> [SessionManager]: ManagerMsg { RunScript }
package "WebServer" {
[Reflector] --> [/api/debug]
[/api/webhook] --> [Scheduler]
[/api/debug] --> HTTP
[/api/webhook] <-- HTTP
}
[SessionManager] <--> gRPC: ControllerService
[SessionManager] --> KubeAPI_Script
```

#### Reflector

Reflector是一个使用List&Watch监视Kubernetes API 资源变动的模块, 同时会在内存中维护一份资源的状态, 即Reflector结构体.Reflector结构体还包括了从Device到Script的映射, 以方便的查找触发关系.
目前控制器会List&Watch两个Kubernetes API 资源, 表示部署规则的Script资源和表示部署设备的Device资源.

reflector函数负载执行List&Watch, 并将资源的变动作为事件流输出. 通过向reflector添加同步或异步hook的方式处理事件流.
其中device_hook和script_hook会维护Reflector结构体, trigger_hook会输出所有产生变动的Device资源, trigger会根据Reflector结构体中的Device到Script的映射产生触发事件, 其中包括了要运行的Script的name和namespace.

Scheduler会接收触发事件, 并根据Script的name和namespace读取Script资源, 并读取其他RunScript消息所需要的内容.随后Scheduler将RunScript通过公平调度器发送给一个SessionManager管理的执行器gRPC连接上.

#### SessionManager

SessionManager即gRPC服务器的实现, 负责与执行器建立连接并处理gRPC协议. SessionManager会将从Scheduler接收到的RunScript消息发给执行器.

#### WebServer

WebServer提供了两个api, debug和webhook. debug可以得到Reflector的值, 可以判断控制器的状态是否正确. webhook可以直接触发一个脚本的执行.

### 执行过程

```plantuml
!theme materia
actor Device1
actor Device2
entity KubeAPI
database Reflector
queue Scheduler
entity SessionManager
entity Executor
Executor -> SessionManager: Continue
Device1 --> KubeAPI: sync
KubeAPI --> Reflector: Device1
Reflector --> Scheduler: trigger_hook
Scheduler --> SessionManager: RunScript
SessionManager -> Executor: RunScript
Executor --> Worker: new thread
Worker --> SessionManager: update_device_desired
SessionManager --> KubeAPI: Patch Device Status
KubeAPI --> Device2: sync
Worker --> SessionManager: update_script_status
SessionManager --> KubeAPI: Patch Script Status
Executor -> SessionManager: Continue
== loop... ==
Executor -> SessionManager: Disconnect
SessionManager -> Executor: Disconnect
```

## 软件安装和配置文档

### 先决条件

规则引擎基于Kubernetes和KubeEdge提供的环境运行, 因此需要先安装Kubernetes和KubeEdge.

#### Kubernetes

请参照Kubernetes官方文档. 建议安装`1.22`版本的Kubernetes

* [安装`kubeadm`](https://kubernetes.io/docs/setup/production-environment/tools/kubeadm/install-kubeadm/)
  * 该过程需要访问`google.com`和`kubernetes.io`, 请确保各节点的网络可以正常访问该域名
* [创建集群](https://kubernetes.io/docs/setup/production-environment/tools/kubeadm/create-cluster-kubeadm/)
  * 该过程需要访问`google.com`和`k8s.gcr.io`, 请确保各节点的网络可以正常访问该域名

请在开发机上安装kubectl, 并按照创建集群的输出配置`.kube/config`配置文件.

#### KubeEdge

请参照KubeEdge官方文档.

##### 云节点

请在Kubernetes的master节点执行如下操作

* 从GitHub下载最新版的[keadm](https://github.com/kubeedge/kubeedge/releases),推荐下载1.12版本
* 解压keadm并移动到`/usr/local/bin`
* [安装云端组件cloudcore](https://kubeedge.io/en/docs/setup/keadm/#setup-cloud-side-kubeedge-master-node)
  * 简言之, 执行`sudo keadm init`
  * 该软件需要访问`github.com`和`raw.githubusercontent.com`, 请确保网络可以正常访问该域名

##### 边缘节点

请在边缘节点上执行如下操作. 边缘节点必须**没有**加入Kubernetes集群.

* 从GitHub下载最新版的[keadm](https://github.com/kubeedge/kubeedge/releases)，推荐下载1.12版本
* 解压keadm并移动到`/usr/local/bin`
* [安装边缘节点组件edgecore](https://kubeedge.io/en/docs/setup/keadm/#setup-edge-side-kubeedge-worker-node)

#### Rust

本软件采用Rust编程语言编写, 请在开发计算机上安装Rust编译器和C/C++编译器.

Rust的安装方式请参考[Rust程序语言官网文档](https://www.rust-lang.org/learn/get-started). C/C++编译器及其工具的安装参考各系统的官方文档.

#### OCI容器构建工具

请安装一个可以构建OCI容器的工具, 可以选择如下组合

* docker
* containerd/nerdctl/buildkit
* podman/buildah

### 测试

#### 生成Script CRD

运行`/utils/script-crd`下的项目, 可以得到Script CRD的输出.

```shell
cd /utils/script-crd
cargo run --release > ../../config/script_crd.yaml
```

如果没有对proto下的协议做出变动,则不需要执行此操作.

#### 编译项目

虽然kubernetes只能在Linux上使用, 但本项目不限制开发阶段能运行的操作系统. 在开发机上项目根目录运行该命令即可编译所有组件.

```shell
cargo build --release
```

二进制位于`target/release`下.

#### 运行

按如下顺序启动各个服务

```shell
cloud
deno_executor
filter-server
```

执行如下命令, 如果有输出, 代表控制器正常启动

```shell
curl -v http://127.0.0.1:8000/api/v1alpha/debug
```

### 配置

#### controller

云端控制器支持如下选项.

```txt
cloud 

USAGE:
    cloud [OPTIONS]

OPTIONS:
    -g <GRPC>        [default: 0.0.0.0:8001]
    -h, --help       Print help information
    -m <MQTT>        [default: 127.0.0.1:1883]
    -w <WEB>         [default: 0.0.0.0:8000]
```

其中

* GRPC为executor的连接端口
* MQTT为MQT Broker的ip/端口号
* WEB为控制器的webhook和调试api的连接端口

在集群外部运行时, 控制器会自动读取`~/.kube/config`下的凭据和配置访问Kubernetes集群.

#### executor

执行器只支持一个参数, 即controller的GRPC连接域名.

### 发布

#### 编译controller

构建`controller/cloud`下的Dockerfile, 并将该镜像发布到私有register

#### 编译executor

构建`executor/deno`下的Dockerfile, 并将该镜像发布到私有register

### 部署

部署需要应用如下yaml, 在应用前检查内容并做出必要的调整, 如URL, 镜像地址和ip地址

```list
config/controller_account.yaml
config/script_crd.yaml
controller/cloud/deployment-cloud.yaml
controller/cloud/service-cloud.yaml
executor/deno/deployment-deno.yaml
```

可以在`controller/cloud/deployment-cloud.yaml`的`spec.template.spec.containers[0].args`指定控制器的参数, 修改端口号后, 请一并修改`controller/cloud/service-cloud.yaml`中的端口号映射.

可以在`executor/deno/deployment-deno.yaml`的`spec.template.spec.containers[0].args`指定执行器要连接的控制权域名.

### (可选)运行示例脚本

请额外构建`utils/filter-server`下的Dockerfile, 并将该镜像发布到私有register

请部署http服务器, 将根目录设置为`config/register`, 随后根据`sercice-filter`的集群DNS设置修改`config/test_script.yaml`中的`spec.env.filter_service_url`的域名为集群dns给定的地址, 将`spec.manifest.register`的域名设置为http服务器的域名.

请额外应用如下yaml

```list
utils/mapper/switch.model.yaml
utils/mapper/switch.yaml
utils/mapper/temp.model.yaml
utils/mapper/temp.yaml
utils/filter-server/deployment-filter.yaml
utils/filter-server/sercice-filter.yaml
config/test_script.yaml
```

运行如下命令执行示例脚本

```shell
curl -v "http://127.0.0.1:8000/api/v1alpha/webhook?namespace=default&name=test-script"
```

查看deno_executor的输出, 可以看到脚本的输出.

运行如下命令查看脚本的运行结果, Status.Message的状态应该为空, Status.Status的状态应该为0

```shell
kubectl describe scripts.hit.edu.cn test-script
```

### 编写新的脚本

脚本的编写分为两步: 1. 使用编程语言编写脚本 2. 编写Script资源的定义文件并应用到集群

#### 脚本部分

目前规则引擎只支持ECMAScript语言. WebAssembly的支持正在开发中, 暂无支持其他编程语言的计划.

脚本支持标准的ECMAScript语言, 并带有部分Web API的支持, 由于Web API的支持尚在开发, 因此并没有对这部分API是否符合规范进行检查.

除了ECMAScript的标准全局变量之外, 规则引擎还具有额外两个只读全局变量, `Deno`和`Device`.

Device全局变量具有如下API

```js
/// 返回可读设备名称的Array
function listReadableDevices()
/// 返回可写设备名称的Array
function listWritableDevices()
/// 获得device设备的property属性值
function getDeviceStatus(device, property)
/// 设置device设备的property属性值
/// 对属性值的修改只有提交后才会生效
function setDeviceStatus(device, property, value)
/// 提交对属性值的修改
async function commitDevice(device, qos)
```

Deno全局变量下的功能均为内部实现或临时功能, 不应视为公开功能.
但规则引擎的确有计划部分兼容Deno的标准库, 只是该功能正在开发.

脚本的入口为`main()`函数, main函数的返回值会被丢弃.

#### script资源定义

Script资源为yaml文件, 其示例如下:

```yaml
apiVersion: hit.edu.cn/v1alpha1
kind: Script
metadata:
  name: test-script
  namespace: default
spec:
  readSelector:
    matchNames:
      temp-sensor-name: dht11
  writeSelector:
    matchNames:
      target-device-name: switch
  env:
    filter_service_url: "http://127.0.0.1:8003/api/v1alpha1/filter"
    threshold-value: "400"
  manifest:
    scriptType: Js
    name: test
    version: 0.1_beta1
    register: http://127.0.0.1:8080
  executePolicy:
    readChange: true
    webhook: true
    cron: ""
    qos: AtMostOnce
```

精确的格式要求见crd定义的OpenAPI v3 Schema

##### readSeledtor/writeSelector

目前仅支持`matchNames`, 其内容为键值对, 键为脚本中使用的设备名称, 值为实际设备的资源名称.

在脚本中使用的Device API所用到的设备名称均为键, 操作对象则为值对应的Device资源.

##### env

env为键值对, 可以在脚本中使用`Deno.env[key]`来访问对应的value.

#### manifest

目前scriptType仅支持Js, 即ECMAScript. 对Wasm, 即WebAssembly的支持正在开发.

name, version和register指定了资源的地址, 最终访问的URL为`${register}/${name}/${version}.${ext}`. 其中ext目前仅支持js, 未来会支持wasm, zip和gz

#### executePolicy

executePolicy的各项功能均未实现, 请保持原样.

原定的字段功能为: readChange表示是否启用基于设备状态变动的触发, webhook表示是否启用基于webhook的触发, cron表示启用基于cron的定时触发, qos可选值为AtMostOnce, AtLeastOnce, OnlyOnce, 分别对应同名的MQTT Qos等级.

基于webhook的触发目前总是启用, 其URL为`http://<host>/api/v1alpha1/webhook?namespace=default&name=script`, namespace和name请求参数指定要触发的Script的namespace和name, 需要使用HTTP Get请求.

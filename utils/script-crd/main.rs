use controller::api::script_sqlite3::*;
use kube::CustomResourceExt;

fn main() {
    // let crd = Script::crd();// 调用Script类型（你的自定义资源）上的crd类方法，该方法由CustomResourceExt trait提供。这个方法生成该自定义资源的CRD描述，其返回的是一个kube::api::CustomResourceDefinition结构实例。
    println!("{}", "hellow cjy");
}
//使用serde_yaml::to_string将CRD结构序列化为YAML格式的字符串。serde_yaml是Rust的一个序列化库，可以处理YAML格式，这里用于生成可以被kubectl apply -f等Kubernetes工具使用的YAML字符串。使用unwrap()来处理可能的序列化错误，虽然在实际应用中最好使用错误处理来代替。
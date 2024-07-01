use controller::api::script::*;
use kube::CustomResourceExt;

fn main() {
    let crd = Script::crd();
    println!("{}", serde_yaml::to_string(&crd).unwrap());
}

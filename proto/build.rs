fn main() {
    tonic_build::compile_protos("./controller.proto")
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}//并编译这些.proto文件为Rust代码，这些代码随后可以被项目直接使用

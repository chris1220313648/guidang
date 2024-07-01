#![allow(non_camel_case_types)]
//通过tonic::include_proto!宏，生成的Rust代码被包含进当前源文件。这使得protobuf定义的RPC服务、消息类型等可以在Rust项目中直接使用。
tonic::include_proto!("rule");
//tonic::include_proto!("rule"); 告诉tonic库在编译时查找并包含一个名为rule的protobuf包生成的Rust代码。这里的"rule"应该对应于一个.proto文件的路径（不包含.proto扩展名），该文件在项目的build.rs构建脚本中通过tonic_build处理过。处理结果通常放在target目录下的某个子目录中，且被Cargo自动识别为编译依赖。
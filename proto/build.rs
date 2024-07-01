fn main() {
    tonic_build::compile_protos("./controller.proto")
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}

use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use executor_ops as ops;
use std::env;
use std::path::PathBuf;
//通过这段代码，你可以在 Deno 环境中预编译 JavaScript 代码，生成快照文件
fn main() {
    let snapshot_path = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("SNAPSHOT.bin");//使用PathBuf和env::var_os("OUT_DIR")来构建快照文件的保存路径。
    let mut rt = JsRuntime::new(RuntimeOptions {//用于执行JavaScript代码的运行时环境
        module_loader: None,//module_loader: None指定没有模块加载器。这意味着此实例不能加载ES模块，除非在扩展中提供加载逻辑。
        extensions: ops::extensions(),//加载自定义扩展
        startup_snapshot: None,
        will_snapshot: true,//生成快照
        ..Default::default()
    });
    let snapshot = rt.snapshot();//从运行时生成一个快照。
    let snapshot_slice: &[u8] = &*snapshot;//快照转换成字节切片
    println!("Snapshot size: {}", snapshot_slice.len());
    std::fs::write(&snapshot_path, snapshot_slice).unwrap();//使用std::fs::write(&snapshot_path, snapshot_slice).unwrap();将快照数据写入之前计算的路径
    println!("Snapshot written to: {} ", snapshot_path.display());
}

use deno_core::{include_js_files, Extension};
//这段Rust代码演示了如何在Deno环境中创建一个Deno扩展，该扩展通过包含JavaScript文件来扩展Deno的功能
pub fn init() -> Extension {
    Extension::builder()//这个方法创建了一个新的扩展构建器实例，这是配置和构建Deno扩展的起点。
        .js(include_js_files!(// 使用include_js_files!宏来静态地包含JavaScript文件。这些文件在Deno运行时启动时会被加载，从而扩展或修改Deno的行为
            prefix "executor:bootstrap",//这个前缀将用于生成这些脚本资源的标识符
            "bootstrap/06_util.js",
            "bootstrap/99_main.js",
        ))
        .build()//调用这个方法来完成扩展的配置，并生成Extension对象。这个对象随后可以被注册到Deno运行时中，作为运行时的一部分。
}

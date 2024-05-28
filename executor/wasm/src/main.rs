use wasmtime::*;
//wasmtime是一个用于WebAssembly的运行时，支持在Rust中执行Wasm模块。
struct State;

fn main() {
    let path = std::env::args().nth(1).unwrap();//命令行获取文件路劲
    let calc = std::fs::read(path).unwrap();//读取wasm文件内容
    let engine = Engine::default();//创建实例
    let module = Module::new(&engine, calc).unwrap();//根据引擎和Wasm文件内容创建模块
    let mut store = Store::new(&engine, State);//存储运行时数据，State作为其泛型参数指定了存储的状态类型。
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let fib = instance//获取Wasm模块中名为fib和same的函数。这些函数接收一个i32类型的参数并返回一个i32类型的结果。
        .get_typed_func::<i32, i32, _>(&mut store, "fib")
        .unwrap();
    let same = instance
        .get_typed_func::<i32, i32, _>(&mut store, "same")
        .unwrap();
    let table = instance
        .get_table(&mut store, "__indirect_function_table")
        .unwrap();
    let n = fib.call(&mut store, 30).unwrap();
    println!("fib(30)={n}");
    table
        .set(&mut store, 1, Val::FuncRef(Some(*same.func())))
        .unwrap();//使用table.set修改间接函数表中的条目，将第二个函数引用（same）替换成fib函数，然后再次调用fib函数，并打印结果。这展示了如何通过间接函数表修改Wasm模块的行为
    let n = fib.call(&mut store, 30).unwrap();
    println!("same hacked fib(30)={n}");
    let native_fib = Func::wrap(&mut store, |mut caller: Caller<'_, State>, n: i32| -> i32 {//。Func::wrap方法用于从一个Rust闭包创建一个Func实例，使其可以从WebAssembly中被调用。
        if n <= 1 {
            return 1;
        }//caller 提供函数调用的上下文，包括对调用实例及其状态的访问
        let fib = caller.get_export("fib").unwrap().into_func().unwrap();
        let fib = fib.typed::<i32, i32, _>(&mut caller).unwrap();
        let a = fib.call(&mut caller, n - 1).unwrap();
        let b = fib.call(&mut caller, n - 2).unwrap();
        a + b
    });
    table
        .set(&mut store, 1, Val::FuncRef(Some(native_fib)))
        .unwrap();//将函数表索引1处的条目替换为native_fib函数。这意味着，Wasm模块中对此表索引的任何间接调用现在都将调用Rust实现的斐波那契函数
    let n = fib.call(&mut store, 30).unwrap();
    println!("natived fib(30)={n}");
}

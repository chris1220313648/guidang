use wasmtime::*;

struct State;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let calc = std::fs::read(path).unwrap();
    let engine = Engine::default();
    let module = Module::new(&engine, calc).unwrap();
    let mut store = Store::new(&engine, State);
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let fib = instance
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
        .unwrap();
    let n = fib.call(&mut store, 30).unwrap();
    println!("same hacked fib(30)={n}");
    let native_fib = Func::wrap(&mut store, |mut caller: Caller<'_, State>, n: i32| -> i32 {
        if n <= 1 {
            return 1;
        }
        let fib = caller.get_export("fib").unwrap().into_func().unwrap();
        let fib = fib.typed::<i32, i32, _>(&mut caller).unwrap();
        let a = fib.call(&mut caller, n - 1).unwrap();
        let b = fib.call(&mut caller, n - 2).unwrap();
        a + b
    });
    table
        .set(&mut store, 1, Val::FuncRef(Some(native_fib)))
        .unwrap();
    let n = fib.call(&mut store, 30).unwrap();
    println!("natived fib(30)={n}");
}

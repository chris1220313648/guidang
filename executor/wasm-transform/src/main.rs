use rayon::iter::ParallelIterator;
use std::{collections::HashMap, path::PathBuf};
//这段代码展示了如何使用walrus，一个用于WebAssembly模块处理的Rust库，来修改WebAssembly（Wasm）模块
use walrus::{
    ir::{dfs_pre_order_mut, Call, Const, Instr, InstrSeq, InstrSeqId, Value, VisitorMut},
    ElementKind, FunctionId, ImportKind, InitExpr, InstrLocId, LocalFunction, ModuleConfig,
    TableId, TypeId,
};

const WRAPPER: &str = "__wrapper__";

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut path = PathBuf::from(path);
    let config = ModuleConfig::new();
    let wasm = std::fs::read(&path).unwrap();
    let mut module = config.parse(&wasm).unwrap();//分别用于存储将要替换的函数信息和将要删除的导入项。
    let mut items = Vec::new();
    let mut to_delete = Vec::new();
    for i in module.imports.iter() {//遍历模块的所有导入项，对于每个导入的函数（匹配ImportKind::Function），如果它属于指定的模块（WRAPPER常量），则记录其ID和名称到to_delete，
        if let ImportKind::Function(funcid) = i.kind {
            if i.module == WRAPPER {//历模块中的所有导入项。对于每一个导入的函数（ImportKind::Function），检查是否属于特定的模块（由WRAPPER常量指定）。
                to_delete.push((i.id(), i.name.clone()));
                let tyid = module.funcs.get(funcid).ty();
                items.push(Item { funcid, tyid })
            }
        }
    }
    let table_id = module.tables.main_function_table().unwrap().unwrap();
    let table = module.tables.get_mut(table_id);
    let offset = table.initial as i32;
    table.initial += to_delete.len() as u32;
    if let Some(max) = table.maximum {
        table.maximum = Some(max + to_delete.len() as u32);
    } else {
        table.maximum = Some(to_delete.len() as u32);
    }
    module.funcs.par_iter_local_mut().for_each(|(_, func)| {
        let start = func.entry_block();
        let mut replacer = Replacer::new(&items, start, offset);
        dfs_pre_order_mut(&mut replacer, func, start);
        replacer.insert_call(func, table_id);
    });//获取主函数表的ID，并根据to_delete的长度扩展函数表的初始和最大大小，为后续的间接调用预留空间。
    let element_kind = ElementKind::Active {
        table: table_id,
        offset: InitExpr::Value(Value::I32(offset)),
    };
    let mut elems = Vec::new();
    for (id, name) in to_delete {
        let real_name = std::str::from_utf8(&name.as_bytes()[WRAPPER.as_bytes().len()..]).unwrap();
        println!("replace call {name} to call_indirect {real_name}");
        if let Some(id) = module.funcs.by_name(real_name) {
            elems.push(Some(id))
        } else {
            panic!("Can't find function `{real_name}` in wasm module");
            //elems.push(None);
        }
        module.imports.delete(id)
    }
    module
        .elements
        .add(element_kind, walrus::ValType::Funcref, elems);
    walrus::passes::gc::run(&mut module);
    path.set_extension("new.wasm");
    module.emit_wasm_file(path).unwrap();
}

struct Item {
    funcid: FunctionId,
    tyid: TypeId,
}

struct Replacer<'a> {
    items: &'a [Item],
    current_instr_seq: InstrSeqId,
    current_replaced: Vec<(InstrLocId, TypeId)>,
    replaced: HashMap<InstrSeqId, Vec<(usize, TypeId)>>,
    offset: i32,
}

impl<'a> VisitorMut for Replacer<'a> {
    fn start_instr_seq_mut(&mut self, instr_seq: &mut InstrSeq) {
        self.current_instr_seq = instr_seq.id()
    }
    fn end_instr_seq_mut(&mut self, instr_seq: &mut InstrSeq) {
        assert_eq!(self.current_instr_seq, instr_seq.id());
        let mut new = Vec::new();
        for item in &self.current_replaced {
            for (i, (_, locid)) in instr_seq.instrs.iter().enumerate() {
                if item.0.data() == locid.data() {
                    new.push((i + 1, item.1))
                }
            }
        }
        self.current_replaced.clear();
        self.replaced.insert(instr_seq.id(), new);
    }
    fn visit_instr_mut(&mut self, instr: &mut Instr, loc: &mut InstrLocId) {
        if let &mut Instr::Call(Call { func }) = instr {
            for (i, item) in self.items.iter().enumerate() {
                if item.funcid == func {
                    *instr = Instr::Const(Const {
                        value: Value::I32(i as i32 + self.offset),
                    });
                    self.current_replaced.push((*loc, item.tyid));
                }
            }
        }
    }
}

impl<'a> Replacer<'a> {
    fn new(items: &'a [Item], id: InstrSeqId, offset: i32) -> Self {
        Replacer {
            items,
            current_instr_seq: id,
            current_replaced: Vec::new(),
            replaced: HashMap::new(),
            offset,
        }
    }
    fn insert_call(self, func: &mut LocalFunction, table: TableId) {
        let builder = func.builder_mut();
        for (id, rep) in self.replaced {
            let mut builder = builder.instr_seq(id);
            for (i, (pos, idx)) in rep.into_iter().enumerate() {
                builder.call_indirect_at(pos + i, idx, table);
            }
        }
    }
}

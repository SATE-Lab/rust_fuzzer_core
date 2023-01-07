//use rustc_data_structures::fx::FxHashSet;
//use rustc_hir::def_id::CrateNum;
use rustc_span::Symbol;

use crate::fuzz_targets_gen::function;

#[allow(dead_code)]
#[derive(Clone)]
// 对于单个crate，内部api
pub(crate) struct CrateGraph {
    pub(crate) krate: Symbol,
    pub(crate) functions: Vec<function::Function>,
}

#[allow(dead_code)]
// 生态系统内部的图集合
struct BigGraph {
    crate_graph_set: Vec<CrateGraph>,
}

impl CrateGraph {
    pub(crate) fn add_function(func: function::Function) {
        self.functions.push(func);
    }
}

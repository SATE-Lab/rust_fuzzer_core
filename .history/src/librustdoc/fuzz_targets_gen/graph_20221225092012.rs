use rustc_data_structures::fx::FxHashSet;
use rustc_hir::def_id::CrateNum;

use crate::fuzz_targets_gen::function;

#[allow(dead_code)]
// 对于单个crate，内部api
pub(crate) struct CrateGraph {
    pub(crate) krate: CrateNum,
    functions: Vec<function::Function>,
}

#[allow(dead_code)]
// 生态系统内部的图集合
struct BigGraph {
    crate_graph_set: FxHashSet<CrateGraph>,
}

use rustc_data_structures::fx::{FxHashMap,
use rustc_hir::def_id::CrateNum;

// 对于单个crate，内部api
struct CrateGraph {
    krate: CrateNum,
}

// 生态系统内部的图集合
struct BigGraph {
    crate_graph_set: FxHashSet<CrateGraph>,
}

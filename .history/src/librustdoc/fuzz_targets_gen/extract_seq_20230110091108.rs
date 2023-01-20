use crate::fuzz_targets_gen::extract_dep::AllDependencies;
use crate::fuzz_targets_gen::util::{Node, Stack};

/// 进行一个深度优先搜索，然后生成遍历序列
/// 获得函数签名之后，就获得了生成序列的源信息
pub fn _extract_sequence<'tcx>(tcx: TyCtxt<'tcx>, all_dependencies: AllDependencies<'tcx>) {
    for _caller in all_dependencies.functions {
        //FIXME:
        let mir = tcx.mir_built(ty::WithOptConstParam {
            did: function,
            const_param_did: tcx.opt_const_param_of(function),
        });
        let mir = mir.borrow();
        let mir: &mir::Body<'_> = &mir;
    }
}

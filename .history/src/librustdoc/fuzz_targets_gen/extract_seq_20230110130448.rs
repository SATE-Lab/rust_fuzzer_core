use crate::fuzz_targets_gen::extract_dep::AllDependencies;
use crate::fuzz_targets_gen::util::{Node, Stack};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::mir;
use rustc_middle::ty::TyCtxt;

use super::extract_dep::{CalleeDependency, extract_arguments, Argument};

/// 进行一个深度优先搜索，然后生成遍历序列
/// 获得函数签名之后，就获得了生成序列的源信息
pub fn _extract_sequence<'tcx>(tcx: TyCtxt<'tcx>, all_dependencies: AllDependencies<'tcx>) {
    // 装入所有解析的序列
    let all_seq = Vec::new();

    for (caller, function) in all_dependencies.functions {
        //FIXME:

        let mut func_seq = Vec::new();

        if let Some(caller_local) = caller.as_local() {


            // 测试每一个参数，如果有任何一个不是primitive类型的，都会成功
            let args = function.arguments;
            if args.iter().any(|arg| !arg.ty.is_primitive_ty()) {
                continue;
            }

            let callee_dependency = function.callee_dependencies;
            for CalleeDependency { arg_sources, callee } in callee_dependency {
                use super::extract_dep::Callee;
                let crate_name = match callee {
                    Callee::DirectCall(def_id) => tcx.crate_name(caller.krate).as_str(),
                    Callee::LocalFunctionPtr(_) => continue, //跳过
                };

                // FIXME: 如果是test crate的api，推入序列
                if crate_name == "url" {
                    if callee.
                    
                    func_seq.push(tcx.def_path_str(caller));
                }

            }
        }
    }
}



#[allow(dead_code)]
struct FunctionInfo {
    id: DefId,
    return_ty: Ty,
    arguments: Vec<Argument>,
}


fn get_function_info<'tcx>(tcx: TyCtxt<'tcx>, function: LocalDefId){
    let mir = tcx.mir_built(ty::WithOptConstParam {
        did: function,
        const_param_did: tcx.opt_const_param_of(function),
    });
    let mir = mir.borrow();
    let _mir: &mir::Body<'_> = &mir;
    
    // caller
    let id = function.to_def_id();
    // 返回值
    let return_ty = mir.local_decls[mir::Local::from_usize(0)].ty;
    // 参数
    let arguments = extract_arguments(&mir);
    

}
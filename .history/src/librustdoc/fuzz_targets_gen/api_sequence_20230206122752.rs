use crate::fuzz_targets_gen::call_type::CallType;
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::def;
use rustc_hir::def_id::LocalDefId;
use rustc_middle::mir;
use rustc_middle::ty::{self, Ty, TyCtxt};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Api<'tcx> {
    pub full_name: String,
    pub local_def_id: LocalDefId,
    pub params: Vec<Param<'tcx>>,
    pub is_unsafe: bool,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Param<'tcx> {
    pub ty: Ty<'tcx>,                          //参数类型
    pub returned_by_full_name: Option<String>, //被哪个参数返回.如果Some(_)，就是_local，否则就是
    pub index: usize,                          //在参数列表里的位置
}

pub struct RetVal<'tcx> {
    pub ty: Ty<'tcx>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiSequence<'tcx> {
    pub crate_name: String, //crate名字
    pub sequence: Vec<Api<'tcx>>, //函数序列
                            //pub full_name_map: FullNameMap,
}

impl Param<'_> {
    pub fn get_param_string(&self, param_prefix: &str, local_param_prefix: &str) -> String {
        match self.returned_by_full_name {
            Some(_) => {
                // 当param是某个函数的返回值的话
                // 参数就是这样：_local0 _local1
                let mut s1 = local_param_prefix.to_string();
                s1 += &(self.index.to_string());
                s1
            }
            None => {
                // 当参数不是函数返回值，而是可以被随机生成的变量的话
                // 参数就是 _param0 _param1
                let mut s1 = param_prefix.to_string();
                s1 += &(self.index.to_string());
                s1
            }
        }
    }
}

impl ApiSequence<'_> {
    pub fn new_from_function_full_names(
        crate_name: String,
        full_names: Vec<String>,
        tcx: TyCtxt<'_>,
    ) -> Self {
        for full_name in full_names {
            //找到对应的函数
            let local_def_id = tcx.hir().body_owners().find(|x| match tcx.def_kind(*x) {
                def::DefKind::Fn
                | def::DefKind::AssocFn
                | def::DefKind::Closure
                | def::DefKind::Generator => {
                    let name = crate_name + "::" + &tcx.def_path_str(x.to_def_id());
                    name == full_name
                }
                _ => false,
            });

            let function = match local_def_id {
                Some(id) => id,
                None => {
                    panic!("Didn't find function {}", full_name);
                }
            };

            //到这里，已经找到了名称对应的LocalDefId
        }
        ApiSequence { crate_name: "123".to_string(), sequence: _ }
    }

    /// 通过local_def_id来解析函数定义
    fn extract_function(tcx: TyCtxt<'_>, local_def_id: LocalDefId, full_name: String) -> Api<'_> {
        // 获取hir::Node
        let hir = tcx.hir().find_by_def_id(local_def_id).unwrap();
        let fn_sig = hir.fn_sig().unwrap();
        let header = &fn_sig.header;
        let decl = fn_sig.decl;

        let is_unsafe = header.is_unsafe();

        /*******************/

        // 获取mir::Body
        let mir = tcx.mir_built(ty::WithOptConstParam {
            did: local_def_id,
            const_param_did: tcx.opt_const_param_of(local_def_id),
        });
        let mir = mir.borrow();
        let mir: &mir::Body<'_> = &mir;

        // 返回值
        let return_ty = mir.local_decls[mir::Local::from_usize(0)].ty;

        // 参数
        let params = Self::extract_input(mir);

        Api { full_name, local_def_id, params, is_unsafe }
    }

    fn extract_input<'tcx>(function: &mir::Body<'tcx>) -> Vec<Param<'tcx>> {
        function
            .args_iter()
            .map(|arg_local| {
                /*
                let symbol = function
                    .var_debug_info
                    .iter()
                    .find(|debug| {
                        use mir::VarDebugInfoContents::*;
                        match &debug.value {
                            Place(place) => place.local == arg_local,
                            Const(_) => false, // FIXME: should I track constant?
                            _other => false,
                        }
                    })
                    .map(|debug| debug.name);
                */
                let ty = function.local_decls[arg_local].ty;

                // local
                Param { ty, returned_by_full_name: None, index: 0 }
            })
            .collect()
    }

    /*
    fn extract_output<'tcx>() -> RetVal<'tcx> {
        _
    }*/

    pub fn have_no_success(&self, index: usize) -> bool {
        let set = FxHashSet::default();

        for api in &self.sequence {
            for param in api.params {
                if let Some(fore_full_name) = param.returned_by_full_name {
                    set.insert(fore_full_name);
                }
            }
        }

        let api_name = &self.sequence[index].full_name;

        !set.contains(api_name)
    }
}

//use crate::fuzz_targets_gen::call_type::CallType;
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
    pub ty: rustc_hir::Ty<'tcx>,               //参数类型
    pub returned_by_full_name: Option<String>, //被哪个参数返回.如果Some(_)，就是_local，否则就是
    pub index: usize,                          //在参数列表里的位置
}

pub struct RetVal<'tcx> {
    pub ty: Ty<'tcx>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiSequence<'tcx> {
    pub crate_name: String,       //crate名字
    pub sequence: Vec<Api<'tcx>>, //函数序列
    pub full_name_map: FullNameMap,
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
    pub fn new() -> Self {
        ApiSequence { crate_name: _, sequence: _, full_name_map: _ }
    }

    pub fn construct_from_function_full_names(
        &mut self,
        crate_name: String,
        full_names: Vec<String>,
        tcx: TyCtxt<'_>,
    ) {
        let sequence = Vec::new();
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

            let local_def_id = match local_def_id {
                Some(id) => id,
                None => {
                    panic!("Didn't find function {}", full_name);
                }
            };

            sequence.push(api);
        }
        self.crate_name = "123".to_owned();

        self.sequence = sequence;
    }
}

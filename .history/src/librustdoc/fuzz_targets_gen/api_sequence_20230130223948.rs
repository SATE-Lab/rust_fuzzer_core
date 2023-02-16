use rustc_hir::def_id::DefId;
use rustc_middle::ty::{Ty, TyCtxt};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Api<'tcx> {
    pub full_name: String,
    pub def_id: DefId,
    pub params: Vec<Param<'tcx>>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Param<'tcx> {
    pub ty: Ty<'tcx>,                          //参数类型
    pub returned_by_full_name: Option<String>, //被哪个参数返回
    pub index: usize,                          //在参数列表里的位置
}

#[allow(dead_code)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiSequence<'a> {
    pub kcrate_name: String,    //crate名字
    pub sequence: Vec<Api<'a>>, //函数序列
}

impl ApiSequence<'_> {
    pub fn new_from_function_full_names(full_names: Vec<String>, tcx: TyCtxt<'tcx>) -> Self {}
}

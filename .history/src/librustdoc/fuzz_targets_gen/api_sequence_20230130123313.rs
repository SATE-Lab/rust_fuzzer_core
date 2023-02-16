use rustc_hir::def_id::DefId;
use rustc_middle::ty::Ty;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Api<'tcx> {
    pub full_name: String,
    pub def_id: DefId,
    pub params: Vec<Ty<'tcx>>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Param


#[allow(dead_code)]
pub struct ApiSequence {
    pub kcrate_name: String,
    pub function_sequence: Vec<String>,
}

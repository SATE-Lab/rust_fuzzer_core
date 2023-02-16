use rustc_hir::def_id::DefId;
use rustc_middle::ty::Ty;

pub struct Api {
    pub full_name: String,
    pub def_id: DefId,
    pub params: Vec<Ty<'_>>,
}

#[allow(dead_code)]
pub struct ApiSequence {
    pub kcrate_name: String,
    pub function_sequence: Vec<String>,
}

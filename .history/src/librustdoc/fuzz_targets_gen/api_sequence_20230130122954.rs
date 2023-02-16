use rustc_hir::def_id::DefId;

pub struct Api {
    pub full_name: String,
    pub def_id: DefId,
}

#[allow(dead_code)]
pub struct ApiSequence {
    pub kcrate_name: String,
    pub function_sequence: Vec<String>,
}

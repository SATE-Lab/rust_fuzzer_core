use crate::fuzz_targets_gen::extract_dep::AllDependencies;

pub fn _extract_sequence<'tcx>(all_dependencies: AllDependencies<'tcx>) {
    for caller in all_dependencies.functions {
        //FIXME:
    }
}

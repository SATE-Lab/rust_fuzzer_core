//use crate::config::RenderInfo;
//use crate::core::{init_lints, EmitIgnoredResolutionErrors};

/// Describes the dependencies of every variables
///
/// Let start with an example:
///
/// ```no_run
/// fn foo(arg1: i32, arg2: i32) -> i32 {
///     let local = arg;
///     println!("{}", arg2);
///     local * 18;
/// }
/// ```
///
/// In this function, the return value has a direct dependency to `local`, since its value is
/// computed from `local`. It also has a direct dependency to the constant `18`. `local` itself has
/// a direct dependency to `arg1`. Therefore the return value has an indirect dependency to `arg1`,
/// and the constant `18` but no dependency to `arg2`.
///
/// ---
///
/// If `dependency[index][index] == true`, then `index` has a dependency to `index`.
///
/// `dependencies` is a 2D array with `dependencies.len() + constants.len()` columns.
///
/// The column 0 is the return value. Then you have the arguments (from index `1` to `arg_count +
/// 1`), then all other locals up to `dependencies.len()`. The last `constants.len()` indexes of
/// the `BitVec` are indexes into `constants`.
///
/// For example, if `dependency[index][18] == true` and `dependencies.len() == 15`, this means that
/// `index` has a dependency to the 3rd (`18 - 15`) constant. To access it conveniently, you can
/// use `self.constant(index)` to get the constant associated with `index`.
#[derive(Debug, Clone)]
pub struct Dependencies<'tcx> {
    dependencies: IndexVec<mir::Local, BitVec>,
    constants: Vec<mir::Constant<'tcx>>,
    arg_count: usize,
}

#[derive(Debug, Clone)]
pub enum DependencyType<'tcx> {
    Return,
    Argument(mir::Local),
    Local(mir::Local),
    Constant(Constant<'tcx>),
}

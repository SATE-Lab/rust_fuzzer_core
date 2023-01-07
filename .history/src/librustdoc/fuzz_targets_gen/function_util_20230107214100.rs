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

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Argument<'tcx> {
    //在函数体里面的编号
    arg_local: mir::Local,
    //参数名字
    symbol: Option<Symbol>,
    //参数类型
    ty: Ty<'tcx>,
}

/// Dependencies of a given local
///
/// Note: we could also track special constants, other than functions
#[derive(Debug, Clone)]
pub enum Source<'tcx> {
    /// a reference to another function
    FunctionId(Ty<'tcx>),

    /// argument of the caller
    Argument(mir::Local),

    /// return variable of another callee
    ReturnVariable(DefId),
}

#[derive(Debug, Clone)]
pub enum Callee<'tcx> {
    DirectCall(DefId),
    LocalFunctionPtr(Vec<Source<'tcx>>),
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CallerDependency<'tcx> {
    /// k是形式参数编号，从0开始
    /// v是依赖集合
    arg_sources: FxHashMap<usize, Vec<Source<'tcx>>>,
    /// function being called
    callee: Callee<'tcx>,
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Function<'tcx> {
    return_ty: Ty<'tcx>,
    arguments: Vec<Argument<'tcx>>,
    dependencies: Vec<CallerDependency<'tcx>>,
    return_deps: Vec<Source<'tcx>>,
}

#[derive(Debug, Clone)]
pub struct AllDependencies<'tcx> {
    // TODO:
    // crate_name: rustc_middle::DefId, // or maybe a Symbol

    // FIXME: externally defined functions are missing
    /// Informations about functions and closures defined in the current crate
    functions: FxHashMap<DefId, Function<'tcx>>, // calleer -> callsite
}

/// 函数调用类型：
/// 1. 直接调用函数
/// 2. 函数指针调用
#[derive(Clone, Debug)]
enum LocalCallType {
    DirectCall(DefId),
    LocalFunctionPtr(mir::Local),
}

/// 函数调用点，通过extract_function_call进行解析获得caller的callee信息
#[derive(Clone, Debug)]
struct CallSite<'tcx> {
    /// 有两种调用类型，一种是直接调用，一种是函数指针
    function: LocalCallType,
    /// 被调用函数的返回值会传递给的局部变量，如果有就是 Some(...)，否则 None
    return_variable: Option<mir::Local>,
    /// 被调用函数的实参对应的局部变量
    arguments: Vec<mir::Operand<'tcx>>,
}

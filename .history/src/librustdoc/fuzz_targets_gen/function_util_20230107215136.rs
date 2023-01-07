use bit_vec::BitVec;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::DefId;
use rustc_index::vec::IndexVec;
use rustc_middle::mir;
use rustc_middle::mir::Constant;
use rustc_middle::ty::Ty;
use rustc_span::symbol::Symbol;

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
pub enum LocalCallType {
    DirectCall(DefId),
    LocalFunctionPtr(mir::Local),
}

/// 函数调用点，通过extract_function_call进行解析获得caller的callee信息
#[derive(Clone, Debug)]
pub struct CallSite<'tcx> {
    /// 有两种调用类型，一种是直接调用，一种是函数指针
    function: LocalCallType,
    /// 被调用函数的返回值会传递给的局部变量，如果有就是 Some(...)，否则 None
    return_variable: Option<mir::Local>,
    /// 被调用函数的实参对应的局部变量
    arguments: Vec<mir::Operand<'tcx>>,
}

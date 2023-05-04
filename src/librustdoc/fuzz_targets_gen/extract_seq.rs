use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use crate::fuzz_targets_gen::extract_dep::AllDependencies;
use crate::fuzz_targets_gen::extract_dep::{
    extract_arguments, Argument, CalleeDependency, Function,
};
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::def_id::DefId;
use rustc_middle::mir;
use rustc_middle::ty::{self, Ty, TyCtxt};

/// 解析序列
pub struct ExtractSequence {
    all_sequence: Vec<Vec<String>>,
}

impl ExtractSequence {
    // 初始化
    pub fn new() -> Self {
        ExtractSequence { all_sequence: Vec::new() }
    }

    /// 进行一个深度优先搜索，然后生成遍历序列
    /// 获得函数签名之后，就获得了生成序列的源信息
    pub fn extract_sequence<'tcx>(
        &mut self,
        tcx: TyCtxt<'tcx>,
        current_crate_name: String,
        test_crate_name: String,
        all_dependencies: AllDependencies<'tcx>,
        enable: bool,
    ) {
        //如果待测crate就是当前crate，那就返回，因为可能解析到非pub
        if current_crate_name == test_crate_name {
            return;
        }

        // 装入所有解析的序列
        let mut all_seq = Vec::new();
        let mut visit_set = FxHashSet::default();

        if !enable {
            return;
        }

        //遍历每一个本地函数
        for (caller_def_id, function) in all_dependencies.functions.iter() {
            // 满足两个条件:
            // 1. 需要当前crate的API
            // 2. 测试每一个参数，如果有任何一个不是primitive类型的，都会成功
            // if function.arguments.iter().all(|arg| arg.ty.is_primitive_ty()) {
            // 能进入这里，说明参数都是基本类型，说明是我们的起始节点

            // 下面开始dfs
            let mut func_seq = Vec::new();
            let mut stack = Vec::new();

            //dfs的start node，初始化stack。这是一个caller
            let function_info = FunctionInfo::new_by_caller_def_id(
                tcx,
                *caller_def_id,
                &*function,
                &all_dependencies,
            );
            stack.push(function_info);

            // 开始进行dfs，使用栈来避免递归
            while !stack.is_empty() {
                //找到function_info（这是个caller），然后插入visit_set，表示被遍历过防止错误。
                let function_info = stack.pop().unwrap();
                if let CallerOrCallee::Caller { dependency_info, .. } = function_info.content {
                    visit_set.insert(tcx.def_path_str(function_info.def_id));
                    //let caller_def_id = function_info.def_id;

                    //下面对于每一个被调用函数进行遍历
                    let callee_dependency = dependency_info.callee_dependencies.clone();
                    for CalleeDependency { callee, callsite, .. } in callee_dependency {
                        use super::extract_dep::Callee;

                        //被调用函数对应的crate_name和DefId
                        let (_crate_name, callee_def_id) = match callee {
                            Callee::DirectCall(def_id) => {
                                (tcx.crate_name(def_id.krate).as_str().to_string(), def_id)
                            }
                            Callee::LocalFunctionPtr(_) => continue, //跳过
                        };

                        // 如果当前的callsite参数都是primitive type的话，把上一个序列终结，开始新序列
                        if callsite.argument_tys.iter().all(|ty| ty.is_primitive_ty()) {
                            if !func_seq.is_empty() {
                                all_seq.push(func_seq.clone());
                                func_seq.clear();
                            }
                        }

                        /*println!(
                            "caller_name {}, callee_name {}",
                            tcx.def_path_str(caller_def_id),
                            tcx.def_path_str(callsite_def_id),
                        );*/

                        let callee_name = tcx.def_path_str(callee_def_id);
                        //if crate_name.starts_with(&test_crate_name) {
                        if callee_name.starts_with(&test_crate_name) {
                            // 如果是test crate的api，推入序列

                            func_seq.push(callee_name);

                            //如果callsite的返回值是基本类型，就截取这个
                            if callsite.return_ty.is_primitive_ty() {
                                all_seq.push(func_seq.clone());
                                func_seq.clear();
                            }
                        } else {
                            // 如果是是当前crate的local函数，那么就入栈

                            if let Some(_) = FunctionInfo::new_by_callee_def_id(
                                tcx,
                                callee_def_id,
                                &all_dependencies,
                            ) {
                                // 一种剪枝
                                if !visit_set.contains(&tcx.def_path_str(callee_def_id).to_string())
                                {
                                    //既然是local的函数，那么一定可以在all_dependencies里面找到，否则就出了bug
                                    let function =
                                        all_dependencies.functions.get(&callee_def_id).unwrap();
                                    let info = FunctionInfo::new_by_caller_def_id(
                                        tcx,
                                        callee_def_id,
                                        function,
                                        &all_dependencies,
                                    );
                                    // 存入stack供下次遍历
                                    stack.push(info);
                                }
                            }
                        }
                    }
                }
            }

            // dfs完毕，开始进行结束处理
            if !func_seq.is_empty() {
                all_seq.push(func_seq);
            }

            // 结束
            //}
        }

        self.all_sequence = all_seq;
    }

    pub fn extract_sequence_new<'tcx>(
        &mut self,
        tcx: TyCtxt<'tcx>,
        current_crate_name: String,
        test_crate_name: String,
        all_dependencies: AllDependencies<'tcx>,
        enable: bool,
    ) {
        //如果待测crate就是当前crate，那就返回，否则可能解析到非pub
        if current_crate_name == test_crate_name {
            return;
        }

        // 装入所有解析的序列
        let mut all_seq = Vec::new();
        let mut visit_set = FxHashSet::default();

        if !enable {
            return;
        }

        //遍历每一个本地函数
        for (caller_def_id, function) in all_dependencies.functions.iter() {
            // 满足两个条件:
            // 1. 需要当前crate的API
            // 2. 测试每一个参数，如果有任何一个不是primitive类型的，都会成功
            // if function.arguments.iter().all(|arg| arg.ty.is_primitive_ty()) {
            // 能进入这里，说明参数都是基本类型，说明是我们的起始节点

            // 下面开始dfs
            let mut func_seq = Vec::new();
            let mut stack = Vec::new();

            //dfs的start node，初始化stack。这是一个caller
            let function_info = FunctionInfo::new_by_caller_def_id(
                tcx,
                *caller_def_id,
                &*function,
                &all_dependencies,
            );
            stack.push(function_info);

            // 开始进行dfs，使用栈来避免递归
            while !stack.is_empty() {
                //找到function_info（这是个caller），然后插入visit_set，表示被遍历过防止错误。
                let function_info = stack.pop().unwrap();
                if let CallerOrCallee::Caller { dependency_info, .. } = function_info.content {
                    visit_set.insert(tcx.def_path_str(function_info.def_id));
                    //let caller_def_id = function_info.def_id;

                    //下面对于每一个被调用函数进行遍历
                    let callee_dependency = dependency_info.callee_dependencies.clone();
                    for CalleeDependency { callee, callsite, .. } in callee_dependency {
                        use super::extract_dep::Callee;

                        //被调用函数对应的crate_name和DefId
                        let (_crate_name, callee_def_id) = match callee {
                            Callee::DirectCall(def_id) => {
                                (tcx.crate_name(def_id.krate).as_str().to_string(), def_id)
                            }
                            Callee::LocalFunctionPtr(_) => continue, //跳过
                        };

                        // 如果当前的callsite参数都是primitive type的话，把上一个序列终结，开始新序列
                        if callsite.argument_tys.iter().all(|ty| ty.is_primitive_ty()) {
                            if !func_seq.is_empty() {
                                all_seq.push(func_seq.clone());
                                func_seq.clear();
                            }
                        }

                        /*println!(
                            "caller_name {}, callee_name {}",
                            tcx.def_path_str(caller_def_id),
                            tcx.def_path_str(callsite_def_id),
                        );*/

                        let callee_name = tcx.def_path_str(callee_def_id);
                        //if crate_name.starts_with(&test_crate_name) {
                        if callee_name.starts_with(&test_crate_name) {
                            // 如果是test crate的api，推入序列

                            func_seq.push(callee_name);

                            //如果callsite的返回值是基本类型，就截取这个
                            if callsite.return_ty.is_primitive_ty() {
                                all_seq.push(func_seq.clone());
                                func_seq.clear();
                            }
                        } else {
                            // 如果是是当前crate的local函数，那么就入栈

                            if let Some(_) = FunctionInfo::new_by_callee_def_id(
                                tcx,
                                callee_def_id,
                                &all_dependencies,
                            ) {
                                // 一种剪枝
                                if !visit_set.contains(&tcx.def_path_str(callee_def_id).to_string())
                                {
                                    //既然是local的函数，那么一定可以在all_dependencies里面找到，否则就出了bug
                                    let function =
                                        all_dependencies.functions.get(&callee_def_id).unwrap();
                                    let info = FunctionInfo::new_by_caller_def_id(
                                        tcx,
                                        callee_def_id,
                                        function,
                                        &all_dependencies,
                                    );
                                    // 存入stack供下次遍历
                                    stack.push(info);
                                }
                            }
                        }
                    }
                }
            }

            // dfs完毕，开始进行结束处理
            if !func_seq.is_empty() {
                all_seq.push(func_seq);
            }

            // 结束
            //}
        }

        self.all_sequence = all_seq;
    }

    pub fn print_sequence(&self, enable: bool, dir_path: &str, _crate_name: &str) {
        if !enable {
            return;
        }

        let dir_path = PathBuf::from(dir_path).join(_crate_name).join("seq");

        println!("\x1b[94mStart to print sequence\x1b[0m");

        let mut file =
            OpenOptions::new().create(true).append(true).open(dir_path).expect("cannot open file");
        for (idx, seq) in self.all_sequence.iter().enumerate() {
            let s = format!("Sequence {}: ", idx);
            println!("{}", s);
            file.write_all(s.as_bytes()).expect("write failed");
            for func in seq {
                let s = format!("{} ", func);
                print!("{} ", s);
                file.write_all(s.as_bytes()).expect("write failed");
            }

            //写入回车
            println!("");
            file.write_all("\n".as_bytes()).expect("write failed");
        }
        println!("\x1b[94mFinish printing\x1b[0m");
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FunctionInfo<'tcx> {
    def_id: DefId,
    content: CallerOrCallee<'tcx>,
    //mir: mir::Body<'tcx>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum CallerOrCallee<'tcx> {
    Caller {
        mir: mir::Body<'tcx>,
        return_ty: Ty<'tcx>,
        arguments: Vec<Argument<'tcx>>,
        dependency_info: Function<'tcx>,
    },
    Callee {
        fn_sig: ty::FnSig<'tcx>,
    },
}

#[allow(dead_code)]
impl FunctionInfo<'_> {
    /// 判断函数是否是当前crate的
    pub fn is_local(&self) -> bool {
        self.def_id.as_local().is_some()
    }

    /// 对于当前crate的LocalDefId
    pub fn new_by_caller_def_id<'a, 'tcx>(
        tcx: TyCtxt<'tcx>,
        def_id: DefId,
        function: &Function<'tcx>,
        all_dependencies: &'a AllDependencies<'tcx>,
    ) -> FunctionInfo<'tcx> {
        let mir = function.mir.to_owned();

        // 返回值
        let return_ty = mir.local_decls[mir::Local::from_usize(0)].ty;
        // 参数
        let arguments = extract_arguments(&mir);

        let dependency_info = *(all_dependencies
            .functions
            .iter()
            .find(|(x, _)| tcx.def_path_str(**x) == tcx.def_path_str(def_id))
            .unwrap()
            .1)
            .clone();

        FunctionInfo {
            def_id,
            content: CallerOrCallee::Caller { mir, return_ty, arguments, dependency_info },
        }
    }

    pub fn new_by_callee_def_id<'a, 'tcx>(
        tcx: TyCtxt<'tcx>,
        def_id: DefId,
        _all_dependencies: &'a AllDependencies<'tcx>,
    ) -> Option<FunctionInfo<'tcx>> {
        // 获得调用点def_id对应的function的local_def_id
        let local_def_id = match tcx
            .hir()
            .body_owners()
            .find(|x| tcx.def_path_str(x.to_def_id()) == tcx.def_path_str(def_id))
        {
            //当callee的路径和body_owner一样的时候，就可以找到函数体
            Some(local) => local,
            None => {
                return None;
            }
        };

        fn get_function_signature<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> ty::FnSig<'tcx> {
            let fn_type = tcx.fn_sig(def_id);
            let fn_type = fn_type.skip_binder();
            fn_type
        }
        let fn_sig = get_function_signature(tcx, def_id);

        let def_id = local_def_id.to_def_id();
        //Some(FunctionInfo { def_id, mir, return_ty, arguments, dependency_info })
        Some(FunctionInfo { def_id, content: CallerOrCallee::Callee { fn_sig } })
    }
}

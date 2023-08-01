use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use crate::fuzz_targets_gen::extract_dep::AllDependencies;
use crate::fuzz_targets_gen::extract_dep::{
    extract_arguments, Argument, CalleeDependency, Function,
};

use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;
use rustc_middle::mir;
use rustc_middle::ty::{self, Ty, TyCtxt};

/// 解析序列
pub struct ExtractInfo {
    pub all_sequences: Vec<Vec<String>>, //暂时用不到
    pub dependencies_info: FxHashMap<(String, String), usize>,
    pub order_info: FxHashMap<(String, String), usize>,
    pub function_info: FxHashMap<String, usize>,
}

impl ExtractInfo {
    // 初始化
    pub fn new<'tcx>(
        tcx: TyCtxt<'tcx>,
        current_crate_name: String,
        test_crate_name: String,
        all_dependencies: &AllDependencies<'tcx>,
        enable: bool,
    ) -> Self {
        let all_sequences = Self::extract_sequence(
            tcx,
            current_crate_name.clone(),
            test_crate_name.clone(),
            all_dependencies,
            enable,
        );

        let (dependencies_info, order_info, function_info) = Self::extract_info(
            tcx,
            current_crate_name.clone(),
            test_crate_name.clone(),
            all_dependencies,
            enable,
        );

        ExtractInfo { all_sequences, dependencies_info, order_info, function_info }
    }

    /// 进行一个深度优先搜索，然后生成遍历序列
    /// 获得函数签名之后，就获得了生成序列的源信息
    pub fn extract_sequence<'tcx>(
        tcx: TyCtxt<'tcx>,
        current_crate_name: String,
        test_crate_name: String,
        all_dependencies: &AllDependencies<'tcx>,
        enable: bool,
    ) -> Vec<Vec<String>> {
        //如果待测crate就是当前crate，那就返回，因为可能解析到非pub
        if current_crate_name == test_crate_name || !enable {
            return Vec::new();
        }

        // 装入所有解析的序列
        let mut all_seq = Vec::new();
        let mut visit_set = FxHashSet::default();

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

        all_seq
    }

    /// 进行一个深度优先搜索，然后生成遍历序列
    /// 获得函数签名之后，就获得了生成序列的源信息
    pub fn extract_info<'tcx>(
        tcx: TyCtxt<'tcx>,
        current_crate_name: String,
        test_crate_name: String,
        all_dependencies: &AllDependencies<'tcx>,
        enable: bool,
    ) -> (
        FxHashMap<(String, String), usize>,
        FxHashMap<(String, String), usize>,
        FxHashMap<String, usize>,
    ) {
        //如果待测crate就是当前crate，那就返回，因为可能解析到非pub
        if current_crate_name == test_crate_name || !enable {
            return (FxHashMap::default(), FxHashMap::default(), FxHashMap::default());
        }

        // 用于剪枝，访问过的API就不用访问了
        let mut visit_set = FxHashSet::default();

        // 依赖哈希表，用于减小文件量的
        let mut pre_succ_map = FxHashMap::default();
        let mut order_map = FxHashMap::default();
        let mut function_map = FxHashMap::default();

        //遍历每一个本地函数
        for (caller_def_id, function) in all_dependencies.functions.iter() {
            // 满足两个条件:
            // 1. 需要当前crate的API
            // 2. 测试每一个参数，如果有任何一个不是primitive类型的，都会成功
            // if function.arguments.iter().all(|arg| arg.ty.is_primitive_ty()) {
            // 能进入这里，说明参数都是基本类型，说明是我们的起始节点

            // 下面开始dfs
            if visit_set.contains(&tcx.def_path_str(*caller_def_id)) {
                continue;
            }

            //dfs的start node，初始化stack。这是一个caller
            let function_info = FunctionInfo::new_by_caller_def_id(
                tcx,
                *caller_def_id,
                &*function,
                &all_dependencies,
            );

            if let CallerOrCallee::Caller { dependency_info, .. } = function_info.content {
                visit_set.insert(tcx.def_path_str(function_info.def_id));

                //下面对于每一个被调用函数进行遍历
                let callee_dependency = dependency_info.callee_dependencies.clone();

                use super::extract_dep::Callee;

                //下面遍历每个callee，解析order_info
                let mut order_sequence = Vec::new();
                for CalleeDependency { callee, .. } in &callee_dependency {
                    //被调用函数对应的crate_name和DefId
                    let (_crate_name, callee_def_id) = match callee {
                        Callee::DirectCall(def_id) => {
                            (tcx.crate_name(def_id.krate).as_str().to_string(), def_id)
                        }
                        Callee::LocalFunctionPtr(_) => continue, //跳过
                    };
                    let callee_name = tcx.def_path_str(*callee_def_id);
                    if callee_name.starts_with(&test_crate_name) {
                        order_sequence.push(callee_name.clone());
                    }
                }
                if order_sequence.len() >= 2 {
                    for i in 0..(order_sequence.len() - 1) {
                        order_map
                            .insert((order_sequence[i].clone(), order_sequence[i + 1].clone()), 1);
                    }
                }
                //下面遍历每个callee，解析dependency_info
                for CalleeDependency { callee, arg_sources, .. } in &callee_dependency {
                    //被调用函数对应的crate_name和DefId
                    let (_crate_name, callee_def_id) = match callee {
                        Callee::DirectCall(def_id) => {
                            (tcx.crate_name(def_id.krate).as_str().to_string(), def_id)
                        }
                        Callee::LocalFunctionPtr(_) => continue, //跳过
                    };

                    let callee_name = tcx.def_path_str(*callee_def_id);
                    //if crate_name.starts_with(&test_crate_name) {
                    if callee_name.starts_with(&test_crate_name) {
                        //先加入function_info
                        if function_map.contains_key(&callee_name) {
                            function_map.insert(
                                callee_name.clone(),
                                function_map.get(&callee_name).unwrap() + 1,
                            );
                        } else {
                            function_map.insert(callee_name.clone(), 1);
                        }

                        // 如果是test crate的api
                        // 检查每个参数，如果有依赖关系的话，就可以把元组推入
                        for (_, arg_srcs) in arg_sources {
                            for arg_src in arg_srcs {
                                match arg_src{
                                        crate::fuzz_targets_gen::extract_dep::Source::ReturnVariable(pre_id) => {
                                            let pre_function_name = tcx.def_path_str(*pre_id);
                                            //只有前驱是tested_lib中的才行
                                            if pre_function_name.starts_with(&test_crate_name)
                                            {
                                                let succ_function_name = callee_name.clone();
                                                let tuple = (pre_function_name, succ_function_name);
                                                //如果有就更新，没有就继续
                                                if pre_succ_map.contains_key(&tuple) {
                                                    pre_succ_map.insert(tuple.clone(), pre_succ_map.get(&tuple).unwrap()+1);
                                                }else{
                                                    pre_succ_map.insert(tuple.clone(), 1);
                                                }
                                            }
                                        },
                                        _=>{
                                            //不是返回值依赖的话，什么也不用做
                                            //FIXME:可以在这添加过程间分析
                                        }
                                    }
                            }
                        }
                    } else {
                        // 如果是是当前crate的local函数，那么就入栈
                        if let Some(_) = FunctionInfo::new_by_callee_def_id(
                            tcx,
                            *callee_def_id,
                            &all_dependencies,
                        ) {}
                    }
                }
            }
        }

        (pre_succ_map, order_map, function_map)
    }

    pub fn print_sequence(&self, enable: bool, dir_path: &str, _crate_name: &str) {
        if !enable {
            return;
        }

        let dir_path = PathBuf::from(dir_path).join(_crate_name).join("seq");

        println!("\x1b[94mStart to print sequence:{:?}\x1b[0m", dir_path);

        let mut file =
            OpenOptions::new().create(true).append(true).open(dir_path).expect("cannot open file");
        for (idx, seq) in self.all_sequences.iter().enumerate() {
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

    pub fn print_dependencies_info(&self, enable: bool, dir_path: &str, _crate_name: &str) {
        if !enable {
            return;
        }

        let dir_path = PathBuf::from(dir_path).join(_crate_name).join("depinfo");

        println!("\x1b[94mStart to print dependency info extracted from corpus.\x1b[0m");

        let mut file =
            OpenOptions::new().create(true).append(true).open(dir_path).expect("cannot open file");
        for (idx, ((pre_func, succ_func), num)) in self.dependencies_info.iter().enumerate() {
            let s = format!(
                "pair_{}:   {}   {}   {}",
                idx,
                _get_function_name(pre_func.clone()),
                _get_function_name(succ_func.clone()),
                num
            );
            println!("{}", s);
            file.write_all(s.as_bytes()).expect("write failed");

            //写入回车
            println!("");
            file.write_all("\n".as_bytes()).expect("write failed");
        }

        println!("\x1b[94mFinish printing\x1b[0m");
    }

    pub fn print_order_info(&self, enable: bool, dir_path: &str, _crate_name: &str) {
        if !enable {
            return;
        }

        let dir_path = PathBuf::from(dir_path).join(_crate_name).join("orderinfo");

        println!("\x1b[94mStart to print order info extracted from corpus.\x1b[0m");

        let mut file =
            OpenOptions::new().create(true).append(true).open(dir_path).expect("cannot open file");
        for (idx, ((before_func, after_func), num)) in self.order_info.iter().enumerate() {
            let s = format!(
                "pair_{}:   {}   {}   {}",
                idx,
                _get_function_name(before_func.clone()),
                _get_function_name(after_func.clone()),
                num
            );
            println!("{}", s);
            file.write_all(s.as_bytes()).expect("write failed");

            //写入回车
            println!("");
            file.write_all("\n".as_bytes()).expect("write failed");
        }

        println!("\x1b[94mFinish printing\x1b[0m");
    }

    pub fn print_functions_info(&self, enable: bool, dir_path: &str, _crate_name: &str) {
        if !enable {
            return;
        }

        let dir_path = PathBuf::from(dir_path).join(_crate_name).join("funcinfo");

        println!("\x1b[94mStart to print function info extracted from corpus.\x1b[0m");

        let mut file =
            OpenOptions::new().create(true).append(true).open(dir_path).expect("cannot open file");
        for (idx, (func, num)) in self.function_info.iter().enumerate() {
            let s = format!(
                "{:?}:   _func_{}:   {}   {}",
                std::env::current_dir().unwrap(),
                idx,
                _get_function_name(func.clone()),
                num
            );
            println!("{}", s);
            file.write_all(s.as_bytes()).expect("write failed");

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
    //这个函数的def_id，是函数定义的ID
    def_id: DefId,
    content: CallerOrCallee<'tcx>,
    //mir: mir::Body<'tcx>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum CallerOrCallee<'tcx> {
    /// 对于caller，需要有函数体，返回值类型，参数的名字以及编号，以及它体内的Function，其实是有点冗余。。。
    Caller {
        mir: mir::Body<'tcx>,
        return_ty: Ty<'tcx>,
        arguments: Vec<Argument<'tcx>>,
        dependency_info: Function<'tcx>,
    },
    //对于被调用函数，只要有签名即可
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

        //找到all_dependencies中caller_def_id对应的，因为传进来的Function不一定是这个
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

fn _get_function_name(name: String) -> String {
    // If no name can be found, return an empty string

    // 使用正则表达式匹配函数名字符串
    //println!("1111111111111111111111111");
    use regex::Regex;
    let re = Regex::new(r"::<[A-Za-z]+>").unwrap();
    let res = re.replace_all(&name, "").to_string();

    //println!("2222222222222222222222222222222222222222");
    res
    //name
}

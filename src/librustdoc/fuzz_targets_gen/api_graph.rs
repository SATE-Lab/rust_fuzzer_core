use super::api_sequence::ReverseApiSequence;
use super::fuzz_type;
use crate::clean::{self, types};
use crate::formats::cache::Cache;
use crate::fuzz_targets_gen::api_function::ApiFunction;
use crate::fuzz_targets_gen::api_sequence::{ApiCall, ApiSequence, ParamType};
use crate::fuzz_targets_gen::api_util::{self};
use crate::fuzz_targets_gen::call_type::CallType;
use crate::fuzz_targets_gen::fuzz_type::FuzzableType;
use crate::fuzz_targets_gen::impl_util::FullNameMap;
use crate::fuzz_targets_gen::mod_visibility::ModVisibity;
use crate::fuzz_targets_gen::prelude_type::{self, PreludeType};
use itertools::Itertools;
use rand::thread_rng;
use rand::Rng;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_middle::ty::Visibility;
use std::time::Duration;
//use super::generic_function::GenericFunction;

lazy_static! {
    static ref RANDOM_WALK_STEPS: FxHashMap<&'static str, usize> = {
        let mut m = FxHashMap::default();
        m.insert("regex", 10000);
        m.insert("url", 10000);
        m.insert("time", 10000);
        m
    };
}

lazy_static! {
    static ref CAN_COVER_NODES: FxHashMap<&'static str, usize> = {
        let mut m = FxHashMap::default();
        m.insert("regex", 96);
        m.insert("serde_json", 41);
        m.insert("clap", 66);
        m
    };
}

#[derive(Clone, Debug)]
pub(crate) struct ApiGraph<'a> {
    /// 当前crate的名字
    pub(crate) _crate_name: String,

    /// 当前待测crate里面公开的API
    pub(crate) api_functions: Vec<ApiFunction>,

    /// 在bfs的时候，访问过的API不再访问
    pub(crate) api_functions_visited: Vec<bool>,

    /// 根据函数签名解析出的API依赖关系
    pub(crate) api_dependencies: Vec<ApiDependency>,

    /// 生成的一切可能的API序列
    pub(crate) api_sequences: Vec<ApiSequence>,

    /// DefId到名字的映射
    pub(crate) full_name_map: FullNameMap,

    /// the visibility of mods，to fix the problem of `pub(crate) use`
    pub(crate) mod_visibility: ModVisibity,

    ///暂时不支持的
    //pub(crate) generic_functions: Vec<GenericFunction>,
    pub(crate) functions_with_unsupported_fuzzable_types: FxHashSet<String>,
    pub(crate) cache: &'a Cache,
    //pub(crate) _sequences_of_all_algorithm : FxFxHashMap<GraphTraverseAlgorithm, Vec<ApiSequence>>
}

use core::fmt::Debug;
use std::thread::sleep;

impl Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache").finish()
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum GraphTraverseAlgorithm {
    _Default,
    _Bfs,
    _FastBfs,
    _BfsEndPoint,
    _FastBfsEndPoint,
    _RandomWalk,
    _RandomWalkEndPoint,
    _TryDeepBfs,
    _DirectBackwardSearch,
    _UseRealWorld, //当前的方法，使用解析出来的sequence
    _Fudge,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Hash, Eq, PartialEq, Copy)]
pub(crate) enum ApiType {
    BareFunction,
    GenericFunction, //currently not support now
}

//函数的依赖关系
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub(crate) struct ApiDependency {
    pub(crate) output_fun: (ApiType, usize), //the index of first func
    pub(crate) input_fun: (ApiType, usize),  //the index of second func
    pub(crate) input_param_index: usize,     //参数的索引
    pub(crate) call_type: CallType,          //调用类型
}

impl<'a> ApiGraph<'a> {
    /// 新建一个api_graph
    pub(crate) fn new(_crate_name: &String, cache: &'a Cache) -> Self {
        //let _sequences_of_all_algorithm = FxFxHashMap::default();
        ApiGraph {
            _crate_name: _crate_name.to_owned(),
            api_functions: Vec::new(),
            api_functions_visited: Vec::new(),
            api_dependencies: Vec::new(),
            api_sequences: Vec::new(),
            full_name_map: FullNameMap::new(),
            mod_visibility: ModVisibity::new(_crate_name),
            //generic_functions: Vec::new(),
            functions_with_unsupported_fuzzable_types: FxHashSet::default(),
            cache,
        }
    }

    /// 向api_graph中投入function，包括method和bare function，支持泛型
    pub(crate) fn add_api_function(&mut self, mut api_fun: ApiFunction) {
        /*if api_fun._is_generic_function() {
            let generic_function = GenericFunction::from(api_fun);
            // self.generic_functions.push(generic_function);
        } else*/
        //泛型函数不会单独考虑
        if api_fun.contains_unsupported_fuzzable_type(self.cache, &self.full_name_map) {
            self.functions_with_unsupported_fuzzable_types.insert(api_fun.full_name.clone());
        } else {
            // FIXME:新加入泛型
            //既然支持了泛型函数，就要初始化generic_substitution
            for generic_arg in &api_fun._generics.params {
                //当这个是泛型类型（而不是生命周期等）
                if let types::GenericParamDefKind::Type { .. } = generic_arg.kind {
                    let generic_name = generic_arg.name.to_string();
                    //暂时只支持把泛型替换成i32
                    api_fun
                        .generic_substitutions
                        .insert(generic_name, clean::Type::Primitive(clean::PrimitiveType::I32));
                }
            }
            self.api_functions.push(api_fun);
        }
    }

    /// 遍历到某个mod的时候，添加mod的可见性，为过滤出可见的api做准备
    pub(crate) fn add_mod_visibility(&mut self, mod_name: &String, visibility: &Visibility) {
        self.mod_visibility.add_one_mod(mod_name, visibility);
    }

    /// 根据prelude type和可见性来过滤api
    pub(crate) fn filter_functions(&mut self, support_generic: bool) {
        self.filter_functions_defined_on_prelude_type();
        self.filter_api_functions_by_mod_visibility();

        /*for (idx, api) in self.api_functions.iter().enumerate() {
            println!(
                "api_functions[{}]: {}",
                idx,
                api._pretty_print(self.cache, &self.full_name_map)
            )
        }*/

        // 不支持泛型，就把泛型过滤出来
        if !support_generic {
            let mut new_api_function = Vec::new();
            for func in &self.api_functions {
                //if func._generics.params.len() == 0 {
                if !func._is_generic_function()
                    && !func.full_name.contains("from_static")
                    && !func.full_name.contains("with_capacity")
                    && !func.full_name.contains("TimeDelta")
                    && !func.full_name.contains("from_raw_parts_mut")
                    && !func.full_name.contains("xi_core_lib::core::XiCore::inner")
                    && !func.full_name.contains("WidthBatchReq::request")
                    && !func.full_name.contains("path_segments_mut")
                    && !func.full_name.contains("with_user_event")
                    && !func.full_name.contains("keyboard")
                    && !func.full_name.contains("scancode")
                {
                    new_api_function.push(func.clone());
                }
            }
            self.api_functions = new_api_function;
        }
        println!("filtered api functions contain {} apis", self.api_functions.len());
    }

    /// 过滤api，一些预装类型的function，比如Result...不在我这个crate里，肯定要过滤掉
    pub(crate) fn filter_functions_defined_on_prelude_type(&mut self) {
        let prelude_types = prelude_type::get_all_preluded_type();
        if prelude_types.len() <= 0 {
            return;
        }
        self.api_functions = self
            .api_functions
            .drain(..)
            .filter(|api_function| api_function.is_not_defined_on_prelude_type(&prelude_types))
            .collect();
    }

    /// 过滤api，根据可见性进行过滤，不是pub就过滤掉
    /// FIXME:  是否必要
    pub(crate) fn filter_api_functions_by_mod_visibility(&mut self) {
        if self.mod_visibility.inner.is_empty() {
            panic!("No mod!!!!!!");
        }

        let invisible_mods = self.mod_visibility.get_invisible_mods();

        let mut new_api_functions = Vec::new();

        //遍历api_graph中的所有的api
        for api_func in &self.api_functions {
            let api_func_name = &api_func.full_name;
            let trait_full_path = &api_func._trait_full_path;
            let mut invisible_flag = false;
            for invisible_mod in &invisible_mods {
                // 两种情况下api不可见：
                // 1. crate::m1::m2::api中的某个mod不可见
                // 2. api实现了某个trait，同时trait不可见
                if api_func_name.as_str().starts_with(invisible_mod.as_str()) {
                    invisible_flag = true;
                    break;
                }
                if api_func_name.as_str().ends_with("lossy_normalization")
                    || api_func_name.as_str().ends_with(":TokenizerBuilder::new")
                {
                    invisible_flag = true;
                    break;
                }

                if let Some(trait_full_path) = trait_full_path {
                    if trait_full_path.as_str().starts_with(invisible_mod) {
                        invisible_flag = true;
                        break;
                    }
                }
            }

            // parent所在mod可见
            if !invisible_flag && api_func.visibility.is_public() {
                new_api_functions.push(api_func.clone());
            }
        }
        self.api_functions = new_api_functions;
    }

    pub(crate) fn set_full_name_map(&mut self, full_name_map: &FullNameMap) {
        self.full_name_map = full_name_map.clone();
    }

    ///找到所有可能的依赖关系，存在api_dependencies中，供后续使用
    pub(crate) fn find_all_dependencies(&mut self, support_generic: bool) {
        println!("find_dependencies");
        self.api_dependencies.clear();

        // 两个api_function之间的dependency
        // 其中i和j分别是first_fun和second_fun在api_graph的index
        for (i, first_fun) in self.api_functions.iter().enumerate() {
            if first_fun._is_end_function(self.cache, &self.full_name_map, support_generic) {
                //如果第一个函数是终止节点，就不寻找这样的依赖
                continue;
            }

            if let Some(ty_) = &first_fun.output {
                let mut output_type = ty_.clone();

                //FIXME: 因为很多new或者什么的返回值是Some(T)或者Ok(T)
                //在这里对unwrap做特殊处理！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！
                //如果output_type是Option或者Result，那么就先提取出来，下面_same_type就减少了unwrap那一步，之后生成的部分就不会多出来一个unwrap
                //后面在生成function call字符串的时候，特殊考虑一下，如果output_type就直接.unwrap()就好了
                if prelude_type::_prelude_type_need_special_dealing(
                    &output_type,
                    self.cache,
                    &self.full_name_map,
                ) {
                    //如果是option或者result，先转化prelude_type，然后让output变成里面包装的东西
                    let prelude_type =
                        PreludeType::from_type(&output_type, self.cache, &self.full_name_map);
                    output_type = prelude_type._get_final_type();
                }

                //！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！！

                for (j, second_fun) in self.api_functions.iter().enumerate() {
                    //FIXME:是否要把i=j的情况去掉？
                    if second_fun._is_start_function(
                        self.cache,
                        &self.full_name_map,
                        support_generic,
                    ) {
                        //如果第二个节点是开始节点，那么直接跳过
                        continue;
                    }
                    /*println!(
                        "\nThe first function {} is: {}",
                        i,
                        first_fun._pretty_print(self.cache, &self.full_name_map)
                    );
                    println!(
                        "The second function {} is: {}",
                        j,
                        second_fun._pretty_print(self.cache, &self.full_name_map)
                    );*/
                    //FIXME:写一个替换函数，在这里就把type给替换掉。

                    // 下面开始正题
                    // 对于second_fun的每个参数，看看first_fun的返回值是否对应得上
                    for (k, input_type) in second_fun.inputs.iter().enumerate() {
                        let mut input_type = input_type.clone();
                        //为了添加泛型支持，在这里先替换
                        /*println!(
                            "替换前output: {}",
                            api_util::_type_name(&output_type, self.cache, &self.full_name_map)
                                .as_str()
                        );
                        println!(
                            "替换前input: {}",
                            api_util::_type_name(&input_type, self.cache, &self.full_name_map)
                                .as_str()
                        );*/
                        if support_generic {
                            output_type = match api_util::substitute_type(
                                output_type.clone(),
                                &first_fun.generic_substitutions,
                            ) {
                                Some(substi) => substi,
                                None => {
                                    continue;
                                }
                            };
                            input_type = match api_util::substitute_type(
                                input_type.clone(),
                                &second_fun.generic_substitutions,
                            ) {
                                Some(substi) => substi,
                                None => {
                                    continue;
                                }
                            };
                        } else {
                            /*println!(
                                "找：{}",
                                api_util::_type_name(&output_type, self.cache, &self.full_name_map)
                            );*/
                            if (!prelude_type::_prelude_type_need_special_dealing(
                                &output_type,
                                self.cache,
                                &self.full_name_map,
                            ) && api_util::_is_generic_type(&output_type))
                                || (!prelude_type::_prelude_type_need_special_dealing(
                                    &input_type,
                                    self.cache,
                                    &self.full_name_map,
                                ) && api_util::_is_generic_type(&input_type))
                            {
                                /*println!(
                                    "找到了泛型，不支持：{}",
                                    api_util::_type_name(
                                        &output_type,
                                        self.cache,
                                        &self.full_name_map
                                    )
                                );*/
                                continue;
                            }
                        }
                        /*println!(
                            "替换后output: {}",
                            api_util::_type_name(&output_type, self.cache, &self.full_name_map)
                                .as_str()
                        );
                        println!(
                            "替换后input: {}",
                            api_util::_type_name(&input_type, self.cache, &self.full_name_map)
                                .as_str()
                        );*/
                        let call_type = api_util::_same_type(
                            &output_type,
                            &input_type,
                            true,
                            self.cache,
                            &self.full_name_map,
                        );
                        match &call_type {
                            CallType::_NotCompatible => {
                                //如果无法转换，那就算了
                                continue;
                            }
                            _ => {
                                //println!("ok, it's ok!!!");
                                //如果可以转换的话，那就存入依赖列表里
                                let one_dependency = ApiDependency {
                                    output_fun: (ApiType::BareFunction, i),
                                    input_fun: (ApiType::BareFunction, j),
                                    input_param_index: k,
                                    call_type: call_type.clone(),
                                };
                                self.api_dependencies.push(one_dependency);
                            }
                        }
                    }
                }
            }
        }

        println!(
            "find_dependencies finished! Num of dependencies is {}.",
            self.api_dependencies.len()
        );
    }

    pub(crate) fn _default_generate_sequences(&mut self, lib_name: &str) {
        //BFS + backward search
        self.generate_all_possoble_sequences(
            GraphTraverseAlgorithm::_BfsEndPoint,
            lib_name,
            300,
            200,
            false,
        );
        self._try_to_cover_unvisited_nodes();

        // backward search
        //self.generate_all_possoble_sequences(GraphTraverseAlgorithm::_DirectBackwardSearch);
    }

    pub(crate) fn generate_all_possoble_sequences(
        &mut self,
        algorithm: GraphTraverseAlgorithm,
        lib_name: &str,
        max_num: usize,
        max_len: usize,
        support_generic: bool,
    ) {
        //BFS序列的最大长度：即为函数的数量,或者自定义
        //let bfs_max_len = self.api_functions.len();
        let bfs_max_len = 5;
        //random walk的最大步数

        /*
        let random_walk_max_size = if RANDOM_WALK_STEPS.contains_key(self._crate_name.as_str()) {
            RANDOM_WALK_STEPS.get(self._crate_name.as_str()).unwrap().clone()
        } else {
            100000
        };*/

        let random_walk_max_size = 100000;

        //no depth bound
        let random_walk_max_depth = 0;
        //try deep sequence number
        let max_sequence_number = 100000;
        match algorithm {
            GraphTraverseAlgorithm::_Bfs => {
                println!("using bfs");
                self.bfs(bfs_max_len, false, false);
            }
            GraphTraverseAlgorithm::_FastBfs => {
                println!("using fastbfs");
                self.bfs(bfs_max_len, false, true);
            }
            GraphTraverseAlgorithm::_BfsEndPoint | GraphTraverseAlgorithm::_Default => {
                println!("using bfs end point");
                self.bfs(bfs_max_len, true, false);
            }
            GraphTraverseAlgorithm::_FastBfsEndPoint => {
                println!("using fast bfs end point");
                self.bfs(bfs_max_len, true, true);
            }
            GraphTraverseAlgorithm::_TryDeepBfs => {
                println!("using try deep bfs");
                //self.real_world(lib_name);
                self._try_deep_bfs(max_sequence_number);
            }

            GraphTraverseAlgorithm::_RandomWalkEndPoint => {
                println!("using random walk end point");
                self.random_walk(random_walk_max_size, true, random_walk_max_depth);
            }

            GraphTraverseAlgorithm::_DirectBackwardSearch => {
                println!("using backward search");
                self.api_sequences.clear();
                self.reset_visited();
                self._try_to_cover_unvisited_nodes();
            }
            GraphTraverseAlgorithm::_RandomWalk => {
                println!("using random walk");
                self.random_walk(max_num, false, max_len);
            }
            GraphTraverseAlgorithm::_UseRealWorld => {
                println!("using realworld to generate");
                //self.real_world(lib_name);
                self.my_method(lib_name, max_num, max_len, support_generic);
                //self._try_to_cover_unvisited_nodes();
            }
            GraphTraverseAlgorithm::_Fudge => {
                println!("using realworld to generate");
                self.fudge(lib_name);
            }
        }
    }

    pub(crate) fn reset_visited(&mut self) {
        self.api_functions_visited.clear();
        let api_function_num = self.api_functions.len();
        for _ in 0..api_function_num {
            self.api_functions_visited.push(false);
        }
        //FIXME:还有别的序列可能需要reset
    }

    //检查是否所有函数都访问过了
    pub(crate) fn check_all_visited(&self) -> bool {
        let mut visited_nodes = 0;
        for visited in &self.api_functions_visited {
            if *visited {
                visited_nodes = visited_nodes + 1;
            }
        }
        /*
        if CAN_COVER_NODES.contains_key(self._crate_name.as_str()) {
            let to_cover_nodes = CAN_COVER_NODES.get(self._crate_name.as_str()).unwrap().clone();
            if visited_nodes == to_cover_nodes {
                return true;
            } else {
                return false;
            }
        }*/

        if visited_nodes == self.api_functions_visited.len() {
            return true;
        } else {
            return false;
        }
    }

    //已经访问过的节点数量,用来快速判断bfs是否还需要run下去：如果一轮下来，bfs的长度没有发生变化，那么也可直接quit了
    pub(crate) fn _visited_nodes_num(&self) -> usize {
        let visited: Vec<&bool> =
            (&self.api_functions_visited).into_iter().filter(|x| **x == true).collect();
        visited.len()
    }

    //生成函数序列，且指定调用的参数
    //加入对fast mode的支持
    pub(crate) fn bfs(&mut self, max_len: usize, stop_at_end_function: bool, fast_mode: bool) {
        //清空所有的序列
        //self.api_sequences.clear();
        self.reset_visited();
        if max_len < 1 {
            return;
        }

        let api_function_num = self.api_functions.len();

        //无需加入长度为1的，从空序列开始即可，加入一个长度为0的序列作为初始
        let api_sequence = ApiSequence::new();
        self.api_sequences.push(api_sequence);

        //接下来开始从长度1一直到max_len遍历
        for len in 0..max_len {
            let mut tmp_sequences = Vec::new();
            for sequence in &self.api_sequences {
                if stop_at_end_function && self.is_sequence_ended(sequence, false) {
                    //如果需要引入终止函数，并且当前序列的最后一个函数是终止函数，那么就不再继续添加
                    continue;
                }
                if sequence.len() == len {
                    tmp_sequences.push(sequence.clone());
                }
            }

            for sequence in &tmp_sequences {
                //长度为len的序列，去匹配每一个函数，如果可以加入的话，就生成一个新的序列
                let api_type = ApiType::BareFunction;
                for api_func_index in 0..api_function_num {
                    //bfs fast, 访问过的函数不再访问
                    if fast_mode && self.api_functions_visited[api_func_index] {
                        continue;
                    }
                    if let Some(new_sequence) =
                        self.is_fun_satisfied(&api_type, api_func_index, sequence)
                    {
                        self.api_sequences.push(new_sequence);
                        self.api_functions_visited[api_func_index] = true;

                        //bfs fast，如果都已经别访问过，直接退出
                        if self.check_all_visited() {
                            //println!("bfs all visited");
                            //return;
                        }
                    }
                }
            }
        }

        println!("There are total {} sequences after bfs", self.api_sequences.len());
        /*if !stop_at_end_function {
            std::process::exit(0);
        }*/
    }

    //为探索比较深的路径专门进行优化
    //主要还是针对比较大的库,函数比较多的
    pub(crate) fn _try_deep_bfs(&mut self, max_sequence_number: usize) {
        //清空所有的序列
        self.api_sequences.clear();
        self.reset_visited();
        let max_len = self.api_functions.len();
        if max_len < 1 {
            return;
        }

        let api_function_num = self.api_functions.len();

        //无需加入长度为1的，从空序列开始即可，加入一个长度为0的序列作为初始
        let api_sequence = ApiSequence::new();
        self.api_sequences.push(api_sequence);

        let mut already_covered_nodes = FxHashSet::default();
        let mut already_covered_edges = FxHashSet::default();
        //接下来开始从长度1一直到max_len遍历
        for len in 0..max_len {
            let current_sequence_number = self.api_sequences.len();
            let covered_nodes = self._visited_nodes_num();
            let mut has_new_coverage_flag = false;
            if len > 2 && current_sequence_number * covered_nodes >= max_sequence_number {
                break;
            }

            let mut tmp_sequences = Vec::new();
            for sequence in &self.api_sequences {
                if self.is_sequence_ended(sequence, false) {
                    //如果需要引入终止函数，并且当前序列的最后一个函数是终止函数，那么就不再继续添加
                    continue;
                }
                if sequence.len() == len {
                    tmp_sequences.push(sequence.clone());
                }
            }
            for sequence in &tmp_sequences {
                //长度为len的序列，去匹配每一个函数，如果可以加入的话，就生成一个新的序列
                let api_type = ApiType::BareFunction;
                for api_func_index in 0..api_function_num {
                    if let Some(new_sequence) =
                        self.is_fun_satisfied(&api_type, api_func_index, sequence)
                    {
                        let covered_nodes = new_sequence._get_contained_api_functions();
                        for covered_node in &covered_nodes {
                            if !already_covered_nodes.contains(covered_node) {
                                already_covered_nodes.insert(*covered_node);
                                has_new_coverage_flag = true;
                            }
                        }

                        let covered_edges = &new_sequence._covered_dependencies;
                        for covered_edge in covered_edges {
                            if !already_covered_edges.contains(covered_edge) {
                                already_covered_edges.insert(*covered_edge);
                                has_new_coverage_flag = true;
                            }
                        }

                        self.api_sequences.push(new_sequence);
                        self.api_functions_visited[api_func_index] = true;
                    }
                }
            }
            if !has_new_coverage_flag {
                println!("forward bfs can not find more.");
                break;
            }
        }
    }

    pub(crate) fn random_walk(
        &mut self,
        max_size: usize, //最大长度
        stop_at_end_function: bool,
        max_len: usize, //需要生成的长度
    ) {
        self.api_sequences.clear();
        self.reset_visited();

        //没有函数的话，直接return
        if self.api_functions.len() <= 0 {
            return;
        }

        //加入一个长度为0的序列
        let api_sequence = ApiSequence::new();
        self.api_sequences.push(api_sequence);

        //start random work
        let function_len = self.api_functions.len();
        let mut rng = thread_rng();

        let mut seq_num = 0;
        // max_size是api序列的最大数量
        loop {
            let current_sequence_len = self.api_sequences.len();
            let chosen_sequence_index = rng.gen_range(0, current_sequence_len);
            let chosen_sequence = &self.api_sequences[chosen_sequence_index];
            //如果需要在终止节点处停止
            if stop_at_end_function && self.is_sequence_ended(&chosen_sequence, false) {
                continue;
            }

            //如果深度没有很深，就继续加

            let chosen_fun_index = rng.gen_range(0, function_len);
            //let chosen_fun = &self.api_functions[chosen_fun_index];
            let fun_type = ApiType::BareFunction;
            if let Some(new_sequence) =
                self.is_fun_satisfied(&fun_type, chosen_fun_index, chosen_sequence)
            {
                self.api_sequences.push(new_sequence.clone());

                self.api_functions_visited[chosen_fun_index] = true;

                if new_sequence.len() >= max_len {
                    //println!("api_functions {}", new_sequence.len());
                    seq_num += 1;
                    if seq_num > max_size {
                        break;
                    }
                }

                //如果全都已经访问过，直接退出
                // if self.check_all_visited() {
                //     println!("random run {} times", i);
                //     return;
                // }
            }
        }
    }

    pub(crate) fn fudge(&mut self, lib_name: &str) {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let mut sequences = Vec::new();

        //在语料库中所有API
        let mut apis_existing_in_corpus_map = FxHashMap::default();

        let seq_file_path = format!(
            "/home/yxz/workspace/fuzz/experiment_root/{}/seq-dedup.ans",
            lib_name.to_string().replace("-", "_")
        );
        println!("{}", seq_file_path);
        let file = File::open(seq_file_path).unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();
            let fields = line.split("|").into_iter().map(|x| x.to_string()).collect_vec();

            // 1.解析出序列频率

            let freq = fields.get(1).unwrap();
            let cnt_str: String = freq.chars().filter(|c| c.is_digit(10)).collect();
            let parsed_number: i32 = cnt_str.parse().unwrap();

            // 2.解析sequence

            let sequence = fields.last().unwrap().clone();
            //获得api的名字
            let functions: Vec<String> = sequence
                .split(" ")
                .map(|x| x.to_string())
                .filter(|x| x.len() > 1) //过滤""
                .collect();

            for func in functions.clone() {
                if apis_existing_in_corpus_map.contains_key(&func) {
                    //包含这个func，就加上去
                    apis_existing_in_corpus_map.insert(
                        func.clone(),
                        apis_existing_in_corpus_map.get(&func).unwrap() + parsed_number,
                    );
                } else {
                    //如果没有，就创建这个entry
                    apis_existing_in_corpus_map.insert(func, parsed_number);
                }
            }

            sequences.push(functions.clone());

            //打印出名字
            println!("被解析出来的合理的序列: {:?}", functions);
        }

        // check一下有没有corpus都在里面
        //获得所有存在于corpus里面API的名字
        for (apis, _) in &apis_existing_in_corpus_map {
            println!("存在于corpus的function{}", apis);
        }

        //清空所有的序列
        self.api_sequences.clear();
        self.reset_visited();
        let max_len = 4;
        if max_len < 1 {
            return;
        }

        self.api_sequences.clear();
        self.reset_visited();

        //没有函数的话，直接return
        if self.api_functions.len() <= 0 {
            return;
        }

        //加入一个长度为0的序列
        let api_sequence = ApiSequence::new();
        self.api_sequences.push(api_sequence);

        //start random work
        let function_len = self.api_functions.len();
        let mut rng = thread_rng();

        let mut seq_num = 0;
        // max_size是api序列的最大数量
        for _ in 0..10000000 {
            let current_sequence_len = self.api_sequences.len();
            let chosen_sequence_index = rng.gen_range(0, current_sequence_len);
            let chosen_sequence = &self.api_sequences[chosen_sequence_index];
            //如果需要在终止节点处停止

            //如果深度没有很深，就继续加

            let chosen_fun_index = rng.gen_range(0, function_len);
            if !apis_existing_in_corpus_map
                .contains_key(&self.api_functions[chosen_fun_index].full_name)
            {
                //continue;
            }
            //let chosen_fun = &self.api_functions[chosen_fun_index];
            let fun_type = ApiType::BareFunction;
            if let Some(new_sequence) =
                self.is_fun_satisfied(&fun_type, chosen_fun_index, chosen_sequence)
            {
                self.api_sequences.push(new_sequence.clone());

                self.api_functions_visited[chosen_fun_index] = true;

                if new_sequence.len() >= max_len {
                    //println!("api_functions {}", new_sequence.len());
                    seq_num += 1;
                    if seq_num > 50 {
                        break;
                    }
                }
            }
        }
        //****************************************************************************** */
        println!("sequences len is {}", self.api_sequences.len());
    }

    pub(crate) fn my_method(
        &mut self,
        lib_name: &str,
        max_num: usize,
        max_len: usize,
        support_generic: bool,
    ) {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let mut depinfo = FxHashMap::default();
        let mut orderinfo = FxHashMap::default();
        let mut funcinfo = FxHashMap::default();

        self.reset_visited();

        //依赖信息
        {
            let depinfo_file_path =
                format!("/home/yxz/workspace/fuzz/experiment_root/{}/depinfo.txt", lib_name);
            match File::open(depinfo_file_path) {
                Ok(file) => {
                    let reader = BufReader::new(file);
                    for line in reader.lines() {
                        let parts = line
                            .unwrap()
                            .split("  |  ")
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect_vec();
                        let cnt = parts[1].parse::<usize>().unwrap();
                        let (func1, func2) = parts[2]
                            .split("   ")
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect_tuple()
                            .unwrap();

                        //把 (func2, cnt) 存入 func1对应的后续列表中

                        if depinfo.contains_key(&func1) {
                            let inner_map: &mut FxHashMap<String, usize> =
                                depinfo.get_mut(&func1).unwrap();
                            inner_map
                                .entry(func2.clone())
                                .and_modify(|value| *value += cnt)
                                .or_insert(cnt);
                        } else {
                            let mut inner_map = FxHashMap::default();
                            inner_map.insert(func2.clone(), cnt);
                            depinfo.insert(func1.clone(), inner_map);
                        }
                    }
                }
                _ => {}
            }
        }

        //解析顺序信息
        {
            let orderinfo_file_path =
                format!("/home/yxz/workspace/fuzz/experiment_root/{}/orderinfo.txt", lib_name);
            match File::open(orderinfo_file_path) {
                Ok(file) => {
                    let reader = BufReader::new(file);
                    for line in reader.lines() {
                        let parts = line
                            .unwrap()
                            .split("  |  ")
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect_vec();
                        let cnt = parts[1].parse::<usize>().unwrap();
                        let (func1, func2) = parts[2]
                            .split("   ")
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect_tuple()
                            .unwrap();

                        //把 (func2, cnt) 存入 func1对应的后续列表中

                        if orderinfo.contains_key(&func1) {
                            let inner_map: &mut FxHashMap<String, usize> =
                                orderinfo.get_mut(&func1).unwrap();
                            inner_map
                                .entry(func2.clone())
                                .and_modify(|value| *value += cnt)
                                .or_insert(cnt);
                        } else {
                            let mut inner_map = FxHashMap::default();
                            inner_map.insert(func2.clone(), cnt);
                            orderinfo.insert(func1.clone(), inner_map);
                        }
                    }
                }
                _ => {}
            }
        }

        //解析函数频率信息（暂时没用）
        {
            let funcinfo_file_path =
                format!("/home/yxz/workspace/fuzz/experiment_root/{}/funcinfo.txt", lib_name);
            match File::open(funcinfo_file_path) {
                Ok(file) => {
                    let reader = BufReader::new(file);
                    for line in reader.lines() {
                        let parts = line
                            .unwrap()
                            .split("  |  ")
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect_vec();
                        let cnt = parts[1].parse::<usize>().unwrap();
                        let func = &parts[2].clone();

                        //把 (func, cnt)存入funcindo中

                        funcinfo
                            .entry(func.clone())
                            .and_modify(|value| *value += cnt)
                            .or_insert(cnt);
                    }
                }
                _ => {}
            }
        }

        let print = true;
        //打印各种信息
        if print {
            println!("打印依赖信息！");
            //打印依赖频率信息
            for (func_name, inner_map) in &depinfo {
                print!("Func : [{}]-> \n\t", func_name);
                for (succ_name, cnt) in inner_map {
                    print!("[{}];{}  ", succ_name, cnt);
                }
                println!("\n");
            }
            println!("");

            //打印依赖频率信息
            for (func_name, inner_map) in &orderinfo {
                print!("顺序信息 Func : [{}]-> \n\t", func_name);
                for (succ_name, cnt) in inner_map {
                    print!("[{}];{}  ", succ_name, cnt);
                }
                println!("\n");
            }
            println!("");

            println!("打印函数频率信息！共{}个函数", funcinfo.len());
            //打印函数频率信息
            for (func_name, cnt) in &funcinfo {
                print!("[{}];[cnt={}]\t", func_name, cnt);
            }
            println!("");

            println!("打印所有被解析出来的函数");
            for (index, func) in self.api_functions.iter().enumerate() {
                //println!("{} {}", index, func._pretty_print(self.cache, &self.full_name_map));
                println!("function{}: {}", index, func.full_name);
            }
            println!("");
        }

        let mut covered_function = FxHashSet::default();

        let _function_succ_tables_map = construct_function_succ_table(self, &depinfo);
        let mut _function_succ_tables_map_weighted = FxHashMap::default();
        //预计算权重
        {
            for (pre_function_index, selected_succ_table) in &_function_succ_tables_map {
                //计算权重
                let weights = selected_succ_table
                    .iter()
                    .map(|(succ, freq, _)| {
                        let order_pair_occur_freq = if let Some(inner_map) =
                            orderinfo.get(&self.api_functions[*pre_function_index].full_name)
                        {
                            if let Some(frequence) =
                                inner_map.get(&self.api_functions[*succ].full_name)
                            {
                                //println!("Yes, frequence{}", *frequence);
                                *frequence
                            } else {
                                0
                            }
                        } else {
                            0
                        };

                        //println!("{}, {}", order_pair_occur_freq, *freq);
                        order_pair_occur_freq + *freq
                    })
                    .collect_vec();
                //println!("{}", weights.len());
                //归一化
                let normalized_weights = normalize_weights(&weights);
                //println!("{:?}", normalized_weights);
                /*println!(
                    "Func: {}, normalized: {:?}",
                    &self.api_functions[*pre_function_index].full_name, normalized_weights
                );*/
                let mut weighted_succ_table = Vec::new();

                for (index, (succ, _, _)) in selected_succ_table.iter().enumerate() {
                    weighted_succ_table.push((*succ, normalized_weights[index]));
                }
                /*
                println!(
                    "生产者：{}, {:#?}",
                    self.api_functions[*pre_function_index].full_name, weighted_succ_table
                );*/

                _function_succ_tables_map_weighted.insert(*pre_function_index, weighted_succ_table);
            }
        }

        let _start_functions = extract_start_function(self, support_generic);
        println!("这里有{}个start function", _start_functions.len());
        for start in &_start_functions {
            println!(
                "Start function: {}",
                self.api_functions[*start]._pretty_print(self.cache, &self.full_name_map)
            );
        }

        let mut sequences: Vec<ApiSequence> = Vec::new();

        let mut start_index_polling = 0;

        //FIXME: 在这里编写逻辑
        loop {
            if sequences.len() >= max_num {
                break;
            }

            //初始化当前sequence
            let mut sequence = ApiSequence::new();
            let mut indexs_in_sequence: Vec<usize> = Vec::new();

            let mut need_new: bool = false;

            //获得随机start函数在全局的index
            //let start_idx = _start_functions[rand_num(0, _start_functions.len())];
            start_index_polling += 1;
            if start_index_polling >= _start_functions.len() {
                start_index_polling = 0;
            }

            let start_function_index = _start_functions[start_index_polling];

            sequence = match self.is_fun_satisfied(
                &ApiType::BareFunction,
                start_function_index,
                &sequence,
            ) {
                Some(seq) => {
                    /*println!(
                        "select start function {}",
                        self.api_functions[start_function_index].full_name
                    );*/
                    indexs_in_sequence.push(start_function_index);
                    covered_function.insert(start_function_index);
                    seq
                }
                None => continue,
            };

            loop {
                if sequence.len() >= max_len {
                    break;
                }
                let rand = rand_num(0, sequence.len() + 3);
                //println!("rand = {}", rand);
                if rand == 0 || need_new {
                    //有1/(len+5)的概率接触到new
                    //获得随机start函数在全局的index
                    //println!("选择start");
                    assert!(_start_functions.len() > 0);
                    let start_idx = _start_functions[rand_num(0, _start_functions.len())];
                    sequence =
                        match self.is_fun_satisfied(&ApiType::BareFunction, start_idx, &sequence) {
                            Some(seq) => {
                                indexs_in_sequence.push(start_idx);
                                covered_function.insert(start_idx);
                                //加入了new，就可以了
                                need_new = false;
                                seq
                            }
                            None => continue,
                        };
                } else {
                    //还有 1-l/(1en+3)选到其他

                    let available_function_indexs = _get_available_function_indexs(
                        self,
                        &sequence,
                        &_function_succ_tables_map_weighted,
                    );
                    if available_function_indexs.is_empty() {
                        //可获得的没了，说明需要start函数了
                        need_new = true;
                        continue;
                    }

                    //随机找到序列中一个可获得的返回值
                    let selected_function_index =
                        available_function_indexs[rand_num(0, available_function_indexs.len())];
                    /*println!(
                        "长度为{}, 被选择的生产者是{}",
                        available_function_indexs.len(),
                        &self.api_functions[selected_function_index].full_name
                    );*/

                    /*
                    //找到后继表
                    let selected_succ_table =
                        _function_succ_tables_map.get(&selected_function_index).unwrap();

                    //计算权重
                    let weights = selected_succ_table
                        .iter()
                        .map(|(succ, freq, _)| {
                            let order_pair_occur_freq = if let Some(inner_map) = orderinfo
                                .get(&self.api_functions[selected_function_index].full_name)
                            {
                                if let Some(frequence) =
                                    inner_map.get(&self.api_functions[*succ].full_name)
                                {
                                    println!("Yes, frequence{}", *frequence);
                                    *frequence
                                } else {
                                    0
                                }
                            } else {
                                0
                            };
                            order_pair_occur_freq + *freq
                        })
                        .collect_vec();
                    let normalized_weights = normalize_weights(&weights);*/

                    let selected_succ_table_weighted =
                        _function_succ_tables_map_weighted.get(&selected_function_index).unwrap();

                    let normalized_weights =
                        selected_succ_table_weighted.iter().map(|(_, w)| *w).collect_vec();

                    let mut succ_index = 0;

                    for _ in 0..3 {
                        //let select_immutable = _select_immutable_or_not(sequence.len(), max_len);
                        match _random_select(&normalized_weights) {
                            Some(i) => {
                                //先存着
                                succ_index = i;
                                if covered_function
                                    .contains(&selected_succ_table_weighted[succ_index].0)
                                {
                                    continue;
                                }
                            }
                            None => succ_index = 0,
                        };
                        break;
                    }
                    //let succ_index = 0;
                    //println!("succ_index = {:?}", succ_index);
                    let (succ_api_function_index, _) = &selected_succ_table_weighted[succ_index];
                    //println!("succ_api_func_index = {}", succ_api_function_index);
                    sequence = match self.is_fun_satisfied(
                        &ApiType::BareFunction,
                        *succ_api_function_index,
                        &sequence,
                    ) {
                        Some(seq) => {
                            indexs_in_sequence.push(*succ_api_function_index);
                            covered_function.insert(*succ_api_function_index);
                            seq
                        }
                        None => continue,
                    };
                }
            }

            sequences.push(sequence);
        }

        for index in covered_function.iter() {
            self.api_functions_visited[*index] = true;
        }

        //最后赋值给graph.api_sequences
        self.api_sequences = sequences;

        println!(
            "覆盖的API数量: {}, API覆盖率: {}",
            covered_function.len(),
            (covered_function.len() as f32) / (self.api_functions.len() as f32)
        );

        /// 归一化函数
        /// 导致每个差距都在20以内
        #[allow(dead_code)]
        fn normalize_weights(weights: &Vec<usize>) -> Vec<f32> {
            if weights.len() == 0 {
                return Vec::new();
            }
            // 使用对数函数进行调整
            let mut weights = weights.iter().map(|x| *x as f32).collect_vec();

            for weight in weights.iter_mut() {
                *weight = (*weight + 2.0).ln();
            }

            // 计算归一化前的最大值和最小值
            let max_value = *weights.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
            let min_value = *weights.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

            // 缩放权重，使其范围在0.05到0.95之间
            for weight in weights.iter_mut() {
                *weight = 0.05 + ((*weight - min_value) * 0.9) / (1.0 + max_value - min_value);
            }
            //println!("归一化后的权重：{:?}", weights);
            weights
        }

        /// 构造函数后继表
        fn construct_function_succ_table(
            graph: &mut ApiGraph<'_>,
            depinfo: &FxHashMap<String, FxHashMap<String, usize>>,
        ) -> FxHashMap<usize, Vec<(usize, usize, ApiDependency)>> {
            let mut function_succ_tables = FxHashMap::default();
            //对每个API初始化它们的表
            for i in 0..graph.api_functions.len() {
                function_succ_tables.insert(i, Vec::new());
            }

            for api_dependency in &graph.api_dependencies {
                let output_func_index = api_dependency.output_fun.1;
                let input_func_index = api_dependency.input_fun.1;

                let output_func_name = &graph.api_functions[output_func_index].full_name;
                let input_func_name = &graph.api_functions[input_func_index].full_name;

                //println!("生产者{} 消费者{}", output_func_name, input_func_name);

                //因为前面初始化了，所以unwrap肯定没问题
                let output_func_table = function_succ_tables.get_mut(&output_func_index).unwrap();

                if depinfo.contains_key(output_func_name) {
                    let inner_map = depinfo.get(output_func_name).unwrap();
                    if inner_map.contains_key(input_func_name) {
                        let freq = inner_map.get(input_func_name).unwrap();
                        output_func_table.push((input_func_index, *freq, api_dependency.clone()));
                    } else {
                        output_func_table.push((input_func_index, 0, api_dependency.clone()));
                    }
                } else {
                    output_func_table.push((input_func_index, 0, api_dependency.clone()));
                }
            }
            /*
            for (output, table) in &function_succ_tables {
                for (input, freq, _) in table {
                    let output_func_name = &graph.api_functions[*output].full_name;
                    let input_func_name = &graph.api_functions[*input].full_name;
                    println!(
                        "[first function: {} {}], [second function: {} {}], [freq: {}]",
                        output, output_func_name, input, input_func_name, freq
                    );
                }
            }*/

            function_succ_tables
        }

        fn extract_start_function(api_graph: &ApiGraph<'_>, support_generic: bool) -> Vec<usize> {
            let mut res = Vec::new();
            for (index, api_function) in api_graph.api_functions.iter().enumerate() {
                if api_function._is_start_function(
                    api_graph.cache,
                    &api_graph.full_name_map,
                    support_generic,
                ) {
                    /*println!(
                        "Start func: {}",
                        api_function._pretty_print(api_graph.cache, &api_graph.full_name_map)
                    );*/
                    res.push(index);
                }
            }
            //println!("Start function有{}个", res.len());

            res
        }

        /// [min, max)
        fn rand_num(min: usize, max: usize) -> usize {
            if min >= max {
                return 0;
            }
            let mut rng = rand::thread_rng();
            let random_number = rng.gen_range(min, max);

            //println!("随机数: {}", random_number);
            random_number
        }

        fn _get_succ_indexs(
            //api_graph: &ApiGraph<'_>,
            sequence: &ApiSequence,
            indexs_in_sequence: &Vec<usize>,
            _function_succ_tables_map: &FxHashMap<usize, Vec<(usize, usize, ApiDependency)>>,
        ) -> Vec<usize> {
            let mut res = FxHashSet::default();
            let mut filtered_indexs = Vec::new();
            for (apicall_idx, func_index) in indexs_in_sequence.iter().enumerate() {
                if !sequence._is_moved(apicall_idx) {
                    filtered_indexs.push(*func_index);
                }
            }

            for index in filtered_indexs {
                let table = _function_succ_tables_map.get(&index).unwrap(); //这里不会出现问题
                for (succ, _, _) in table {
                    res.insert(*succ);
                }
            }
            res.into_iter().collect_vec()
        }

        //获得sequence里面返回值没有被move的函数的api_function index
        fn _get_available_function_indexs(
            _api_graph: &ApiGraph<'_>,
            sequence: &ApiSequence,
            succ_tables: &FxHashMap<usize, Vec<(usize, f32)>>,
        ) -> Vec<usize> {
            let mut res = Vec::new();
            //println!();
            for (api_call_index, api) in sequence.functions.iter().enumerate() {
                let api_function_index = api.func.1;

                /*println!(
                    "前面的funcname: {}, ",
                    _api_graph.api_functions[api_call_index].full_name
                );*/
                //如果没有被move，并且有output
                if succ_tables.get(&api_function_index).unwrap().len() > 0
                    && !sequence._is_moved(api_call_index)
                {
                    /*println!(
                        "被选择的前面的funcname: {}, ",
                        _api_graph.api_functions[api_function_index].full_name
                    );*/
                    res.push(api_function_index);
                }
            }
            return res;
        }

        fn _random_select(probabilities: &Vec<f32>) -> Option<usize> {
            use rand::prelude::SliceRandom;
            let mut rng = thread_rng();
            let weighted_indices: Vec<usize> = (0..probabilities.len()).collect();
            let dist = weighted_indices.choose_weighted(&mut rng, |&i| probabilities[i]).ok()?;
            Some(*dist)
        }
        fn _select_immutable_or_not(i: usize, max_len: usize) -> bool {
            //选择不可变引用，随着1/4概率增大到5/8
            let p_of_immutable_ref = ((max_len / 6 + i) as f32) / ((i + max_len) as f32);
            let rand_from_0_to_99 = rand_num(0, 100);

            rand_from_0_to_99 < (p_of_immutable_ref as usize) * 100
        }
    }

    pub(crate) fn _choose_candidate_sequence_for_merge(&self) -> Vec<usize> {
        let mut res = Vec::new();
        let all_sequence_number = self.api_sequences.len();
        for i in 0..all_sequence_number {
            let api_sequence = &self.api_sequences[i];
            let dead_code = api_sequence._dead_code(self);
            let api_sequence_len = api_sequence.len();
            if self.is_sequence_ended(api_sequence, false) {
                //如果当前序列已经结束
                continue;
            }
            if api_sequence_len <= 0 {
                continue;
            } else if api_sequence_len == 1 {
                res.push(i);
            } else {
                let mut dead_code_flag = false;
                for j in 0..api_sequence_len - 1 {
                    if dead_code[j] {
                        dead_code_flag = true;
                        break;
                    }
                }
                if !dead_code_flag {
                    res.push(i);
                }
            }
        }
        res
    }

    pub(crate) fn _try_to_cover_unvisited_nodes(&mut self) {
        //println!("try to cover more nodes");
        let mut apis_covered_by_reverse_search = 0;
        let mut unvisited_nodes = FxHashSet::default();
        let api_fun_number = self.api_functions.len();
        for i in 0..api_fun_number {
            if !self.api_functions_visited[i] {
                unvisited_nodes.insert(i);
            }
        }
        let mut covered_node_this_iteration = FxHashSet::default();
        //最多循环没访问到的节点的数量
        for _ in 0..unvisited_nodes.len() {
            covered_node_this_iteration.clear();
            let candidate_sequences = self._choose_candidate_sequence_for_merge();
            //println!("sequence number, {}", self.api_sequences.len());
            //println!("candidate sequence number, {}", candidate_sequences.len());
            for unvisited_node in &unvisited_nodes {
                let unvisited_api_func = &self.api_functions[*unvisited_node];
                let inputs = &unvisited_api_func.inputs;
                let mut dependent_sequence_indexes = Vec::new();
                let mut can_be_covered_flag = true;
                let input_param_num = inputs.len();
                for i in 0..input_param_num {
                    let input_type = &inputs[i];
                    if api_util::is_fuzzable_type(input_type, self.cache, &self.full_name_map, None)
                    {
                        continue;
                    }
                    let mut can_find_dependency_flag = false;
                    let mut tmp_dependent_index = -1;
                    for candidate_sequence_index in &candidate_sequences {
                        let output_type = ApiType::BareFunction;
                        let input_type = ApiType::BareFunction;
                        let candidate_sequence = &self.api_sequences[*candidate_sequence_index];
                        let output_index = candidate_sequence._last_api_func_index().unwrap();

                        if let Some(_) = self.check_dependency(
                            &output_type,
                            output_index,
                            &input_type,
                            *unvisited_node,
                            i,
                        ) {
                            can_find_dependency_flag = true;
                            //dependent_sequence_indexes.push(*candidate_sequence_index);
                            tmp_dependent_index = *candidate_sequence_index as i32;

                            //prefer sequence with fuzzable inputs
                            if !candidate_sequence._has_no_fuzzables() {
                                break;
                            }
                        }
                    }
                    if !can_find_dependency_flag {
                        can_be_covered_flag = false;
                    } else {
                        dependent_sequence_indexes.push(tmp_dependent_index as usize);
                    }
                }
                if can_be_covered_flag {
                    //println!("{:?} can be covered", unvisited_api_func.full_name);
                    let dependent_sequences: Vec<ApiSequence> = dependent_sequence_indexes
                        .into_iter()
                        .map(|index| self.api_sequences[index].clone())
                        .collect();
                    let merged_sequence = ApiSequence::_merge_sequences(&dependent_sequences);
                    let input_type = ApiType::BareFunction;
                    if let Some(generated_sequence) =
                        self.is_fun_satisfied(&input_type, *unvisited_node, &merged_sequence)
                    {
                        //println!("{}", generated_sequence._to_well_written_function(self, 0, 0));

                        self.api_sequences.push(generated_sequence);
                        self.api_functions_visited[*unvisited_node] = true;
                        covered_node_this_iteration.insert(*unvisited_node);
                        apis_covered_by_reverse_search = apis_covered_by_reverse_search + 1;
                    } else {
                        //The possible cause is there is some wrong fuzzable type
                        println!("Should not go to here. Only if algorithm error occurs");
                    }
                }
            }
            if covered_node_this_iteration.len() == 0 {
                println!("reverse search can not cover more nodes");
                break;
            } else {
                for covered_node in &covered_node_this_iteration {
                    unvisited_nodes.remove(covered_node);
                }
            }
        }

        let mut totol_sequences_number = 0;
        let mut total_length = 0;
        let mut covered_nodes = FxHashSet::default();
        let mut covered_edges = FxHashSet::default();

        for sequence in &self.api_sequences {
            if sequence._has_no_fuzzables() {
                continue;
            }
            totol_sequences_number = totol_sequences_number + 1;
            total_length = total_length + sequence.len();
            let cover_nodes = sequence._get_contained_api_functions();
            for cover_node in &cover_nodes {
                covered_nodes.insert(*cover_node);
            }

            let cover_edges = &sequence._covered_dependencies;
            for cover_edge in cover_edges {
                covered_edges.insert(*cover_edge);
            }
        }

        println!("after backward search");
        println!("targets = {}", totol_sequences_number);
        println!("total length = {}", total_length);
        let average_visit_time = (total_length as f64) / (covered_nodes.len() as f64);
        println!("average time to visit = {}", average_visit_time);
        println!("edge covered by reverse search = {}", covered_edges.len());

        //println!("There are total {} APIs covered by reverse search", apis_covered_by_reverse_search);
    }

    pub(crate) fn _naive_choose_sequence(&self, max_sequence_size: usize) -> Vec<ApiSequence> {
        let mut to_cover_nodes = Vec::new();
        let function_len = self.api_functions.len();
        for i in 0..function_len {
            if self.api_functions_visited[i] {
                to_cover_nodes.push(i);
            }
        }
        let to_cover_nodes_number = to_cover_nodes.len();
        println!("There are total {} nodes need to be covered.", to_cover_nodes_number);

        let mut chosen_sequence_flag = Vec::new();
        let prepared_sequence_number = self.api_sequences.len();
        for _ in 0..prepared_sequence_number {
            chosen_sequence_flag.push(false);
        }

        let mut res = Vec::new();
        let mut node_candidate_sequences = FxHashMap::default();

        for node in &to_cover_nodes {
            node_candidate_sequences.insert(*node, Vec::new());
        }

        for i in 0..prepared_sequence_number {
            let api_sequence = &self.api_sequences[i];
            let contains_nodes = api_sequence._get_contained_api_functions();
            for node in contains_nodes {
                if let Some(v) = node_candidate_sequences.get_mut(&node) {
                    if !v.contains(&i) {
                        v.push(i);
                    }
                }
            }
        }

        let mut rng = thread_rng();
        for _ in 0..max_sequence_size {
            if to_cover_nodes.len() == 0 {
                println!("all {} nodes need to be covered is covered", to_cover_nodes_number);
                break;
            }
            //println!("need_to_cover_nodes:{:?}", to_cover_nodes);
            let next_cover_node = to_cover_nodes.first().unwrap();
            let candidate_sequences =
                node_candidate_sequences.get(next_cover_node).unwrap().clone();
            let unvisited_candidate_sequences = candidate_sequences
                .into_iter()
                .filter(|node| chosen_sequence_flag[*node] == false)
                .collect::<Vec<_>>();
            let candidate_number = unvisited_candidate_sequences.len();
            let random_index = rng.gen_range(0, candidate_number);
            let chosen_index = unvisited_candidate_sequences[random_index];
            //println!("randomc index{}", random_index);
            let chosen_sequence = &self.api_sequences[chosen_index];
            //println!("{:}",chosen_sequence._to_well_written_function(self, 0, 0));

            let covered_nodes = chosen_sequence._get_contained_api_functions();
            to_cover_nodes =
                to_cover_nodes.into_iter().filter(|node| !covered_nodes.contains(node)).collect();
            chosen_sequence_flag[random_index] = true;
            res.push(chosen_sequence.clone());
        }
        res
    }

    pub(crate) fn _random_choose(&self, max_size: usize) -> Vec<ApiSequence> {
        let mut res = Vec::new();
        let mut covered_nodes = FxHashSet::default();
        let mut covered_edges = FxHashSet::default();
        let mut sequence_indexes = Vec::new();

        let total_sequence_size = self.api_sequences.len();

        for i in 0..total_sequence_size {
            sequence_indexes.push(i);
        }

        let mut rng = thread_rng();
        for _ in 0..max_size {
            let rest_sequences_number = sequence_indexes.len();
            if rest_sequences_number <= 0 {
                break;
            }

            let chosen_index = rng.gen_range(0, rest_sequences_number);
            let sequence_index = sequence_indexes[chosen_index];

            let sequence = &self.api_sequences[sequence_index];
            res.push(sequence.clone());
            sequence_indexes.remove(chosen_index);

            for covered_node in sequence._get_contained_api_functions() {
                covered_nodes.insert(covered_node);
            }

            for covered_edge in &sequence._covered_dependencies {
                covered_edges.insert(covered_edge.clone());
            }
        }

        println!("-----------STATISTICS-----------");
        println!("Random selection selected {} targets", res.len());
        println!("Random selection covered {} nodes", covered_nodes.len());
        println!("Random selection covered {} edges", covered_edges.len());
        println!("--------------------------------");

        res
    }

    pub(crate) fn _first_choose(&self, max_size: usize, max_len: usize) -> Vec<ApiSequence> {
        let mut res = Vec::new();
        let mut already_covered_nodes = FxHashSet::default();
        let mut already_covered_edges = FxHashSet::default();

        let total_sequence_size = self.api_sequences.len();

        for index in 0..total_sequence_size {
            let sequence = &self.api_sequences[index];
            if sequence._has_no_fuzzables() {
                continue;
            }
            if sequence.len() < max_len {
                continue;
            }

            res.push(sequence.clone());

            let covered_nodes = sequence._get_contained_api_functions();
            for cover_node in covered_nodes {
                already_covered_nodes.insert(cover_node);
            }
            let covered_edges = &sequence._covered_dependencies;
            //println!("covered_edges = {:?}", covered_edges);
            for cover_edge in covered_edges {
                already_covered_edges.insert(*cover_edge);
            }

            if res.len() >= max_size {
                break;
            }
        }

        let mut valid_api_number = 0;
        for api_function_ in &self.api_functions {
            if !api_function_.contains_unsupported_fuzzable_type(self.cache, &self.full_name_map) {
                valid_api_number = valid_api_number + 1;
            }
        }

        let binding = already_covered_nodes.clone();
        let mut acn = binding.iter().collect_vec();
        acn.sort();
        for idx in &already_covered_nodes {
            println!(
                "covered function: {}",
                self.api_functions[*idx]._pretty_print(self.cache, &self.full_name_map)
            );
        }

        println!("-----------STATISTICS-----------");
        let total_functions_number = self.api_functions.len();
        println!("total nodes: {}", total_functions_number);

        let total_dependencies_number = self.api_dependencies.len();
        println!("total edges: {}", total_dependencies_number);

        let covered_node_num = already_covered_nodes.len();
        let covered_edges_num = already_covered_edges.len();
        println!("covered nodes: {}", covered_node_num);
        println!("covered edges: {}", covered_edges_num);

        let node_coverage = (already_covered_nodes.len() as f64) / (valid_api_number as f64);
        let edge_coverage =
            (already_covered_edges.len() as f64) / (total_dependencies_number as f64);
        println!("node coverage: {}", node_coverage);
        println!("edge coverage: {}", edge_coverage);
        println!("--------------------------------");

        res
    }

    pub(crate) fn _heuristic_choose(
        &self,
        max_size: usize,
        stop_at_visit_all_nodes: bool,
    ) -> Vec<ApiSequence> {
        let mut res = Vec::new();
        let mut to_cover_nodes = Vec::new();

        let mut fixed_covered_nodes = FxHashSet::default();

        let mut api_sequences = self.api_sequences.clone();
        api_sequences.reverse();
        for fixed_sequence in &api_sequences {
            //let covered_nodes = fixed_sequence._get_contained_api_functions();
            //for covered_node in &covered_nodes {
            //    fixed_covered_nodes.insert(*covered_node);
            //}

            if !fixed_sequence._has_no_fuzzables()
                && !fixed_sequence._contains_dead_code_except_last_one(self)
            {
                let covered_nodes = fixed_sequence._get_contained_api_functions();
                for covered_node in &covered_nodes {
                    fixed_covered_nodes.insert(*covered_node);
                }
            }
        }

        for fixed_covered_node in fixed_covered_nodes {
            to_cover_nodes.push(fixed_covered_node);
        }

        let to_cover_nodes_number = to_cover_nodes.len();
        //println!("There are total {} nodes need to be covered.", to_cover_nodes_number);
        let to_cover_dependency_number = self.api_dependencies.len();
        //println!("There are total {} edges need to be covered.", to_cover_dependency_number);
        let total_sequence_number = self.api_sequences.len();

        //println!("There are toatl {} sequences.", total_sequence_number);
        let mut valid_fuzz_sequence_count = 0;
        for sequence in &self.api_sequences {
            if !sequence._has_no_fuzzables() && !sequence._contains_dead_code_except_last_one(self)
            {
                valid_fuzz_sequence_count = valid_fuzz_sequence_count + 1;
            }
        }
        //println!("There are toatl {} valid sequences for fuzz.", valid_fuzz_sequence_count);
        if valid_fuzz_sequence_count <= 0 {
            return res;
        }

        let mut already_covered_nodes = FxHashSet::default();
        let mut already_covered_edges = FxHashSet::default();
        let mut already_chosen_sequences = FxHashSet::default();
        let mut sorted_chosen_sequences = Vec::new();
        let mut dynamic_fuzzable_length_sequences_count = 0;
        let mut fixed_fuzzale_length_sequences_count = 0;

        let mut try_to_find_dynamic_length_flag = true;
        for _ in 0..max_size + 1 {
            let mut current_chosen_sequence_index = 0;
            let mut current_max_covered_nodes = 0;
            let mut current_max_covered_edges = 0;
            let mut current_chosen_sequence_len = 0;

            for j in 0..total_sequence_number {
                if already_chosen_sequences.contains(&j) {
                    continue;
                }
                let api_sequence = &self.api_sequences[j];

                if api_sequence._has_no_fuzzables()
                    || api_sequence._contains_dead_code_except_last_one(self)
                {
                    continue;
                }

                if try_to_find_dynamic_length_flag && api_sequence._is_fuzzables_fixed_length() {
                    //优先寻找fuzzable部分具有动态长度的情况
                    continue;
                }

                if !try_to_find_dynamic_length_flag && !api_sequence._is_fuzzables_fixed_length() {
                    //再寻找fuzzable部分具有静态长度的情况
                    continue;
                }

                let covered_nodes = api_sequence._get_contained_api_functions();
                let mut uncovered_nodes_by_former_sequence_count = 0;
                for covered_node in &covered_nodes {
                    if !already_covered_nodes.contains(covered_node) {
                        uncovered_nodes_by_former_sequence_count =
                            uncovered_nodes_by_former_sequence_count + 1;
                    }
                }

                if uncovered_nodes_by_former_sequence_count < current_max_covered_nodes {
                    continue;
                }
                let covered_edges = &api_sequence._covered_dependencies;
                let mut uncovered_edges_by_former_sequence_count = 0;
                for covered_edge in covered_edges {
                    if !already_covered_edges.contains(covered_edge) {
                        uncovered_edges_by_former_sequence_count =
                            uncovered_edges_by_former_sequence_count + 1;
                    }
                }
                if uncovered_nodes_by_former_sequence_count == current_max_covered_nodes
                    && uncovered_edges_by_former_sequence_count < current_max_covered_edges
                {
                    continue;
                }
                let sequence_len = api_sequence.len();
                if (uncovered_nodes_by_former_sequence_count > current_max_covered_nodes)
                    || (uncovered_nodes_by_former_sequence_count == current_max_covered_nodes
                        && uncovered_edges_by_former_sequence_count > current_max_covered_edges)
                    || (uncovered_nodes_by_former_sequence_count == current_max_covered_nodes
                        && uncovered_edges_by_former_sequence_count == current_max_covered_edges
                        && sequence_len < current_chosen_sequence_len)
                {
                    current_chosen_sequence_index = j;
                    current_max_covered_nodes = uncovered_nodes_by_former_sequence_count;
                    current_max_covered_edges = uncovered_edges_by_former_sequence_count;
                    current_chosen_sequence_len = sequence_len;
                }
            }

            if try_to_find_dynamic_length_flag && current_max_covered_nodes <= 0 {
                //println!("sequences with dynamic length can not cover more nodes");
                try_to_find_dynamic_length_flag = false;
                continue;
            }

            if !try_to_find_dynamic_length_flag
                && current_max_covered_edges <= 0
                && current_max_covered_nodes <= 0
            {
                //println!("can't cover more edges or nodes");
                break;
            }
            already_chosen_sequences.insert(current_chosen_sequence_index);
            sorted_chosen_sequences.push(current_chosen_sequence_index);

            if try_to_find_dynamic_length_flag {
                dynamic_fuzzable_length_sequences_count =
                    dynamic_fuzzable_length_sequences_count + 1;
            } else {
                fixed_fuzzale_length_sequences_count = fixed_fuzzale_length_sequences_count + 1;
            }

            let chosen_sequence = &self.api_sequences[current_chosen_sequence_index];

            let covered_nodes = chosen_sequence._get_contained_api_functions();
            for cover_node in covered_nodes {
                already_covered_nodes.insert(cover_node);
            }
            let covered_edges = &chosen_sequence._covered_dependencies;
            //println!("covered_edges = {:?}", covered_edges);
            for cover_edge in covered_edges {
                already_covered_edges.insert(*cover_edge);
            }

            if already_chosen_sequences.len() == valid_fuzz_sequence_count {
                //println!("all sequence visited");
                break;
            }
            if to_cover_dependency_number != 0
                && already_covered_edges.len() == to_cover_dependency_number
            {
                //println!("all edges visited");
                //should we stop at visit all edges?
                break;
            }
            if stop_at_visit_all_nodes && already_covered_nodes.len() == to_cover_nodes_number {
                //println!("all nodes visited");
                break;
            }
            //println!("no fuzzable count = {}", no_fuzzable_count);
        }

        let total_functions_number = self.api_functions.len();
        println!("-----------STATISTICS-----------");
        println!("total nodes: {}", total_functions_number);

        let mut valid_api_number = 0;
        for api_function_ in &self.api_functions {
            if !api_function_.contains_unsupported_fuzzable_type(self.cache, &self.full_name_map) {
                valid_api_number = valid_api_number + 1;
            }
            //else {
            //    println!("{}", api_function_._pretty_print(&self.full_name_map));
            //}
        }
        //println!("total valid nodes: {}", valid_api_number);

        let total_dependencies_number = self.api_dependencies.len();
        println!("total edges: {}", total_dependencies_number);

        let covered_node_num = already_covered_nodes.len();
        let covered_edges_num = already_covered_edges.len();
        println!("covered nodes: {}", covered_node_num);
        println!("covered edges: {}", covered_edges_num);

        let node_coverage = (already_covered_nodes.len() as f64) / (valid_api_number as f64);
        let edge_coverage =
            (already_covered_edges.len() as f64) / (total_dependencies_number as f64);
        println!("node coverage: {}", node_coverage);
        println!("edge coverage: {}", edge_coverage);
        //println!("sequence with dynamic fuzzable length: {}", dynamic_fuzzable_length_sequences_count);
        //println!("sequence with fixed fuzzable length: {}",fixed_fuzzale_length_sequences_count);

        let mut sequnce_covered_by_reverse_search = 0;
        let mut max_length = 0;
        for sequence_index in sorted_chosen_sequences {
            let api_sequence = self.api_sequences[sequence_index].clone();

            if api_sequence.len() > 3 {
                sequnce_covered_by_reverse_search = sequnce_covered_by_reverse_search + 1;
                if api_sequence.len() > max_length {
                    max_length = api_sequence.len();
                }
            }

            res.push(api_sequence);
        }

        println!("targets covered by reverse search: {}", sequnce_covered_by_reverse_search);
        println!("total targets: {}", res.len());
        println!("max length = {}", max_length);

        let mut total_length = 0;
        for selected_sequence in &res {
            total_length = total_length + selected_sequence.len();
        }

        println!("total length = {}", total_length);
        let average_time_to_fuzz_each_api =
            (total_length as f64) / (already_covered_nodes.len() as f64);
        println!("average time to fuzz each api = {}", average_time_to_fuzz_each_api);

        println!("--------------------------------");

        res
    }

    //OK: 判断一个函数能否加入给定的序列中,如果可以加入，返回Some(new_sequence),new_sequence是将新的调用加进去之后的情况，否则返回None
    pub(crate) fn is_fun_satisfied(
        &self,
        input_fun_type: &ApiType, //其实这玩意没用了
        input_fun_index: usize,
        sequence: &ApiSequence,
    ) -> Option<ApiSequence> {
        use super::api_util::substitute_type;
        //判断一个给定的函数能否加入到一个sequence中去
        match input_fun_type {
            ApiType::BareFunction => {
                let mut new_sequence = sequence.clone();
                let mut api_call = ApiCall::_new(input_fun_index);

                let mut _moved_indexes = new_sequence._moved.clone(); //用来保存发生move的那些语句的index
                let mut _multi_mut = FxHashSet::default(); //用来保存会被多次可变引用的情况
                let mut _immutable_borrow = FxHashSet::default(); //不可变借用

                //下面是全局借用和可变借用标记
                //let mut global_mut_borrow = new_sequence._mut_borrow.clone();
                //let mut global_borrow = new_sequence._borrow.clone();

                //函数
                let input_function = &self.api_functions[input_fun_index];

                //如果是个unsafe函数，给sequence添加unsafe标记
                if input_function._unsafe_tag._is_unsafe() {
                    new_sequence.set_unsafe();
                }
                //如果用到了trait，添加到序列的trait列表
                if input_function._trait_full_path.is_some() {
                    let trait_full_path = input_function._trait_full_path.as_ref().unwrap();
                    new_sequence.add_trait(trait_full_path);
                }

                //看看之前序列的返回值是否可以作为它的参数
                let input_params = &input_function.inputs;
                if input_params.is_empty() {
                    //无需输入参数，直接是可满足的
                    new_sequence._add_fn(api_call);
                    return Some(new_sequence);
                }
                //对于每个参数进行遍历
                for (i, current_ty) in input_params.iter().enumerate() {
                    // 如果参数是fuzzable的话，...
                    // 在这里T会被替换成concrete type
                    let current_ty = &match substitute_type(
                        current_ty.clone(),
                        &input_function.generic_substitutions,
                    ) {
                        //FIXME:
                        Some(substi) => substi,
                        None => current_ty.clone(),
                    };

                    if api_util::is_fuzzable_type(
                        current_ty,
                        self.cache,
                        &self.full_name_map,
                        Some(&input_function.generic_substitutions),
                    ) {
                        /*
                        println!(
                            "param_{} in function {} is fuzzable type",
                            i, input_function.full_name
                        );*/
                        //如果当前参数是fuzzable的
                        let current_fuzzable_index = new_sequence.fuzzable_params.len();
                        let fuzzable_call_type = fuzz_type::fuzzable_call_type(
                            current_ty,
                            self.cache,
                            &self.full_name_map,
                            Some(&input_function.generic_substitutions),
                        );
                        let (fuzzable_type, call_type) =
                            fuzzable_call_type.generate_fuzzable_type_and_call_type();

                        //如果出现了下面这段话，说明出现了Fuzzable参数但不知道如何参数化的
                        //典型例子是tuple里面出现了引用（&usize），这种情况不再去寻找dependency，直接返回无法添加即可
                        match &fuzzable_type {
                            FuzzableType::NoFuzzable => {
                                //println!("Fuzzable Type Error Occurs!");
                                //println!("type = {:?}", current_ty);
                                //println!("fuzzable_call_type = {:?}", fuzzable_call_type);
                                //println!("fuzzable_type = {:?}", fuzzable_type);
                                return None;
                            }
                            _ => {}
                        }

                        //判断要不要加mut tag
                        if api_util::_need_mut_tag(&call_type) {
                            new_sequence._insert_fuzzable_mut_tag(current_fuzzable_index);
                        }

                        //添加到sequence中去
                        new_sequence.fuzzable_params.push(fuzzable_type);
                        api_call._add_param(
                            ParamType::_FuzzableType,
                            current_fuzzable_index,
                            call_type,
                        );
                    }
                    //如果参数不是fuzzable的话，也就是无法直接被afl转化，就需要看看有没有依赖关系
                    else {
                        // 如果当前参数不是fuzzable的，那么就去api sequence寻找是否有这个依赖
                        // 也就是说，api sequence里是否有某个api的返回值是它的参数

                        //FIXME: 处理move的情况
                        let functions_in_sequence_len = sequence.functions.len();
                        let mut dependency_flag = false;

                        for function_index in 0..functions_in_sequence_len {
                            //每次换个api，都会换掉

                            // 如果这个sequence里面的该函数返回值已经被move掉了，那么就跳过，不再能被使用了
                            // 后面的都是默认这个返回值没有被move，而是被可变借用或不可变借用
                            if _moved_indexes.contains(&function_index) {
                                continue;
                            }

                            //获取序列中对应函数的api_call
                            let found_function = &new_sequence.functions[function_index];
                            let (api_type, index) = &found_function.func;
                            let index = *index;
                            //如果有依赖关系，才有后面的说法
                            if let Some(dependency_index) = self.check_dependency(
                                api_type,
                                index,
                                input_fun_type,
                                input_fun_index,
                                i,
                            ) {
                                // 理论上这里泛型依赖也会出现

                                let dependency_ = self.api_dependencies[dependency_index].clone();
                                //将覆盖到的边加入到新的sequence中去
                                new_sequence._add_dependency(dependency_index);
                                //找到了依赖，当前参数是可以被满足的，设置flag并退出循环
                                dependency_flag = true;

                                /*println!(
                                    "！！！！！！！！！！！！！！！！！！！！可变借用，{}, {}",
                                    api_util::_type_name(
                                        current_ty,
                                        self.cache,
                                        &self.full_name_map
                                    ),
                                    &dependency_.call_type._to_call_string(
                                        &"hhhhhh".to_string(),
                                        self.cache,
                                        &self.full_name_map
                                    )
                                );*/

                                //如果满足move发生的条件
                                if api_util::_move_condition(current_ty, &dependency_.call_type) {
                                    /*println!(
                                        "！！！！！！！！！！！！！！！！！！！！移动，{}, {}",
                                        api_util::_type_name(
                                            current_ty,
                                            self.cache,
                                            &self.full_name_map
                                        ),
                                        &dependency_.call_type._to_call_string(
                                            &"hhhhhh".to_string(),
                                            self.cache,
                                            &self.full_name_map
                                        )
                                    );*/
                                    if _multi_mut.contains(&function_index)
                                        || _immutable_borrow.contains(&function_index)
                                    {
                                        dependency_flag = false;
                                        continue;
                                    } else {
                                        //如果遇到了前面记录的要被可变借用，就相当于move了
                                        if new_sequence.careful_pairs.contains_key(&function_index)
                                        {
                                            let movables = &*(new_sequence
                                                .careful_pairs
                                                .get(&function_index)
                                                .unwrap());
                                            for movable in movables {
                                                /*println!("我是{}, 在这里我可变引用了{}的返回值，但是前面被{}不可变借用了, move掉",
                                                &self.api_functions[input_fun_index]._pretty_print(self.cache, &self.full_name_map),
                                                &self.api_functions[index]._pretty_print(self.cache, &self.full_name_map),
                                                &self.api_functions[new_sequence.functions[*movable].func.1]._pretty_print(self.cache, &self.full_name_map));*/
                                                _moved_indexes.insert(*movable);
                                            }
                                        }
                                        _moved_indexes.insert(function_index);
                                    }
                                }
                                //如果当前调用是可变借用
                                else if api_util::_is_mutable_borrow_occurs(
                                    current_ty,
                                    &dependency_.call_type,
                                ) {
                                    /*println!(
                                        "！！！！！！！！！！！！！！！！！！！！可变借用，{}, {}",
                                        api_util::_type_name(
                                            current_ty,
                                            self.cache,
                                            &self.full_name_map
                                        ),
                                        &dependency_.call_type._to_call_string(
                                            &"hhhhhh".to_string(),
                                            self.cache,
                                            &self.full_name_map
                                        )
                                    );*/
                                    //println!("既然这是 {} 可变引用，我看看有没有符合规则", _type_name(current_ty, self.cache, &self.full_name_map));
                                    //如果在前面的参数已经被借用过了
                                    if _multi_mut.contains(&function_index)
                                        || _immutable_borrow.contains(&function_index)
                                    {
                                        dependency_flag = false;
                                        continue;
                                    } else {
                                        //如果遇到了前面记录的要被可变借用，就相当于move了
                                        if new_sequence.careful_pairs.contains_key(&function_index)
                                        {
                                            let movables = &*(new_sequence
                                                .careful_pairs
                                                .get(&function_index)
                                                .unwrap());
                                            for movable in movables {
                                                /*println!("我是{}, 在这里我可变引用了{}的返回值，但是前面被{}不可变借用了, move掉",
                                                &self.api_functions[input_fun_index]._pretty_print(self.cache, &self.full_name_map),
                                                &self.api_functions[index]._pretty_print(self.cache, &self.full_name_map),
                                                &self.api_functions[new_sequence.functions[*movable].func.1]._pretty_print(self.cache, &self.full_name_map));*/
                                                _moved_indexes.insert(*movable);
                                            }
                                        }

                                        _multi_mut.insert(function_index);
                                        //global_mut_borrow.insert(function_index);
                                    }
                                }
                                //如果当前调用是引用，且之前已经被可变引用过，那么这个引用是非法的
                                else if api_util::_is_immutable_borrow_occurs(
                                    current_ty,
                                    &dependency_.call_type,
                                ) {
                                    //如果前面的参数已经被可变借用了
                                    if _multi_mut.contains(&function_index) {
                                        dependency_flag = false;
                                        continue;
                                    } else {
                                        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                                        if !self.api_functions[input_fun_index]._has_no_output() {
                                            //println!("有输出");
                                            //如果，func2返回值是不可变引用
                                            if api_util::_is_immutable_borrow_type(
                                                &self.api_functions[input_fun_index]
                                                    .clone()
                                                    .output
                                                    .unwrap(),
                                            ) && !api_util::_is_immutable_borrow_type(
                                                &self.api_functions[index].clone().output.unwrap(),
                                            ) {
                                                /*println!(
                                                    "我的 {} 函数返回值是不可变引用，同时我不可变借用了 {} 函数",
                                                    &self.api_functions[input_fun_index]
                                                        ._pretty_print(
                                                            self.cache,
                                                            &self.full_name_map
                                                        ),
                                                    &self.api_functions[index]._pretty_print(
                                                        self.cache,
                                                        &self.full_name_map
                                                    )
                                                );*/
                                                //插入func1和func2

                                                let cur_index = new_sequence.len();
                                                if new_sequence
                                                    .careful_pairs
                                                    .contains_key(&function_index)
                                                {
                                                    new_sequence
                                                        .careful_pairs
                                                        .get_mut(&function_index)
                                                        .unwrap()
                                                        .push(cur_index);
                                                } else {
                                                    new_sequence
                                                        .careful_pairs
                                                        .insert(function_index, vec![cur_index]);
                                                }
                                            }
                                        }
                                        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

                                        _immutable_borrow.insert(function_index);
                                        //global_borrow.insert(function_index);
                                    }
                                }

                                //参数需要加mut 标记的话
                                if api_util::_need_mut_tag(&dependency_.call_type) {
                                    new_sequence._insert_function_mut_tag(function_index);
                                }
                                //如果call type是unsafe的，那么给sequence加上unsafe标记
                                if dependency_.call_type.unsafe_call_type()._is_unsafe() {
                                    new_sequence.set_unsafe();
                                }
                                api_call._add_param(
                                    ParamType::_FunctionReturn,
                                    function_index,
                                    dependency_.call_type,
                                );
                                break;
                            }
                        }
                        if !dependency_flag {
                            //如果这个参数没有寻找到依赖，则这个函数不可以被加入到序列中
                            return None;
                        }
                    }
                }

                //所有参数都可以找到依赖，那么这个函数就可以加入序列
                new_sequence._add_fn(api_call);
                new_sequence._moved = _moved_indexes;
                //new_sequence._mut_borrow = global_mut_borrow;
                //new_sequence._borrow = global_borrow;

                if new_sequence._contains_multi_dynamic_length_fuzzable() {
                    //如果新生成的序列包含多维可变的参数，就不把这个序列加进去
                    return None;
                }
                return Some(new_sequence);
            }
            ApiType::GenericFunction => None,
        }
    }

    //OK: 判断一个函数能否加入给定的序列中,如果可以加入，返回Some(new_sequence),new_sequence是将新的调用加进去之后的情况，否则返回None
    /*pub(crate) fn is_fun_satisfied(
        &self,
        input_type: &ApiType,
        input_fun_index: usize,
        sequence: &ApiSequence,
    ) -> Option<ApiSequence> {
        //判断一个给定的函数能否加入到一个sequence中去
        match input_type {
            ApiType::BareFunction => {
                let mut new_sequence = sequence.clone();
                let mut api_call = ApiCall::_new(input_fun_index);

                let mut _moved_indexes = FxHashSet::default(); //用来保存发生move的那些语句的index
                                                               //let mut _multi_mut = FxHashSet::default(); //用来保存会被多次可变引用的情况
                                                               //let mut _immutable_borrow = FxHashSet::default(); //不可变借用

                //函数
                let input_function = &self.api_functions[input_fun_index];

                //如果是个unsafe函数，给sequence添加unsafe标记
                if input_function._unsafe_tag._is_unsafe() {
                    new_sequence.set_unsafe();
                }
                //如果用到了trait，添加到序列的trait列表
                if input_function._trait_full_path.is_some() {
                    let trait_full_path = input_function._trait_full_path.as_ref().unwrap();
                    new_sequence.add_trait(trait_full_path);
                }

                //看看之前序列的返回值是否可以作为它的参数
                let input_params = &input_function.inputs;
                if input_params.is_empty() {
                    //无需输入参数，直接是可满足的
                    new_sequence._add_fn(api_call);
                    return Some(new_sequence);
                }
                //对于每个参数进行遍历
                for (i, current_ty) in input_params.iter().enumerate() {
                    if api_util::is_fuzzable_type(current_ty, self.cache, &self.full_name_map, None)
                    {
                        //如果当前参数是fuzzable的
                        let current_fuzzable_index = new_sequence.fuzzable_params.len();
                        let fuzzable_call_type = fuzz_type::fuzzable_call_type(
                            current_ty,
                            self.cache,
                            &self.full_name_map,
                            None,
                        );
                        let (fuzzable_type, call_type) =
                            fuzzable_call_type.generate_fuzzable_type_and_call_type();

                        //如果出现了下面这段话，说明出现了Fuzzable参数但不知道如何参数化的
                        //典型例子是tuple里面出现了引用（&usize），这种情况不再去寻找dependency，直接返回无法添加即可
                        match &fuzzable_type {
                            FuzzableType::NoFuzzable => {
                                //println!("Fuzzable Type Error Occurs!");
                                //println!("type = {:?}", current_ty);
                                //println!("fuzzable_call_type = {:?}", fuzzable_call_type);
                                //println!("fuzzable_type = {:?}", fuzzable_type);
                                return None;
                            }
                            _ => {}
                        }

                        //判断要不要加mut tag
                        if api_util::_need_mut_tag(&call_type) {
                            new_sequence._insert_fuzzable_mut_tag(current_fuzzable_index);
                        }

                        //添加到sequence中去
                        new_sequence.fuzzable_params.push(fuzzable_type);
                        api_call._add_param(
                            ParamType::_FuzzableType,
                            current_fuzzable_index,
                            call_type,
                        );
                    } else {
                        // 如果当前参数不是fuzzable的，那么就去api sequence寻找是否有这个依赖
                        // 也就是说，api sequence里是否有某个api的返回值是它的参数

                        //FIXME: 处理move的情况
                        let functions_in_sequence_len = sequence.functions.len();
                        let mut dependency_flag = false;

                        for function_index in 0..functions_in_sequence_len {
                            // 如果这个sequence里面的该函数返回值已经被move掉了，那么就跳过，不再能被使用了
                            // 后面的都是默认这个返回值没有被move，而是被可变借用或不可变借用
                            if new_sequence._is_moved(function_index)
                                || _moved_indexes.contains(&function_index)
                            {
                                continue;
                            }

                            let found_function = &new_sequence.functions[function_index];
                            let (api_type, index) = &found_function.func;
                            if let Some(dependency_index) = self.check_dependency(
                                api_type,
                                *index,
                                input_type,
                                input_fun_index,
                                i,
                            ) {
                                let dependency_ = self.api_dependencies[dependency_index].clone();
                                //将覆盖到的边加入到新的sequence中去
                                new_sequence._add_dependency(dependency_index);
                                //找到了依赖，当前参数是可以被满足的，设置flag并退出循环
                                dependency_flag = true;

                                //如果满足move发生的条件
                                /*if api_util::_move_condition(current_ty, &dependency_.call_type) {
                                    if _multi_mut.contains(&function_index)
                                        || _immutable_borrow.contains(&function_index)
                                    {
                                        dependency_flag = false;
                                        continue;
                                    } else {
                                        _moved_indexes.insert(function_index);
                                    }
                                }
                                //如果当前调用是可变借用
                                if api_util::_is_mutable_borrow_occurs(
                                    current_ty,
                                    &dependency_.call_type,
                                ) {
                                    //如果之前已经被借用过了
                                    if _multi_mut.contains(&function_index)
                                        || _immutable_borrow.contains(&function_index)
                                    {
                                        dependency_flag = false;
                                        continue;
                                    } else {
                                        _multi_mut.insert(function_index);
                                    }
                                }
                                //如果当前调用是引用，且之前已经被可变引用过，那么这个引用是非法的
                                if api_util::_is_immutable_borrow_occurs(
                                    current_ty,
                                    &dependency_.call_type,
                                ) {
                                    if _multi_mut.contains(&function_index) {
                                        dependency_flag = false;
                                        continue;
                                    } else {
                                        _immutable_borrow.insert(function_index);
                                    }
                                }*/
                                //参数需要加mut 标记的话
                                if api_util::_need_mut_tag(&dependency_.call_type) {
                                    new_sequence._insert_function_mut_tag(function_index);
                                }
                                //如果call type是unsafe的，那么给sequence加上unsafe标记
                                if dependency_.call_type.unsafe_call_type()._is_unsafe() {
                                    new_sequence.set_unsafe();
                                }
                                api_call._add_param(
                                    ParamType::_FunctionReturn,
                                    function_index,
                                    dependency_.call_type,
                                );
                                break;
                            }
                        }
                        if !dependency_flag {
                            //如果这个参数没有寻找到依赖，则这个函数不可以被加入到序列中
                            return None;
                        }
                    }
                }
                //所有参数都可以找到依赖，那么这个函数就可以加入序列
                new_sequence._add_fn(api_call);
                for move_index in _moved_indexes {
                    new_sequence._insert_move_index(move_index);
                }
                if new_sequence._contains_multi_dynamic_length_fuzzable() {
                    //如果新生成的序列包含多维可变的参数，就不把这个序列加进去
                    return None;
                }
                return Some(new_sequence);
            }
            ApiType::GenericFunction => todo!(),
        }
    }*/

    /// 从后往前推，做一个dfs
    pub(crate) fn _reverse_construct(
        &self,
        tail_api_type: &ApiType,
        tail_api_index: usize,
        print: bool,
    ) -> Option<ReverseApiSequence> {
        match tail_api_type {
            ApiType::BareFunction => {
                if print {
                    println!("开始反向构造");
                }
                //初始化新反向序列
                let mut new_reverse_sequence = ReverseApiSequence::_new();

                //let mut _moved_indexes = FxHashSet::default(); //用来保存发生move的那些语句的index
                //let mut _multi_mut = FxHashSet::default(); //用来保存会被多次可变引用的情况
                //let mut _immutable_borrow = FxHashSet::default(); //不可变借用

                //我们为终止API创建了调用点，然后要在其中加入api_call
                let mut api_call = ApiCall::_new(tail_api_index);

                let (_, input_fun_index) = api_call.func;
                let input_fun = &self.api_functions[input_fun_index];
                let params = &input_fun.inputs;
                if print {
                    println!("name: {}", input_fun.full_name);
                }
                sleep(Duration::from_millis(20));

                //对于当前函数的param，有依赖
                let mut param_reverse_sequences = Vec::new();
                let mut current_param_index = 1;

                //对每个都要找个参数
                for (input_param_index_, current_ty) in params.iter().enumerate() {
                    /*********************************************************************************************************/
                    //如果当前参数是可fuzz的
                    if api_util::is_fuzzable_type(current_ty, self.cache, &self.full_name_map, None)
                    {
                        //如果当前参数是fuzzable的
                        let current_fuzzable_index = new_reverse_sequence.fuzzable_params.len();
                        let fuzzable_call_type = fuzz_type::fuzzable_call_type(
                            current_ty,
                            self.cache,
                            &self.full_name_map,
                            None,
                        );
                        let (fuzzable_type, call_type) =
                            fuzzable_call_type.generate_fuzzable_type_and_call_type();

                        //如果出现了下面这段话，说明出现了Fuzzable参数但不知道如何参数化的
                        //典型例子是tuple里面出现了引用（&usize），这种情况不再去寻找dependency，直接返回无法添加即可
                        match &fuzzable_type {
                            FuzzableType::NoFuzzable => {
                                return None;
                            }
                            _ => {}
                        }

                        //判断要不要加mut tag
                        if api_util::_need_mut_tag(&call_type) {
                            new_reverse_sequence._insert_fuzzable_mut_tag(current_fuzzable_index);
                        }

                        //添加到sequence中去
                        new_reverse_sequence.fuzzable_params.push(fuzzable_type);
                        api_call._add_param(
                            ParamType::_FuzzableType,
                            current_fuzzable_index,
                            call_type,
                        );
                    }
                    /******************************************************************************************************** */
                    //如果当前参数不可由afl提供，只能去找依赖
                    else {
                        let mut dependency_flag = false;
                        //遍历函数，看看哪个函数的output可以作为当前的param
                        for (output_fun_index, _output_fun) in self.api_functions.iter().enumerate()
                        {
                            //防止死循环
                            if output_fun_index == input_fun_index {
                                break;
                            }

                            //检查前后是否有依赖关系
                            //output_fun -> struct -> input_fun
                            if let Some(dependency_index) = self.check_dependency(
                                &ApiType::BareFunction,
                                output_fun_index,
                                &api_call.func.0,
                                input_fun_index,
                                input_param_index_,
                            ) {
                                let param_seq = match self._reverse_construct(
                                    &ApiType::BareFunction,
                                    output_fun_index,
                                    false,
                                ) {
                                    Some(seq) => seq,
                                    None => {
                                        //没找到通路，那就看其他的api
                                        continue;
                                    }
                                };

                                //下面是找到了通路
                                param_reverse_sequences.push(param_seq.clone());

                                //根据dependency_index找到对应的dependency
                                let dependency_ = self.api_dependencies[dependency_index].clone();

                                //将覆盖到的边加入到新的sequence中去
                                //好像没啥用
                                new_reverse_sequence._add_dependency(dependency_index);

                                //找到了依赖，当前参数是可以被满足的，设置flag并退出循环
                                dependency_flag = true;

                                //参数需要加mut 标记的话
                                if api_util::_need_mut_tag(&dependency_.call_type) {
                                    new_reverse_sequence
                                        ._insert_function_mut_tag(current_param_index);
                                }
                                //如果call type是unsafe的，那么给sequence加上unsafe标记
                                if dependency_.call_type.unsafe_call_type()._is_unsafe() {
                                    new_reverse_sequence._set_unsafe();
                                }

                                //为api_call添加依赖
                                api_call._add_param(
                                    ParamType::_FunctionReturn,
                                    current_param_index,
                                    dependency_.call_type,
                                );
                                current_param_index += param_seq.functions.len();

                                if print {
                                    println!(
                                        "找到了依赖，{}的返回值给{}",
                                        self.api_functions[output_fun_index].full_name,
                                        self.api_functions[input_fun_index].full_name
                                    );
                                }
                                break;
                            }
                        }
                        //如果所有函数都无法作为当前函数的前驱。。。
                        if !dependency_flag {
                            if print {
                                println!("所有函数都无法作为当前函数的前驱");
                            }
                            return None;
                        }
                    }
                    /******************************************************************************************************** */
                }
                //遍历完所有参数，merge所有反向序列

                new_reverse_sequence.functions.push(api_call);

                for seq in param_reverse_sequences {
                    new_reverse_sequence = new_reverse_sequence._combine(seq);
                }

                if print {
                    new_reverse_sequence._print_reverse_sequence(&self);

                    println!("反向构造结束");
                }
                return Some(new_reverse_sequence);
            }
            ApiType::GenericFunction => todo!(),
        }
    }

    //判断一个依赖是否存在,存在的话返回Some(ApiDependency),否则返回None
    pub(crate) fn check_dependency(
        &self,
        output_type: &ApiType,
        output_index: usize,
        input_type: &ApiType,
        input_index: usize,
        input_param_index_: usize,
    ) -> Option<usize> {
        let dependency_num = self.api_dependencies.len();
        for index in 0..dependency_num {
            let dependency = &self.api_dependencies[index];
            //FIXME: 直接比较每一项内容是否可以节省点时间？
            let tmp_dependency = ApiDependency {
                output_fun: (*output_type, output_index),
                input_fun: (*input_type, input_index),
                input_param_index: input_param_index_,
                call_type: dependency.call_type.clone(),
            };
            if tmp_dependency == *dependency {
                //存在依赖
                return Some(index);
            }
        }
        //没找到依赖
        return None;
    }

    //判断一个调用序列是否已经到达终止端点
    fn is_sequence_ended(&self, api_sequence: &ApiSequence, support_generic: bool) -> bool {
        let functions = &api_sequence.functions;
        let last_fun = functions.last();
        match last_fun {
            None => false,
            Some(api_call) => {
                let (api_type, index) = &api_call.func;
                match api_type {
                    ApiType::BareFunction => {
                        let last_func = &self.api_functions[*index];
                        if last_func._is_end_function(
                            self.cache,
                            &self.full_name_map,
                            support_generic,
                        ) {
                            return true;
                        } else {
                            return false;
                        }
                    }
                    ApiType::GenericFunction => todo!(),
                }
            }
        }
    }
}

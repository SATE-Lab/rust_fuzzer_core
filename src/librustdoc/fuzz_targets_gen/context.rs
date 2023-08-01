//! context相当于驱动器
//! 实现了FormatRenderer<'tcx>（暂时没用）
//! 在init中对数据进行解析

use std::path::PathBuf;
use std::rc::Rc;

use rustc_data_structures::fx::FxHashMap;
//use rustc_data_structures::fx::{FxHashMap, FxHashSet};
//use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;

use super::{api_function, api_util, impl_util};
use crate::clean::{self, types as clean_types};
use crate::config::RenderOptions;
use crate::error::Error;
use crate::formats::cache::Cache;
use crate::formats::item_type::ItemType;
use crate::formats::FormatRenderer;
use crate::fuzz_targets_gen::api_graph::ApiGraph;
use crate::fuzz_targets_gen::extract_dep::extract_all_dependencies;
use crate::fuzz_targets_gen::extract_info::ExtractInfo;
use crate::fuzz_targets_gen::file_util::{self};
use rustc_data_structures::fx::FxHashSet;

lazy_static! {
    pub static ref REAL_WORLD_CRATE: FxHashSet<String> = {
        let mut m = FxHashSet::default();
        m.insert("hifitime");//yes
        m.insert("url");//yes
        m.insert("ratatui");//yes
        m.insert("regex_automata");//yes
        m.insert("time");
        m.insert("regex");//yes
        m.insert("unicode_segmentation");
        m.insert("csv");
        m.insert("clap");
        m.insert("tui");
        m.insert("http");
        m.insert("hyper");
        m.insert("textwrap");
        m.insert("serde_json");
        m.insert("chrono");
        m.insert("bytes");
        m.insert("byteorder");
        m.insert("charabia");
        m.insert("bat");
        m.insert("xi_core_lib");
        m.insert("semver");
        m.insert("winit");
        m.insert("difference");
        m.insert("diffy");
        m.insert("tokio");
        m.insert("httparse");
        m.insert("regex_syntax");
        m.insert("protobuf");
        m.insert("console");
        m.insert("unicode_normalization");
        let m = m.into_iter().map(|x| x.to_string()).collect();
        m
    };
}

#[derive(Clone)]
pub(crate) struct Context<'tcx> {
    /// Current hierarchy of components leading down to what's currently being
    /// rendered
    pub(crate) current: Vec<Symbol>,

    /// 代表了目前mod的路径! 比如fuzz_targets_gen::context
    pub(crate) dst: PathBuf,

    /// Type Context
    pub(crate) _tcx: TyCtxt<'tcx>,
    pub(crate) _cache: Rc<Cache>,
}

impl Context<'_> {
    /// 获得全部的路径名称，比如 crate::abc::cde::FunctionName
    pub(crate) fn full_path(&self, item: &clean_types::Item) -> String {
        /// 辅助函数，用::连接
        fn join_with_double_colon(syms: &[Symbol]) -> String {
            let mut s = String::with_capacity(200);
            s.push_str(syms[0].as_str());
            for sym in &syms[1..] {
                s.push_str("::");
                s.push_str(sym.as_str());
            }
            s
        }

        let mut s = join_with_double_colon(&self.current);
        s.push_str("::");
        s.push_str(item.name.unwrap().as_str());
        s
    }
}

impl<'tcx> FormatRenderer<'tcx> for Context<'tcx> {
    fn descr() -> &'static str {
        "fuzz targets generator"
    }

    const RUN_ON_MODULE: bool = true;

    fn init(
        krate: clean::Crate,
        options: RenderOptions,
        cache: Cache,
        tcx: TyCtxt<'tcx>,
    ) -> Result<(Self, clean::Crate), Error> {
        let cx =
            Context { current: Vec::new(), dst: PathBuf::new(), _tcx: tcx, _cache: Rc::new(cache) };
        let out_dir_str = options.output.to_str().unwrap();
        let strs: Vec<&str> = out_dir_str.split("/").collect();
        let target_dir_name = strs[strs.len() - 2];

        if target_dir_name == "target" {
            // 解析corpus program

            let tested_lib_name = "semver";
            let experiment_root = "/home/yxz/workspace/fuzz/experiment_root/";

            if !std::env::current_dir().unwrap().starts_with(experiment_root) {
                return Ok((cx, krate));
            }

            println!(
                "\nStart to parse dependencies.\nThe name of the parsed crate is {}.",
                krate.name(tcx)
            );
            let _ = tcx.sess.time("build_call_graph", || {
                let all_dependencies = extract_all_dependencies(tcx);
                //print_all_dependencies(tcx, all_dependencies.clone(), true);

                let enable = true;
                let extract_info = ExtractInfo::new(
                    tcx,
                    krate.name(tcx).to_string(),
                    tested_lib_name.to_string().replace("-", "_"),
                    &all_dependencies,
                    enable,
                );

                extract_info.print_sequence(enable, experiment_root, tested_lib_name);
                extract_info.print_dependencies_info(enable, experiment_root, tested_lib_name);
                extract_info.print_order_info(enable, experiment_root, tested_lib_name);
                extract_info.print_functions_info(enable, experiment_root, tested_lib_name);
            });

            println!(
                "Finish parsing dependencies. The name of the parsed crate is {}.",
                krate.name(tcx)
            );
        } else {
            use std::time::Instant;
            let start = Instant::now();
            // 解析tested lib
            let kname = krate.name(tcx).to_string();

            if !REAL_WORLD_CRATE.contains(&kname) {
                println!("待测库没有这个crate {}", kname);
                return Ok((cx, krate));
            }

            println!(
                "\nStart to parse tested crate and generate test file.\nThe name of the tested crate is {}.",
                kname
            );

            let support_generic = false;

            // 新建一个API依赖图
            let mut api_graph = ApiGraph::new(&krate.name(tcx).to_string(), cx.cache());
            let mut full_name_map = impl_util::FullNameMap::new();

            // 下面的代码块把method和bare function解析进入api_graph
            {
                impl_util::extract_impls_from_cache(
                    cx.cache(),
                    tcx,
                    &mut full_name_map,
                    &mut api_graph,
                );
                let _ret =
                    cx.clone().add_bare_functions_into_api_graph(tcx, &krate, &mut api_graph);
            }

            api_graph.filter_functions(support_generic);

            api_graph.find_all_dependencies(support_generic);

            println!("total functions in crate : {:?}", api_graph.api_functions.len());

            use crate::fuzz_targets_gen::api_graph::GraphTraverseAlgorithm::*;

            let fries = true;

            let random = false;

            let fudge = false;
            let fudge_test_lib = "bat";

            let max_num = 100;
            let max_len = 15;

            if fries {
                println!(
                    "Fries Start!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
                );
                api_graph.api_sequences.clear();
                //let generation_strategy = _Bfs;
                let generation_strategy = _UseRealWorld;
                //let generation_strategy = _RandomWalk;
                api_graph.generate_all_possoble_sequences(
                    generation_strategy,
                    krate.name(tcx).as_str().replace("_", "-").as_str(),
                    max_num,
                    max_len,
                    support_generic,
                );
                // 计算经过的时间
                let duration = start.elapsed();
                println!("代码执行时间: {:?}", duration);

                if file_util::can_write_to_file(
                    &api_graph._crate_name.replace("_", "-"),
                    //&"unicode-segmentation".to_owned(),
                    generation_strategy,
                ) {
                    println!("I will write test case into files");
                    //whether to use random strategy
                    let file_helper = file_util::FileHelper::new(
                        &api_graph,
                        generation_strategy,
                        max_num,
                        max_len,
                    );
                    file_helper.write_files();
                }

                println!("Fries! Finish to parse tested crate and generate test file.");
            }

            if fudge {
                println!(
                    "Fudge Start!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
                );
                api_graph.api_sequences.clear();
                let generation_strategy = _Fudge;
                //let generation_strategy = _RandomWalk;
                api_graph.generate_all_possoble_sequences(
                    generation_strategy,
                    fudge_test_lib.replace("_", "-").as_str(),
                    //krate.name(tcx).as_str().replace("_", "-").as_str(),
                    max_num,
                    max_len,
                    support_generic,
                );
                // 计算经过的时间
                let duration = start.elapsed();
                println!("代码执行时间: {:?}", duration);
                println!("total functions in crate : {:?}", api_graph.api_functions.len());

                if file_util::can_write_to_file(
                    &api_graph._crate_name.replace("_", "-"),
                    //&"unicode-segmentation".to_owned(),
                    generation_strategy,
                ) {
                    println!("I will write test case into files");
                    //whether to use random strategy
                    let file_helper = file_util::FileHelper::new(
                        &api_graph,
                        generation_strategy,
                        max_num,
                        max_len,
                    );
                    file_helper.write_files();
                }

                println!("Fudge! Finish to parse tested crate and generate test file.");
            }

            if random {
                println!(
                    "Random Start!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
                );
                api_graph.api_sequences.clear();
                let generation_strategy = _RandomWalk;
                //let generation_strategy = _RandomWalk;
                api_graph.generate_all_possoble_sequences(
                    generation_strategy,
                    krate.name(tcx).as_str().replace("_", "-").as_str(),
                    max_num,
                    max_len,
                    support_generic,
                );
                // 计算经过的时间

                println!("total functions in crate : {:?}", api_graph.api_functions.len());

                if file_util::can_write_to_file(
                    &api_graph._crate_name.replace("_", "-"),
                    //&"unicode-segmentation".to_owned(),
                    generation_strategy,
                ) {
                    println!("I will write test case into files");
                    //whether to use random strategy
                    let file_helper = file_util::FileHelper::new(
                        &api_graph,
                        generation_strategy,
                        max_num,
                        max_len,
                    );
                    file_helper.write_files();
                }

                println!("Random! Finish to parse tested crate and generate test file.");
            }
            let duration = start.elapsed();
            println!("代码执行时间: {:?}", duration);
        }
        Ok((cx, krate))
    }

    fn make_child_renderer(&self) -> Self {
        self.clone()
    }

    fn item(&mut self, item: clean::Item) -> Result<(), Error> {
        //FIXME: 如果是函数
        let _name = item.name;
        Ok(())
    }

    fn mod_item_in(&mut self, item: &clean::Item) -> Result<(), Error> {
        let _name = item.name;
        Ok(())
    }

    fn mod_item_out(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn after_krate(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn cache(&self) -> &Cache {
        &self._cache
    }
}

impl Context<'_> {
    /// 用来把裸函数装进图中
    fn add_bare_functions_into_api_graph(
        self,
        tcx: TyCtxt<'_>,
        raw_krate: &clean::Crate,
        mut api_graph: &mut ApiGraph<'_>,
    ) -> Result<(), Error> {
        // 当前的krate
        let krate = raw_krate.clone();
        //crate根代表的module
        let item = krate.module.clone();

        //let krate_name = krate.name(tcx).clone();
        //item.name = Some(krate_name);

        //创建栈，对其进行深度优先搜索
        //let mut work = vec![(self.clone(), item)];

        {
            //while let Some((cx, item)) = work.pop() {
            self.add_bare_functions_into_api_graph_util(
                tcx,
                item,
                &mut api_graph,
                //&mut work,
                //|conx, item| {
                // work.push((conx.to_owned(), item))
                //}
            )?
            //}
        }
        Ok(())
    }

    /// 辅助函数
    fn add_bare_functions_into_api_graph_util(
        mut self,
        tcx: TyCtxt<'_>,
        item: clean::Item,
        api_graph: &mut ApiGraph<'_>,
    ) -> Result<(), Error> {
        //如果是模块，就递归进去
        if item.is_mod() {
            let name = item.name.unwrap();
            if name.is_empty() {
                panic!("empty name: {:?}", self.current);
            }

            //深入一层，在后面要回溯
            self.dst.push(name.to_string());
            self.current.push(name);

            let mod_name =
                self.current.iter().map(|x| x.to_string()).collect::<Vec<String>>().join("::");

            //添加mod的可见性
            api_graph.add_mod_visibility(&mod_name, &item.visibility(tcx).unwrap().expect_local());

            let m = match *item.kind {
                clean::StrippedItem(box clean::ModuleItem(m)) | clean::ModuleItem(m) => m,
                _ => unreachable!(),
            };

            //对module里面的内容进行遍历
            for item in m.items {
                //work.push((self.clone(), item));
                //f(self, item);
                let cx = self.clone();
                cx.add_bare_functions_into_api_graph_util(tcx, item.clone(), api_graph)?
            }

            //回溯
            self.dst.pop();
            self.current.pop();
        }
        // 如果不是模块，但有名字
        else if item.name.is_some() {
            //item是函数,将函数添加到api_dependency_graph里面去
            let item_type = item.type_();
            if item_type == ItemType::Function {
                let full_name = self.full_path(&item);
                //println!("full_name = {}", full_name);
                match *item.kind {
                    clean::FunctionItem(ref func) => {
                        let decl = func.decl.clone();
                        let _generics = func.generics.clone();

                        let clean::FnDecl { inputs, output, .. } = decl;
                        let inputs = api_util::_extract_input_types(&inputs);
                        let output = api_util::_extract_output_type(&output);
                        let api_unsafety = api_function::ApiUnsafety::_get_unsafety_from_fnheader(
                            &item.fn_header(tcx).unwrap(),
                        );
                        let api_fun = api_function::ApiFunction {
                            full_name,
                            _generics,
                            generic_substitutions: FxHashMap::default(), //！！！！！！！初始化
                            inputs,
                            output,
                            _trait_full_path: None,
                            _unsafe_tag: api_unsafety,
                            visibility: item.visibility(tcx).unwrap().expect_local(),
                        };

                        //let output_type = api_fun.output.clone().unwrap();
                        //println!("{:?}", output_type);
                        //let full_name_map = &api_dependency_graph.full_name_map;
                        //let preluded_type = prelude_type::PreludeType::from_type(&output_type, full_name_map);
                        //println!("{:?}", preluded_type);
                        //println!("preluded_type: {}", preluded_type._to_type_name(full_name_map));

                        api_graph.add_api_function(api_fun);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

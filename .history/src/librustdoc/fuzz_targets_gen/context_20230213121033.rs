//! context相当于驱动器
//! 实现了FormatRenderer<'tcx>（暂时没用）
//! 在init中对数据进行解析
//!
//!
//!

use std::path::PathBuf;
use std::rc::Rc;

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
use crate::fuzz_targets_gen::extract_dep::{extract_all_dependencies, print_all_dependencies};
use crate::fuzz_targets_gen::extract_seq::ExtractSequence;
use crate::fuzz_targets_gen::function::Function;

#[derive(Clone)]
pub(crate) struct Context<'tcx> {
    /// Current hierarchy of components leading down to what's currently being
    /// rendered
    pub(crate) current: Vec<Symbol>,

    pub dst: PathBuf,

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

        println!("Output dir name is {}", target_dir_name);
        if target_dir_name == "target" {
            //解析
            println!(
                "\nStart to parse dependencies.\nThe name of the parsed crate is {}.",
                krate.name(tcx)
            );
            let _ = tcx.sess.time("build_call_graph", || {
                let all_dependencies = extract_all_dependencies(tcx);
                print_all_dependencies(tcx, all_dependencies.clone(), true);

                let enable = true;
                let mut extract_sequemce = ExtractSequence::new();
                extract_sequemce.extract_sequence(tcx, "url".to_string(), all_dependencies, enable);
                extract_sequemce.print_sequence(enable);
            });

            println!(
                "Finish parsing dependencies. The name of the parsed crate is {}.",
                krate.name(tcx)
            );
        } else {
            println!(
                "\nStart to parse tested crate and generate test file.\nThe name of the tested crate is {}.",
                krate.name(tcx)
            );

            let mut api_graph = ApiGraph::new(&krate.name(tcx).to_string(), cx.cache());
            let mut full_name_map = impl_util::FullNameMap::new();
            impl_util::extract_impls_from_cache(
                cx.cache(),
                tcx,
                &mut full_name_map,
                &mut api_graph,
            );
            let _ret = cx.analyse_clean_krate(tcx, &krate, &mut api_graph);

            api_graph.filter_functions();

            println!(
                "Finish parsing dependencies. The name of the parsed crate is {}.",
                krate.name(tcx)
            );
        }
        Ok((cx, krate))
    }

    fn make_child_renderer(&self) -> Self {
        self.clone()
    }

    fn item(&mut self, item: clean::Item) -> Result<(), Error> {
        //FIXME: 如果是函数
        match *item.kind {
            clean_types::ItemKind::FunctionItem(_) => {
                let full_name = self.full_path(&item);
                println!("Paring function item [{}]", full_name);

                //解析函数签名
                let _function = Function::create(full_name.clone(), item.clone());
                //self.graph.add_function(function);

                //开始解析函数身体
                let hir = self._tcx.hir();
                let _body = hir
                    .body(hir.body_owned_by(item.item_id.as_def_id().unwrap().as_local().unwrap()));
                //println!("Body:\n{:#?}", body);
            }
            _ => {
                println!("Not a function item");
            }
        }

        Ok(())
    }

    fn mod_item_in(&mut self, item: &clean::Item) -> Result<(), Error> {
        println!("{:?}", item);
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
    fn analyse_clean_krate(
        &self,
        tcx: TyCtxt<'_>,
        raw_krate: &clean::Crate,
        mut api_graph: &mut ApiGraph<'_>,
    ) -> Result<(), Error> {
        let krate = raw_krate.clone();
        //println!("analyse clean krate");
        let mut item = krate.module.clone();
        //let span = &item.source;
        //println!("span = {:?}",span);

        let krate_name = krate.name(tcx).clone();
        item.name = Some(krate_name);

        //let mut all = AllTypes::new();
        {
            let mut work = vec![(self, item)];
            while let Some((&mut cx, item)) = work.pop() {
                cx.another_analyse_item(tcx, item, &mut api_graph, |conx, item| {
                    work.push((conx, item))
                })?
            }
        }
        Ok(())
    }
    fn another_analyse_item<F>(
        &mut self,
        tcx: TyCtxt<'_>,
        item: clean::Item,
        api_graph: &mut ApiGraph<'_>,
        mut f: F,
    ) -> Result<(), Error>
    where
        F: FnMut(&mut Context<'_>, clean::Item),
    {
        if item.is_mod() {
            let name = item.name.unwrap();
            if name.is_empty() {
                panic!("empty name : {:?}", self.current);
            }

            let prev = self.dst.clone();
            self.dst.push(name.to_string());
            self.current.push(name);

            let mod_name =
                self.current.iter().map(|x| x.to_string()).collect::<Vec<String>>().join("::");
            api_graph.add_mod_visibility(&mod_name, &item.visibility(tcx).unwrap().expect_local());

            let m = match *item.kind {
                clean::StrippedItem(box clean::ModuleItem(m)) | clean::ModuleItem(m) => m,
                _ => unreachable!(),
            };

            for item in m.items {
                f(self, item);
            }
            self.dst = prev;
            self.current.pop().unwrap();
        } else if item.name.is_some() {
            //item是函数,将函数添加到api_dependency_graph里面去
            let item_type = item.type_();
            if item_type == ItemType::Function {
                let full_name = self.full_path(&item);
                //println!("full_name = {}", full_name);
                match *item.kind {
                    clean::FunctionItem(ref func) => {
                        //println!("func = {:?}",func);
                        let decl = func.decl.clone();
                        let clean::FnDecl { inputs, output, .. } = decl;
                        let generics = func.generics.clone();
                        let inputs = api_util::_extract_input_types(&inputs);
                        let output = api_util::_extract_output_type(&output);
                        let api_unsafety = api_function::ApiUnsafety::_get_unsafety_from_fnheader(
                            &item.fn_header(tcx).unwrap(),
                        );
                        let api_fun = api_function::ApiFunction {
                            full_name,
                            generics,
                            inputs,
                            output,
                            _trait_full_path: None,
                            _unsafe_tag: api_unsafety,
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

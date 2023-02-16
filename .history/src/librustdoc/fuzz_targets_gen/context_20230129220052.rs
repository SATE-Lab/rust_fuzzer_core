//! context相当于驱动器
//! 实现了FormatRenderer<'tcx>（暂时没用）
//! 在init中对数据进行解析
//!
//!
//!

use std::rc::Rc;

//use rustc_data_structures::fx::{FxHashMap, FxHashSet};
//use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;

use crate::clean::{self, types as clean_types};
use crate::config::RenderOptions;
use crate::error::Error;
use crate::formats::cache::Cache;
use crate::formats::FormatRenderer;
use crate::fuzz_targets_gen::extract_dep::{extract_all_dependencies, print_all_dependencies};
use crate::fuzz_targets_gen::extract_seq::ExtractSequence;
use crate::fuzz_targets_gen::function::Function;

#[derive(Clone)]
pub(crate) struct Context<'tcx> {
    /// Current hierarchy of components leading down to what's currently being
    /// rendered
    pub(crate) current: Vec<Symbol>,

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
            //解析
            println!(
                "\nStart to parse tested crate generate test file..\nThe name of the tested crate is {}.",
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
        }
        Ok((Context { current: Vec::new(), _tcx: tcx, _cache: Rc::new(cache) }, krate))
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
        let item_name = item.name.unwrap();
        self.current.push(item_name);

        //FIXME:
        Ok(())
    }

    fn mod_item_out(&mut self) -> Result<(), Error> {
        // Go back to where we were at
        //self.current.pop();
        Ok(())
    }

    fn after_krate(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn cache(&self) -> &Cache {
        &self._cache
    }
}

use std::rc::Rc;

//use rustc_data_structures::fx::{FxHashMap, FxHashSet};
//use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;

use crate::clean;
use crate::config::RenderOptions;
use crate::error::Error;
use crate::formats::cache::Cache;
use crate::formats::FormatRenderer;

pub(crate) struct Context<'tcx> {
    tcx: TyCtxt<'tcx>,
    cache: Rc<Cache>,
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
        println!("Name of the parsed crate is {}.", krate.name(tcx));

        Ok((Context { tcx, cache: Rc::new(cache) }, krate))
    }

    fn make_child_renderer(&self) -> Self {
        todo!()
    }

    fn item(&mut self, item: clean::Item) -> Result<(), Error> {
        todo!()
    }

    fn mod_item_in(&mut self, item: &clean::Item) -> Result<(), Error> {
        todo!()
    }

    fn after_krate(&mut self) -> Result<(), Error> {
        todo!()
    }

    fn cache(&self) -> &Cache {
        todo!()
    }

    fn mod_item_out(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

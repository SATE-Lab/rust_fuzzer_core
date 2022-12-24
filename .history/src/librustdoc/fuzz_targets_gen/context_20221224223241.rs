use std::rc::Rc;

//use rustc_data_structures::fx::{FxHashMap, FxHashSet};
//use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;

use crate::clean;
use crate::config::RenderOptions;
use crate::error::Error;
use crate::formats::cache::Cache;
use crate::formats::FormatRenderer;

#[derive(Clone)]
pub(crate) struct Context<'tcx> {
    /// Current hierarchy of components leading down to what's currently being
    /// rendered
    pub(crate) current: Vec<Symbol>,

    /// Type Context
    _tcx: TyCtxt<'tcx>,
    _cache: Rc<Cache>,
}

impl<'tcx> FormatRenderer<'tcx> for Context<'tcx> {
    fn descr() -> &'static str {
        "fuzz targets generator"
    }

    const RUN_ON_MODULE: bool = true;

    fn init(
        krate: clean::Crate,
        _options: RenderOptions,
        cache: Cache,
        tcx: TyCtxt<'tcx>,
    ) -> Result<(Self, clean::Crate), Error> {
        println!("Name of the parsed crate is {}.", krate.name(tcx));

        Ok((Context { _tcx: tcx, _cache: Rc::new(cache) }, krate))
    }

    fn make_child_renderer(&self) -> Self {
        self.clone()
    }

    fn item(&mut self, _item: clean::Item) -> Result<(), Error> {
        todo!()
    }

    fn mod_item_in(&mut self, _item: &clean::Item) -> Result<(), Error> {
        todo!()
    }

    fn mod_item_out(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn after_krate(&mut self) -> Result<(), Error> {
        todo!()
    }

    fn cache(&self) -> &Cache {
        &self._cache
    }
}

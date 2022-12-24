use std::cell::RefCell;
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::rc::Rc;

use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;
use rustc_session::Session;
use rustc_span::def_id::LOCAL_CRATE;

use rustdoc_json_types as types;

use crate::clean::types::{ExternalCrate, ExternalLocation};
use crate::clean::ItemKind;
use crate::config::RenderOptions;
use crate::docfs::PathError;
use crate::error::Error;
use crate::formats::cache::Cache;
use crate::formats::FormatRenderer;
use crate::json::conversions::{from_item_id, from_item_id_with_name, IntoWithTcx};
use crate::{clean, try_err};

pub(crate) struct TargetGenerator<'tcx> {
    tcx: TyCtxt<'tcx>,
    /// A mapping of IDs that contains all local items for this crate which gets output as a top
    /// level field of the JSON blob.
    index: Rc<RefCell<FxHashMap<types::Id, types::Item>>>,
    /// The directory where the blob will be written to.
    out_path: PathBuf,
    cache: Rc<Cache>,
    imported_items: FxHashSet<DefId>,
}

impl<'tcx> FormatRenderer<'tcx> for TargetGenerator<'tcx> {}

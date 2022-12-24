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

impl<'tcx> FormatRenderer<'tcx> for TargetGenerator {}

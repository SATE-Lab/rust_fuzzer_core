//! 一些工具

use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::DefId;

use crate::formats::item_type::ItemType;

/// 存储名字
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullNameMap {
    pub map: FxHashMap<DefId, (String, ItemType)>,
}

#[allow(dead_code)]
impl FullNameMap {
    pub fn new() -> Self {
        let map = FxHashMap::default();
        FullNameMap { map }
    }

    pub fn push_mapping(&mut self, def_id: &DefId, full_name: &String, item_type: ItemType) {
        self.map.insert(def_id.clone(), (full_name.clone(), item_type));
    }

    pub fn _get_full_name(&self, def_id: &DefId) -> Option<&String> {
        match self.map.get(def_id) {
            None => None,
            Some((full_name, _)) => Some(full_name),
        }
    }

    pub get_type
}

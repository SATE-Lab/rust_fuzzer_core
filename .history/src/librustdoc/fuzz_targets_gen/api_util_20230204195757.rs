//! 一些工具

/// 存储名字
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullNameMap {
    pub map: HashMap<DefId, (String, ItemType)>,
}

#[allow(dead_code)]
impl FullNameMap {
    pub fn new() -> Self {
        let map = HashMap::default();
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
}

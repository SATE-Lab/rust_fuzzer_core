//! 这里定义了模块的可见性，用于通过可见性来过滤函数
//!

use rustc_data_structures::fx::FxHashMap;
use rustc_middle::ty::Visibility;
#[derive(Debug, Clone)]
pub struct ModVisibity {
    pub crate_name: String,
    pub inner: FxHashMap<String, Visibility>,
}

impl ModVisibity {
    /// 构造函数
    pub fn new(crate_name_: &String) -> Self {
        let crate_name = crate_name_.clone();
        let inner = FxHashMap::default();
        ModVisibity { crate_name, inner }
    }

    /// 添加一个新模块，同时标记它的可见性
    pub fn add_one_mod(&mut self, mod_name: &String, visibility: &Visibility) {
        self.inner.insert(mod_name.clone(), visibility.clone());
    }

    /// 获取所有对外不可见的模块
    pub fn get_invisible_mods(&self) -> Vec<String> {
        let mod_number = self.inner.len();

        let mut new_mod_visibility = FxHashMap::default();
        if !self.inner.contains_key(&self.crate_name) {
            panic!("No crate mod");
        }

        // 根模块肯定可见
        new_mod_visibility.insert(self.crate_name.clone(), true);
        for _ in 0..mod_number {
            //对于每一个模块
            for (mod_name, visibility) in &self.inner {
                if new_mod_visibility.contains_key(mod_name) {
                    continue;
                }
                let parent_mod_name = get_parent_mod_name(mod_name).unwrap();
                if !new_mod_visibility.contains_key(&parent_mod_name) {
                    continue;
                }
                let parent_visibility = new_mod_visibility.get(&parent_mod_name).unwrap();
                if Visibility::Public == *visibility && *parent_visibility {
                    new_mod_visibility.insert(mod_name.clone(), true);
                } else {
                    new_mod_visibility.insert(mod_name.clone(), false);
                }
            }

            if new_mod_visibility.len() == mod_number {
                break;
            }
        }

        let mut res = Vec::new();
        for (mod_name, visibility) in &new_mod_visibility {
            if !*visibility {
                res.push(mod_name.clone());
            }
        }
        res
    }
}

/// 辅助函数，获取父模块
pub fn get_parent_mod_name(mod_name: &String) -> Option<String> {
    if !mod_name.contains("::") {
        return None;
    }
    let mut mod_split: Vec<&str> = mod_name.as_str().split("::").collect();
    mod_split.pop();
    let parent_mod_name = mod_split.join("::");
    Some(parent_mod_name)
}

use crate::clean;
use rustc_data_structures::fx::FxHashMap;

use super::api_function::ApiFunction;

#[derive(Debug, Clone)]
pub(crate) struct GenericFunction {
    pub(crate) _api_function: ApiFunction,
    pub(crate) _generic_substitute: FxHashMap<String, clean::Type>,
}

impl From<ApiFunction> for GenericFunction {
    fn from(_api_function: ApiFunction) -> Self {
        GenericFunction { _api_function, _generic_substitute: FxHashMap::default() }
    }
}

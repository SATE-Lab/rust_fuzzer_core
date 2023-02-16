use std::collections::HashMap;

use crate::clean;

use super::api_function::ApiFunction;

#[derive(Debug, Clone)]
pub(crate) struct GenericFunction {
    pub(crate) api_function: ApiFunction,
    pub(crate) generic_substitute: HashMap<String, clean::Type>,
}

impl From<ApiFunction> for GenericFunction {
    fn from(api_function: ApiFunction) -> Self {
        GenericFunction { api_function, generic_substitute: FxHashMap::new() }
    }
}

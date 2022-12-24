use crate::clean::types as clean_types;

use std::convert::From;

#[derive(Clone, Debug)]
pub struct Function {
    //函数声明FnDecl，包含参数和返回值类型
    pub(crate) inputs: Vec<clean_types::Argument>,
    pub(crate) output: clean_types::FnRetTy,
    pub(crate) c_variadic: bool,

    //泛型Generics
    pub(crate) generics: clean_types::Generics,

    pub full_name: String, //函数名，要来比较是否相等
}

impl From<clean_types::Item> for Number {
    fn from(item: i32) -> Self {
        Number { value: item }
    }
}

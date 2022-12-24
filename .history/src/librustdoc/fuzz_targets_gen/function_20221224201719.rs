use rustc_span::symbol::Symbol;

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

pub struct Argument {
    pub(crate) type_: Type,
    pub(crate) name: String,
    /// This field is used to represent "const" arguments from the `rustc_legacy_const_generics`
    /// feature. More information in <https://github.com/rust-lang/rust/issues/83167>.
    pub(crate) is_const: bool,
}

//从Item解析成Function
impl From<clean_types::Item> for Function {
    fn from(item: clean_types::Item) -> Self {
        match *item.kind {
            clean_types::ItemKind::FunctionItem(func) => {
                let inputs = *func.decl.inputs.clone();
                Function { inputs, output: (), c_variadic: (), generics: (), full_name: () }
            }
            _ => {
                panic!("The item is not a function")
            }
        }
    }
}

fn function_from_trait_argument_helper(args: &Vec<clean_types::Argument>) -> Vec<Argument> {
    let arguments = Vec::new();
    for arg in args {
        arg.name.to_string()
    }
}

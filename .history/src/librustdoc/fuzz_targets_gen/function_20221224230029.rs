use rustc_span::Symbol;

use crate::clean::types as clean_types;

use std::convert::From;

#[derive(Clone, Debug)]
pub struct Function {
    pub(crate) full_name: String, //函数名，要来比较是否相等

    //函数声明FnDecl，包含参数和返回值类型
    pub(crate) inputs: Vec<Argument>,
    pub(crate) output: clean_types::FnRetTy,
    pub(crate) c_variadic: bool,

    //泛型Generics
    pub(crate) generics: clean_types::Generics,
}

#[derive(Clone, Debug)]
pub struct Argument {
    pub(crate) type_: clean_types::Type,
    pub(crate) name: Symbol,
    /// This field is used to represent "const" arguments from the `rustc_legacy_const_generics`
    /// feature. More information in <https://github.com/rust-lang/rust/issues/83167>.
    pub(crate) is_const: bool,
}

//从Item解析成Function
impl Function {
    fn create(full_name: String,item: clean_types::Item) -> Self {
        //辅助函数，把clean::types::Argument解析成本mod的Argument，方便使用
        fn function_from_trait_argument_helper(args: &clean_types::Arguments) -> Vec<Argument> {
            let mut arguments = Vec::new();
            for arg in &args.values {
                let type_ = arg.type_.clone();
                let name = arg.name.clone();
                let is_const = arg.is_const;
                let argument = Argument { type_, name, is_const };
                arguments.push(argument);
            }
            arguments
        }

        match *item.kind {
            clean_types::ItemKind::FunctionItem(func) => {
                let full_name;= 

                let clean_types::FnDecl { inputs, output, c_variadic } = func.decl.clone();
                let inputs = function_from_trait_argument_helper(&inputs);

                let generics = func.generics.clone();

                Function { full_name, inputs, output, c_variadic, generics }
            }
            _ => {
                panic!("The item is not a function")
            }
        }
    }
}

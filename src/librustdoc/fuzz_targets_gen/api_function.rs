//! 摘要，这部分是跟API依赖图中ApiFunction有关的API
//! 1. ApiUnsafety：跟安全性有关，不说了
//! 2. ApiFunction：
//!     [`_is_start_function`] 是否是开始函数
//!     [`_is_end_function`] 是否是终结函数
//!     [`contains_mut_borrow`] 是否参数包含可变借用
//!     [`is_not_defined_on_prelude_type`] 是否有Option Result
//!     [`_is_generic_function`] 是否是泛型函数
//!     [`_has_no_output`] 是否没有输出
//!     [`contains_unsupported_fuzzable_type`] 是否包含未支持的fuzzable类型，比如多维可变长度参数
//!     [`_pretty_print`]：打印

use crate::formats::cache::Cache;
use crate::fuzz_targets_gen::api_util;
use crate::fuzz_targets_gen::call_type::CallType;
use crate::fuzz_targets_gen::fuzz_type::{self, FuzzableType};
use crate::fuzz_targets_gen::impl_util::FullNameMap;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::{self, Mutability};
use rustc_middle::ty::Visibility;

use crate::clean;

/// 用来标识API是否unsafe
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum ApiUnsafety {
    Unsafe,
    Normal,
}

impl ApiUnsafety {
    //辅助构造函数作用，标识函数是否是unsafe
    pub(crate) fn _get_unsafety_from_fnheader(fn_header: &rustc_hir::FnHeader) -> Self {
        let unsafety = fn_header.unsafety;
        match unsafety {
            rustc_hir::Unsafety::Unsafe => ApiUnsafety::Unsafe,
            rustc_hir::Unsafety::Normal => ApiUnsafety::Normal,
        }
    }

    //返回是否unsafe
    pub(crate) fn _is_unsafe(&self) -> bool {
        match self {
            ApiUnsafety::Unsafe => true,
            ApiUnsafety::Normal => false,
        }
    }
}

/// 用来标识API图中的API
#[derive(Clone, Debug)]
pub(crate) struct ApiFunction {
    pub(crate) full_name: String,          //函数名，要来比较是否相等
    pub(crate) _generics: clean::Generics, // 泛型
    pub(crate) generic_substitutions: FxHashMap<String, clean::Type>, //用来替换泛型
    pub(crate) inputs: Vec<clean::Type>,   //输入的参数
    pub(crate) output: Option<clean::Type>, //返回值
    pub(crate) _trait_full_path: Option<String>, //Trait的全限定路径,因为使用trait::fun来调用函数的时候，需要将trait的全路径引入
    pub(crate) _unsafe_tag: ApiUnsafety,         //是否unsafe
    pub(crate) visibility: Visibility,           //可见性
}

impl ApiFunction {
    /// 所有参数都是primitive才能是start_function，其中泛型参数虽然我们不打算支持结构体，但是仍然保留生成依赖的可能。
    pub(crate) fn _is_start_function(
        &self,
        cache: &Cache,
        full_name_map: &FullNameMap,
        support_generic: bool,
    ) -> bool {
        let input_types = &self.inputs;
        let mut flag = true;
        for ty in input_types {
            if !api_util::_is_end_type(&ty, cache, full_name_map, support_generic) {
                flag = false;
                break;
            }
        }
        //println!("name: {}, {}", self._pretty_print(cache, full_name_map), flag);
        flag
    }

    /// 返回值不存在或者是primitive类型的函数是终结函数，即返回值是primitive type
    pub(crate) fn _is_end_function(
        &self,
        cache: &Cache,
        full_name_map: &FullNameMap,
        support_generic: bool,
    ) -> bool {
        if self.contains_mut_borrow() {
            return false;
        }
        let return_type = &self.output;
        match return_type {
            Some(ty) => {
                if api_util::_is_end_type(&ty, cache, full_name_map, support_generic) {
                    return true;
                } else {
                    return false;
                }
            }
            None => true,
        }
        //不考虑可变引用或者是可变裸指针做参数的情况
    }

    /// 判断函数，参数是否包含可变借用
    pub(crate) fn contains_mut_borrow(&self) -> bool {
        //let input_len = self.inputs.len();

        for input_type in &self.inputs {
            match input_type {
                clean::Type::BorrowedRef { mutability, .. }
                | clean::Type::RawPointer(mutability, _) => {
                    if let Mutability::Mut = mutability {
                        return true;
                    }
                }
                _ => {}
            }
        }
        return false;
    }

    /// 是否有prelude type，如果不是返回true
    pub(crate) fn is_not_defined_on_prelude_type(&self, prelude_types: &FxHashSet<String>) -> bool {
        let function_name_contains_prelude_type =
            prelude_types.iter().any(|prelude_type| self.full_name.starts_with(prelude_type));
        let trait_contains_prelude_type = if let Some(ref trait_name) = self._trait_full_path {
            prelude_types.iter().any(|prelude_type| trait_name.starts_with(prelude_type))
        } else {
            false
        };
        !function_name_contains_prelude_type & !trait_contains_prelude_type
    }

    //FIXME:  判断一个函数是否是泛型函数
    pub(crate) fn _is_generic_function(&self) -> bool {
        /*let input_types = &self.inputs;
        for ty in input_types {
            if api_util::_is_generic_type(&ty) {
                return true;
            }
        }
        let output_type = &self.output;
        if let Some(ty) = output_type {
            if api_util::_is_generic_type(&ty) {
                return true;
            }
        }
        return false;*/
        for param in &self._generics.params {
            if param.kind.is_type() {
                return true;
            }
        }
        return false;
    }

    /// 是否有返回值
    pub(crate) fn _has_no_output(&self) -> bool {
        self.output.is_none()
    }

    /// 是否包含了未支持的类型
    /// 不兼容的调用类型、多维动态数组&[&[]]
    pub(crate) fn contains_unsupported_fuzzable_type(
        &self,
        cache: &Cache,
        full_name_map: &FullNameMap,
    ) -> bool {
        for input_ty_ in &self.inputs {
            // 意思是
            // 如果有fuzzable_type，就进去判断一下，包含多为动态数组或者不兼容的调用类型的，就不行
            // 否则，就可能是结构体，这种应该pass
            if api_util::is_fuzzable_type(input_ty_, cache, full_name_map, None) {
                // !!!!!!!!!!
                // 从fuzzable_call_type来生成fuzzable_type和call_type

                //这一行返回的是用substitution替换后的FuzzableCallType
                let fuzzable_call_type =
                    fuzz_type::fuzzable_call_type(input_ty_, cache, full_name_map, None);
                //这一行是使用替换后的FuzzableCallType来生成Fuzzable_type和CallType
                let (fuzzable_type, call_type) =
                    fuzzable_call_type.generate_fuzzable_type_and_call_type();

                //这行没用
                match &fuzzable_type {
                    FuzzableType::NoFuzzable => {
                        return true;
                    }
                    _ => {}
                }

                if fuzzable_type._is_multiple_dynamic_length() {
                    return true;
                }

                match &call_type {
                    CallType::_NotCompatible => {
                        return true;
                    }
                    _ => {}
                }
                //警惕！！！差点改错了
            }
        }
        return false;
    }

    /// 打印函数(包含泛型函数)
    pub(crate) fn _pretty_print(&self, cache: &Cache, full_name_map: &FullNameMap) -> String {
        let generic_part = if self._generics.params.len() > 0 {
            let mut line = "\x1B[31m<".to_string();

            for (idx, generic) in self._generics.params.iter().enumerate() {
                line.push_str(generic.name.to_string().as_str());
                if idx != &self._generics.params.len() - 1 {
                    line.push_str(", ");
                }
            }
            line.push_str(">\x1B[0m");
            line
        } else {
            "".to_string()
        };
        let mut fn_line = format!("fn {}{}(", self.full_name, generic_part);
        let input_len = self.inputs.len();
        for i in 0..input_len {
            let input_type = &self.inputs[i];
            if i != 0 {
                fn_line.push_str(", ");
            }
            fn_line.push_str(api_util::_type_name(input_type, cache, full_name_map).as_str());
        }
        fn_line.push_str(")");
        if let Some(ref ty_) = self.output {
            fn_line.push_str("->");
            fn_line.push_str(api_util::_type_name(ty_, cache, full_name_map).as_str());
        }
        fn_line
    }
}

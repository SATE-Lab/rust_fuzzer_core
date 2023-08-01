//! 摘要，这部分是跟API有关的辅助模块
//! [`_extract_input_types`] 解析函数参数类型列表
//! [`_extract_output_type`] 解析函数返回值类型
//! [`_is_generic_type`] 判断是否是泛型
//! [`_is_end_type`] 判断是否是基本类型
//! [`_type_name`] 类型名字
//! [`substitute_type`] 替换泛型参数，在调用same_type之前就把泛型进行替换
//! [`_same_type`]：这个是判断output_type能否通过某些CallType（比如Option、unwrap、&、*这种）转换成input_type

use crate::clean::{self, GenericArg, GenericArgs, PrimitiveType};
use crate::formats::cache::Cache;
use crate::fuzz_targets_gen::call_type::CallType;
use crate::fuzz_targets_gen::fuzz_type::{self, FuzzableCallType};
use crate::fuzz_targets_gen::impl_util::FullNameMap;
use crate::fuzz_targets_gen::prelude_type::{self, PreludeType};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{self, Mutability};
use thin_vec::ThinVec;

/// ok
/// 解析参数类型
pub(crate) fn _extract_input_types(inputs: &clean::Arguments) -> Vec<clean::Type> {
    let mut input_types = Vec::new();
    for argument in &inputs.values {
        let arg_ty = argument.type_.clone();
        input_types.push(arg_ty);
    }
    input_types
}
/// ok
/// 解析返回值类型，如果有就Some，没有就None
pub(crate) fn _extract_output_type(output: &clean::FnRetTy) -> Option<clean::Type> {
    match output {
        clean::FnRetTy::Return(ty) => Some(ty.clone()),
        clean::FnRetTy::DefaultReturn => None,
    }
}
/// ok
/// 判断一个Type是否是泛型
pub(crate) fn _is_generic_type(ty: &clean::Type) -> bool {
    //FIXME: self不需要考虑，因为在产生api function的时候就已经完成转换，但需要考虑类型嵌套的情况
    match ty {
        clean::Type::Generic(_) => true,
        clean::Type::Path { path } => {
            let segments = &path.segments;
            for segment in segments {
                let generic_args = &segment.args;
                match generic_args {
                    clean::GenericArgs::AngleBracketed { args, .. } => {
                        for generic_arg in args.iter() {
                            if let clean::GenericArg::Type(_inner_ty) = generic_arg {
                                return true;
                                //FIXME:
                                // if _is_generic_type(&_inner_ty) {
                                //     return true;
                                // }
                            }
                        }
                    }
                    //其实我不打算考虑这个
                    clean::GenericArgs::Parenthesized { inputs, output } => {
                        for input_ty in inputs.iter() {
                            if _is_generic_type(input_ty) {
                                return true;
                            }
                        }
                        if let Some(output_ty) = output {
                            if _is_generic_type(output_ty) {
                                return true;
                            }
                        }
                    }
                }
            }
            return false;
        }
        clean::Type::Tuple(types) => {
            for ty_ in types {
                if _is_generic_type(ty_) {
                    return true;
                }
            }
            return false;
        }
        clean::Type::Slice(type_)
        | clean::Type::Array(type_, ..)
        | clean::Type::RawPointer(_, type_)
        | clean::Type::BorrowedRef { type_, .. } => {
            let inner_type = &**type_;
            return _is_generic_type(inner_type);
        }
        _ => {
            //infer, qpath, impltrait不考虑！！！！
            //FIXME: implTrait是否当作泛型呢？QPath是否当作泛型呢？
            //如果有不支持的类型，也可以往这个函数里面丢，会在将函数加到图里面的时候最后过滤一遍
            return false;
        }
    }
}

pub(crate) fn _is_immutable_borrow_type(ty: &clean::Type) -> bool {
    //FIXME: self不需要考虑，因为在产生api function的时候就已经完成转换，但需要考虑类型嵌套的情况
    match ty {
        clean::Type::Generic(_) => false,
        clean::Type::Path { path } => {
            let segments = &path.segments;
            for segment in segments {
                let generic_args = &segment.args;
                match generic_args {
                    clean::GenericArgs::AngleBracketed { args, .. } => {
                        for generic_arg in args.iter() {
                            if let clean::GenericArg::Type(inner_ty) = generic_arg {
                                if _is_immutable_borrow_type(&inner_ty) {
                                    return true;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            return false;
        }
        clean::Type::Tuple(types) => {
            for ty_ in types {
                if _is_immutable_borrow_type(ty_) {
                    return true;
                }
            }
            return false;
        }
        clean::Type::Slice(type_)
        | clean::Type::Array(type_, ..)
        | clean::Type::RawPointer(_, type_) => {
            let inner_type = &**type_;
            return _is_immutable_borrow_type(inner_type);
        }
        clean::Type::BorrowedRef { type_, mutability, .. } => {
            if !mutability.is_mut() {
                return true;
            }
            let inner_type = &**type_;
            return _is_immutable_borrow_type(inner_type);
        }
        _ => {
            //infer, qpath, impltrait不考虑！！！！
            //FIXME: implTrait是否当作泛型呢？QPath是否当作泛型呢？
            //如果有不支持的类型，也可以往这个函数里面丢，会在将函数加到图里面的时候最后过滤一遍
            return false;
        }
    }
}

/// ok
/// 是否是终结类型
pub(crate) fn _is_end_type(
    ty: &clean::Type,
    cache: &Cache,
    full_name_map: &FullNameMap,
    support_generic: bool,
) -> bool {
    match ty {
        clean::Type::Path { .. } => {
            //FIXME: need more analyse
            if prelude_type::_prelude_type_need_special_dealing(ty, cache, full_name_map) {
                let prelude_type = PreludeType::from_type(ty, cache, full_name_map);
                let final_type = prelude_type._get_final_type();
                if _is_end_type(&final_type, cache, full_name_map, support_generic) {
                    return true;
                }
            }
            return false;
        }
        clean::Type::Generic(_s) => {
            //println!("generic type = {:?}", s);
            //FIXME: 泛型肯定不是它可以成为结构体
            if support_generic { true } else { false }
        }
        clean::Type::Primitive(_) => true,
        clean::Type::BareFunction(_) => false,
        clean::Type::Tuple(inner) => {
            let mut flag = true;
            for inner_type in inner {
                if !_is_end_type(inner_type, cache, full_name_map, support_generic) {
                    flag = false;
                    break;
                }
            }
            flag
        }
        clean::Type::Slice(inner)
        | clean::Type::Array(inner, ..)
        | clean::Type::RawPointer(_, inner) => {
            let inner_type = &**inner;
            return _is_end_type(inner_type, cache, full_name_map, support_generic);
        }
        clean::Type::BorrowedRef { type_, .. } => {
            let inner_type = &**type_;
            return _is_end_type(inner_type, cache, full_name_map, support_generic);
        }
        clean::Type::QPath(_)
        | clean::Type::Infer
        | clean::Type::ImplTrait(_)
        | clean::Type::DynTrait(_, _) => false,
    }
}

//get the name of a type
pub(crate) fn _type_name(
    type_: &clean::Type,
    cache: &Cache,
    full_name_map: &FullNameMap,
) -> String {
    /*
    if let Some(def_id) = &type_.def_id(cache) {
        if let Some(full_name) = full_name_map._get_full_name(*def_id) {
            return full_name.clone();
        }
    }*/
    match type_ {
        clean::Type::Path { path } => {
            let mut res = "".to_string();
            for (idx, path_seg) in path.segments.iter().enumerate() {
                res += path_seg.name.as_str();

                if let GenericArgs::AngleBracketed { args, .. } = &path_seg.args {
                    if args.len() > 0 {
                        res += "<";
                        for (index2, arg) in args.iter().enumerate() {
                            match &arg {
                                clean::GenericArg::Type(typ) => {
                                    res += &_type_name(typ, cache, full_name_map);
                                }
                                GenericArg::Lifetime(life) => {
                                    res += life.0.as_str();
                                }
                                GenericArg::Const(_) => todo!(),
                                GenericArg::Infer => res += "_",
                            }
                            if index2 != args.len() - 1 {
                                res += ", ";
                            }
                        }
                        res += ">";
                    }
                }
                if idx != path.segments.len() - 1 {
                    res += "::";
                }
            }
            res
        }
        clean::Type::Primitive(primitive_type) => primitive_type.as_sym().to_string(),
        clean::Type::Generic(generic) => generic.to_string(),
        clean::Type::BorrowedRef { type_, mutability, .. } => {
            let inner_type = &**type_;
            let inner_name = _type_name(inner_type, cache, full_name_map);
            let mut_str = if mutability.is_mut() { "mut " } else { "" };
            format!("&{}{}", mut_str, inner_name)
        }
        clean::Type::Tuple(inner_types) => {
            let inner_types_number = inner_types.len();
            let mut res = "(".to_string();
            for i in 0..inner_types_number {
                let inner_type = &inner_types[i];
                if i != 0 {
                    res.push_str(" ,");
                }
                res.push_str(_type_name(inner_type, cache, full_name_map).as_str());
            }
            res.push(')');
            res
        }
        _ => "Currently not supported".to_string(),
    }
}

/// 重要，把泛型类型中的泛型参数都替换
/// 如果替换不成功返回None
#[allow(dead_code)]
pub(crate) fn substitute_type(
    generic_type: clean::Type,
    substitutions: &FxHashMap<String, clean::Type>,
) -> Option<clean::Type> {
    let mut copy_type = generic_type.clone();
    match &mut copy_type {
        //对于 std::core::Vec<T>，替换成std::core::Vec<i32>
        clean::Type::Path { path } => {
            //对于path的每一段（实际上只有一段才会有泛型参数）
            for segments in &mut path.segments {
                //如果这一段的泛型参数是尖括号的形式的话
                if let crate::clean::GenericArgs::AngleBracketed { args, bindings } = &mut segments.args {
                    //获取泛型参数列表，要求可变的
                    let args_ref = &mut (**args);
                    //遍历泛型参数列表
                    for  (iindex, arg) in args_ref.iter_mut().enumerate() {
                        //发现参数列表里有泛型【类型】参数，其他比如生命周期不用管
                        if let GenericArg::Type(ty) = arg {
                            //看看这个泛型参数有没有对应的binding
                            if let Some(_binding) = bindings.get(iindex) {
                                // 处理存在绑定约束的情况
                                // 使用 binding 进行进一步操作
                                // 有binding就不替换了？


                            } else {
                                // 处理没有绑定约束的情况
                                match substitute_type(ty.clone(), substitutions){
                                    Some(substi)=>{
                                        *arg = GenericArg::Type(substi.clone());
                                    }
                                    None=>{
                                        //没有就不替换，因为可能有默认参数
                                        //return None;
                                    }
                                }
                            }

                        }
                    }
                }
            }
        }
        //对于T，替换成i32
        clean::Type::Generic(symbol) => {
            //如果这个类型本身就是泛型，可以直接替换，如果没有就是None
            copy_type = match substitutions.get(&symbol.to_string()){
                 Some(ty)=>{
                    ty.clone()
                }None=>{
                    clean::Primitive(PrimitiveType::I32)
                }
            }
        }
        //对于元组，(T, i32)替换成(i32,i32)
        clean::Type::Tuple(inners) => {
            for inner_type in inners {
                *inner_type = match substitute_type(inner_type.clone(), substitutions) {
                    Some(substi) => substi.clone(),
                    None => {
                        //没查到就返回None
                        return None;
                    }
                }
            }
        }
        //对于slice、数组、原始指针
        clean::Type::Slice(inner)
        | clean::Type::Array(inner, ..)
        | clean::Type::RawPointer(_, inner) => {
            *inner = match substitute_type(*inner.clone(), substitutions) {
                Some(substi) => Box::new(substi),
                None => {
                    return None;
                }
            }
        }
        clean::Type::BorrowedRef { type_, .. } => {
            *type_ = match substitute_type(*type_.clone(), substitutions) {
                Some(substi) => Box::new(substi),
                None => {
                    return None;
                }
            }
        }

        //实体类型
        clean::Type::Primitive(_)
        //下面的不支持
        | clean::Type::BareFunction(_)
        | clean::Type::QPath(_)
        | clean::Type::Infer
        | clean::Type::ImplTrait(_)
        | clean::Type::DynTrait(_, _) => return None,
    }
    //println!("替换成功，返回");
    //最后返回copy_
    Some(copy_type)
}

/// 判断两个类型是否可以通过某种type来转换
/// output_type转换成input_type
pub(crate) fn _same_type(
    output_type: &clean::Type,
    input_type: &clean::Type,
    hard_mode: bool,
    cache: &Cache,
    full_name_map: &FullNameMap,
) -> CallType {
    if hard_mode {
        _same_type_hard_mode(output_type, input_type, cache, full_name_map)
    } else {
        //FIXME: soft mode
        CallType::_NotCompatible
    }
}

//hard_mode
pub(crate) fn _same_type_hard_mode(
    output_type: &clean::Type,
    input_type: &clean::Type,
    cache: &Cache,
    full_name_map: &FullNameMap,
) -> CallType {
    //same type, direct call
    if output_type == input_type {
        return CallType::_DirectCall;
    }

    // 输入类型如果是
    // 1. 引用
    // 2. 原生指针
    // 对输入类型把引用搞没,后面就不在考虑输入类型需要解引用的情况
    match input_type {
        clean::Type::BorrowedRef { mutability, type_, .. } => {
            return _borrowed_ref_in_same_type(
                mutability,
                type_,
                output_type,
                cache,
                full_name_map,
            );
        }
        clean::Type::RawPointer(mutability, type_) => {
            return _raw_pointer_in_same_type(mutability, type_, output_type, cache, full_name_map);
        }
        _ => {}
    }

    //考虑输入类型是prelude type的情况，后面就不再考虑
    if prelude_type::_prelude_type_need_special_dealing(input_type, cache, full_name_map) {
        let input_prelude_type = PreludeType::from_type(input_type, cache, full_name_map);
        let final_type = input_prelude_type._get_final_type();
        let inner_call_type = _same_type_hard_mode(output_type, &final_type, cache, full_name_map);
        match inner_call_type {
            CallType::_NotCompatible => {
                return CallType::_NotCompatible;
            }
            _ => {
                return input_prelude_type._to_call_type(&inner_call_type);
            }
        }
    }

    //对输出类型进行分类讨论
    match output_type {
        //结构体、枚举、联合
        clean::Type::Path { .. } => {
            //FIXME:
            _same_type_resolved_path(output_type, input_type, cache, full_name_map)
        }
        //泛型
        clean::Type::Generic(_generic) => {
            //因为在调用之前已经替换过了，所以不应该在这里出现，如果出现就是
            //println!("!!!{}", _type_name(output_type, cache, full_name_map));
            //panic!("这里不应该出现！");
            CallType::_NotCompatible
        }
        //基本类型
        //FIXME: 暂不考虑两次转换，如char和任意宽度的数字，但考虑char和u8的转换
        clean::Type::Primitive(primitive_type) => _same_type_primitive(primitive_type, input_type),
        clean::Type::Tuple(_inner_types) => CallType::_NotCompatible,
        clean::Type::Slice(_inner_type) => CallType::_NotCompatible,
        clean::Type::Array(_inner_type, _) => CallType::_NotCompatible,
        clean::Type::Infer => CallType::_NotCompatible,
        clean::Type::RawPointer(_, type_) => {
            _same_type_raw_pointer(type_, input_type, cache, full_name_map)
        }
        clean::Type::BorrowedRef { type_, .. } => {
            _same_type_borrowed_ref(type_, input_type, cache, full_name_map)
        }
        clean::Type::BareFunction(_)
        | clean::Type::QPath(_)
        | clean::Type::ImplTrait(_)
        | clean::Type::DynTrait(_, _) => CallType::_NotCompatible,
    }
}

/// ok
/// test if types are the same type
/// 输出类型是ResolvedPath的情况
fn _same_type_resolved_path(
    output_type: &clean::Type,
    input_type: &clean::Type,
    cache: &Cache,
    full_name_map: &FullNameMap,
) -> CallType {
    //处理output type 是 prelude type的情况
    if prelude_type::_prelude_type_need_special_dealing(output_type, cache, full_name_map) {
        let output_prelude_type = PreludeType::from_type(output_type, cache, full_name_map);
        let final_output_type = output_prelude_type._get_final_type();
        let inner_call_type =
            _same_type_hard_mode(&final_output_type, input_type, cache, full_name_map);
        match inner_call_type {
            CallType::_NotCompatible => {
                return CallType::_NotCompatible;
            }
            _ => {
                return output_prelude_type._unwrap_call_type(&inner_call_type);
            }
        }
    }

    match input_type {
        clean::Type::Path { .. } => {
            if *output_type == *input_type {
                //if input type = outer type, then this is the same type
                //only same defid is not sufficient. eg. Option<usize> != Option<&str>
                return CallType::_DirectCall;
            } else if _resolved_path_equal_without_lifetime(output_type, input_type) {
                return CallType::_DirectCall;
            } else {
                return CallType::_NotCompatible;
            }
        }
        _ => CallType::_NotCompatible,
    }
}

/// ok
/// 输出类型是Primitive的情况
fn _same_type_primitive(primitive_type: &PrimitiveType, input_type: &clean::Type) -> CallType {
    match primitive_type {
        PrimitiveType::Isize
        | PrimitiveType::I8
        | PrimitiveType::I16
        | PrimitiveType::I32
        | PrimitiveType::I64
        | PrimitiveType::I128
        | PrimitiveType::Usize
        | PrimitiveType::U8
        | PrimitiveType::U16
        | PrimitiveType::U32
        | PrimitiveType::U64
        | PrimitiveType::U128
        | PrimitiveType::F32
        | PrimitiveType::F64 => {
            //数字类型
            let output_primitive_type = primitive_type;
            match input_type {
                //输入类型也是基础类型
                clean::Type::Primitive(input_primitive_type) => {
                    if output_primitive_type == input_primitive_type {
                        return CallType::_DirectCall;
                    }
                    match input_primitive_type {
                        //输入类型也是数字类型，可以直接as转换
                        PrimitiveType::Isize
                        | PrimitiveType::I8
                        | PrimitiveType::I16
                        | PrimitiveType::I32
                        | PrimitiveType::I64
                        | PrimitiveType::I128
                        | PrimitiveType::Usize
                        | PrimitiveType::U8
                        | PrimitiveType::U16
                        | PrimitiveType::U32
                        | PrimitiveType::U64
                        | PrimitiveType::U128
                        | PrimitiveType::F32
                        | PrimitiveType::F64 => {
                            if output_primitive_type == input_primitive_type {
                                return CallType::_DirectCall;
                            } else {
                                return CallType::_AsConvert(
                                    input_primitive_type.as_sym().to_string(),
                                );
                            }
                        }
                        //输入类型是字符类型，当输出类型是U8的时候，可以as强转
                        PrimitiveType::Char => {
                            if *output_primitive_type == PrimitiveType::U8 {
                                return CallType::_AsConvert(
                                    input_primitive_type.as_sym().to_string(),
                                );
                            } else {
                                return CallType::_NotCompatible;
                            }
                        }
                        PrimitiveType::Bool
                        | PrimitiveType::Str
                        | PrimitiveType::Slice
                        | PrimitiveType::Array
                        | PrimitiveType::Tuple
                        | PrimitiveType::Unit
                        | PrimitiveType::RawPointer
                        | PrimitiveType::Reference
                        | PrimitiveType::Fn
                        | PrimitiveType::Never => {
                            return CallType::_NotCompatible;
                        }
                    }
                }
                _ => return CallType::_NotCompatible,
            }
        }
        PrimitiveType::Char => match input_type {
            clean::Type::Primitive(inner_primitive_type) => match inner_primitive_type {
                PrimitiveType::Char => {
                    return CallType::_DirectCall;
                }
                PrimitiveType::U8 => {
                    return CallType::_AsConvert(inner_primitive_type.as_sym().to_string());
                }
                _ => {
                    return CallType::_NotCompatible;
                }
            },
            _ => CallType::_NotCompatible,
        },
        _ => CallType::_NotCompatible,
    }
}

/// ok
/// 输出类型是RawPointer的情况
fn _same_type_raw_pointer(
    type_: &Box<clean::Type>,
    input_type: &clean::Type,
    cache: &Cache,
    full_name_map: &FullNameMap,
) -> CallType {
    let inner_type = &**type_;
    let inner_compatible = _same_type_hard_mode(inner_type, input_type, cache, full_name_map);
    match inner_compatible {
        CallType::_NotCompatible => {
            return CallType::_NotCompatible;
        }
        _ => {
            return CallType::_UnsafeDeref(Box::new(inner_compatible));
        }
    }
}

/// ok
/// 输出类型是BorrowedRef的情况
fn _same_type_borrowed_ref(
    type_: &Box<clean::Type>,
    input_type: &clean::Type,
    cache: &Cache,
    full_name_map: &FullNameMap,
) -> CallType {
    let inner_type = &**type_;
    let inner_compatible = _same_type_hard_mode(inner_type, input_type, cache, full_name_map);
    match inner_compatible {
        CallType::_NotCompatible => {
            return CallType::_NotCompatible;
        }
        _ => {
            //如果是可以copy的类型，那么直接解引用;否则的话则认为是不能兼容的
            if _copy_type(inner_type) {
                return CallType::_Deref(Box::new(inner_compatible));
            } else {
                //FIXME: 是否需要考虑可以clone的情况？
                return CallType::_NotCompatible;
            }
        }
    }
}

/// ok
/// 作为下个函数的输入类型：second type
/// 处理输入类型是引用的情况
pub(crate) fn _borrowed_ref_in_same_type(
    mutability: &Mutability,
    type_: &Box<clean::Type>,
    output_type: &clean::Type,
    cache: &Cache,
    full_name_map: &FullNameMap,
) -> CallType {
    let inner_type = &**type_;
    let inner_compatible = _same_type_hard_mode(output_type, inner_type, cache, full_name_map);
    match &inner_compatible {
        CallType::_NotCompatible => {
            return CallType::_NotCompatible;
        }
        _ => match mutability {
            Mutability::Mut => {
                return CallType::_MutBorrowedRef(Box::new(inner_compatible.clone()));
            }
            Mutability::Not => {
                return CallType::_BorrowedRef(Box::new(inner_compatible.clone()));
            }
        },
    }
}

/// ok
/// 处理输入类型是裸指针的情况
pub(crate) fn _raw_pointer_in_same_type(
    mutability: &Mutability,
    type_: &Box<clean::Type>,
    output_type: &clean::Type,
    cache: &Cache,
    full_name_map: &FullNameMap,
) -> CallType {
    let inner_type = &**type_;
    let inner_compatible = _same_type_hard_mode(output_type, inner_type, cache, full_name_map);
    match &inner_compatible {
        CallType::_NotCompatible => {
            return CallType::_NotCompatible;
        }
        _ => match mutability {
            Mutability::Mut => {
                return CallType::_MutRawPointer(
                    Box::new(inner_compatible.clone()),
                    inner_type.clone(),
                );
            }
            Mutability::Not => {
                return CallType::_ConstRawPointer(
                    Box::new(inner_compatible.clone()),
                    inner_type.clone(),
                );
            }
        },
    }
}

//判断一个类型是否是按照copy语义来进行传参的
pub(crate) fn _copy_type(type_: &clean::Type) -> bool {
    match type_ {
        clean::Type::Path { .. } => {
            //FIXME: 结构体可能是可以copy的，要看有没有实现copy trait
            return false;
        }
        clean::Type::Generic(_) => {
            //在这里不需要考虑泛型
            return false;
        }
        clean::Type::Primitive(primitive_type) => match primitive_type {
            PrimitiveType::Isize
            | PrimitiveType::I8
            | PrimitiveType::I16
            | PrimitiveType::I32
            | PrimitiveType::I64
            | PrimitiveType::I128
            | PrimitiveType::Usize
            | PrimitiveType::U8
            | PrimitiveType::U16
            | PrimitiveType::U32
            | PrimitiveType::U64
            | PrimitiveType::U128
            | PrimitiveType::F32
            | PrimitiveType::F64
            | PrimitiveType::Char
            | PrimitiveType::Bool => {
                return true;
            }
            _ => {
                return false;
            }
        },
        clean::Type::BareFunction(_) | clean::Type::Infer => return false,
        clean::Type::Tuple(types) => {
            //如果全都是可以copy的，那么整个元组也是可以copy的
            for ty_ in types {
                if !_copy_type(ty_) {
                    return false;
                }
            }
            return true;
        }
        clean::Type::Slice(_type) => {
            //FIXME: 暂时不确定
            return false;
        }
        clean::Type::Array(type_, _) => {
            let inner_type = &**type_;
            if _copy_type(inner_type) {
                return true;
            } else {
                return false;
            }
        }
        clean::Type::RawPointer(..) => {
            return true;
        }
        clean::Type::BorrowedRef { mutability, .. } => match mutability {
            Mutability::Mut => {
                return false;
            }
            Mutability::Not => {
                return true;
            }
        },
        clean::Type::QPath { .. } => {
            //FIXME: 不确定,遇到再看
            return false;
        }
        clean::Type::ImplTrait(_) => {
            //FIXME: 不确定，遇到再看
            return false;
        }
        clean::Type::DynTrait(_, _) => return false,
    }
}

//判断move会发生的条件：
//目前逻辑有些问题
//输入类型不是copy_type，并且调用方式是Direct call, Deref ，UnsafeDeref
pub(crate) fn _move_condition(input_type: &clean::Type, call_type: &CallType) -> bool {
    if call_type._contains_move_call_type() {
        return true;
    }
    if !_copy_type(input_type) {
        if call_type._contains_move_call_type() {
            return true;
        }
        /*match call_type {
            CallType::_DirectCall
            | CallType::_Deref(..)
            | CallType::_UnsafeDeref(..)
            | CallType::_UnwrapOption(..)
            | CallType::_UnwrapResult(..) => {
                return true;
            }
            _ => {}
        }*/
    }
    return false;
}

/// ok
/// 是否是可fuzz的类型
pub(crate) fn is_fuzzable_type(
    ty_: &clean::Type,
    cache: &Cache,
    full_name_map: &FullNameMap,
    substitution: Option<&FxHashMap<String, clean::Type>>,
) -> bool {
    let fuzzable = fuzz_type::fuzzable_call_type(ty_, cache, full_name_map, substitution);
    match fuzzable {
        FuzzableCallType::NoFuzzable => false,
        _ => true,
    }
}

pub(crate) fn _is_mutable_borrow_occurs(input_type_: &clean::Type, call_type: &CallType) -> bool {
    //FIXME: 暂时先这样处理，后面等调整了result处理的逻辑再进行处理
    if call_type._contains_move_call_type() {
        return false;
    }
    //println!("不是move callType,我来看看是不是可变引用");

    match input_type_ {
        clean::Type::BorrowedRef { mutability, .. } | clean::Type::RawPointer(mutability, _) => {
            if let Mutability::Mut = *mutability {
                match call_type {
                    CallType::_DirectCall
                    | CallType::_MutBorrowedRef(..)
                    | CallType::_MutRawPointer(..) => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    return false;
}

pub(crate) fn _is_immutable_borrow_occurs(input_type: &clean::Type, call_type: &CallType) -> bool {
    match input_type {
        clean::Type::BorrowedRef { mutability, .. } | clean::Type::RawPointer(mutability, _) => {
            if let Mutability::Not = *mutability {
                match call_type {
                    CallType::_DirectCall
                    | CallType::_BorrowedRef(..)
                    | CallType::_ConstRawPointer(..) => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    return false;
}

pub(crate) fn _need_mut_tag(call_type: &CallType) -> bool {
    match call_type {
        CallType::_MutBorrowedRef(..) | CallType::_MutRawPointer(..) => true,
        _ => false,
    }
}

/// ok
/// 判断path是否一致，其中需要调用new_segments_without_lifetime来去除生命周期，之后方便比较
pub(crate) fn _resolved_path_equal_without_lifetime(
    ltype: &clean::Type,
    rtype: &clean::Type,
) -> bool {
    if let clean::Type::Path { path: lpath } = ltype {
        if let clean::Type::Path { path: rpath } = rtype {
            /*if *lis_generic || *ris_generic {
                return false;
            }
            if *ldid != *rdid {
                return false;
            }*/
            let clean::Path { res: lres, segments: lsegments } = lpath;
            let clean::Path { res: rres, segments: rsegments } = rpath;
            let lsegment_len = lsegments.len();
            let rsegment_len = rsegments.len();
            if *lres != *rres || lsegment_len != rsegment_len {
                return false;
            }
            let l_segments_without_lifetime = new_segments_without_lifetime(lsegments);
            let r_segments_without_lifetime = new_segments_without_lifetime(rsegments);

            for i in 0..lsegment_len {
                if l_segments_without_lifetime[i] != r_segments_without_lifetime[i] {
                    return false;
                }
            }
            return true;
        }
    }
    return false;
}

/// ok
/// 获取没有生命周期信息的Path，这样就可以比较是否是同一个类型的结构体了
fn new_segments_without_lifetime(
    old_path_segments: &ThinVec<clean::PathSegment>,
) -> Vec<clean::PathSegment> {
    let mut new_segments_without_lifetime = Vec::new();
    for old_path_segment in old_path_segments {
        let segment_name = &old_path_segment.name;
        let generic_args = &old_path_segment.args;
        if let clean::GenericArgs::AngleBracketed { args, bindings } = generic_args {
            let new_args = Vec::new();
            for arg in args.iter() {
                match arg {
                    clean::GenericArg::Lifetime(..) => {} //Lifetime约束被忽略
                    clean::GenericArg::Const(..) | clean::GenericArg::Type(..) => {
                        //Const和Type被加入，因为我们之前就替换泛型了，所以不用考虑泛型
                        //new_args.push(arg.clone());

                        //FIXBUG:我们暂时都不考虑
                    }
                    clean::GenericArg::Infer => todo!(),
                }
            }
            let new_generic_args = clean::GenericArgs::AngleBracketed {
                args: new_args.into(),
                bindings: bindings.clone(),
            };
            let new_path_segment =
                clean::PathSegment { name: segment_name.clone(), args: new_generic_args };
            new_segments_without_lifetime.push(new_path_segment);
        } else {
            new_segments_without_lifetime.push(old_path_segment.clone());
        }
    }
    new_segments_without_lifetime
}

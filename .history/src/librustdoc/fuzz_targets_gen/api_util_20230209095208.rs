//! 一些工具

use crate::fuzz_targets_gen::call_type::CallType;
use rustc_hir::{self, Mutability};
use rustc_middle::ty::{Ty, TyKind};

pub fn _same_type<'tcx>(
    output_type: &Ty<'tcx>,
    input_type: &Ty<'tcx>,
    hard_mode: bool,
) -> CallType<'tcx> {
    if hard_mode {
        _same_type_hard_mode(output_type, input_type);
    } else {
        //TODO:soft mode
        CallType::_NotCompatible
    }
}

//hard_mode
pub fn _same_type_hard_mode<'tcx>(output_type: &Ty<'tcx>, input_type: &Ty<'tcx>) -> CallType<'tcx> {
    //same type, direct call
    if output_type == input_type {
        return CallType::_DirectCall;
    }
    //对输入类型解引用,后面就不在考虑输入类型需要解引用的情况
    match input_type.kind() {
        TyKind::Ref(_, ty, mutbl) => {
            //
            return _borrowed_ref_in_same_type(mutbl, type_, output_type, full_name_map);
        }
        TyKind::RawPtr(tam) => {
            let ty = tam.ty;
            let mutbl = &tam.mutbl;
            return _raw_pointer_in_same_type(mutbl, ty, output_type, full_name_map);
        }

        /*
        clean::Type::BorrowedRef { mutability, type_, .. } => {
            //TODO:should take lifetime into account?
            return _borrowed_ref_in_same_type(mutability, type_, output_type, full_name_map);
        }
        clean::Type::RawPointer(mutability, type_) => {
            return _raw_pointer_in_same_type(mutability, type_, output_type, full_name_map);
        }*/
        _ => {}
    }

    //考虑输入类型是prelude type的情况，后面就不再考虑
    if prelude_type::_prelude_type_need_special_dealing(input_type, full_name_map) {
        let input_prelude_type = PreludeType::from_type(input_type, full_name_map);
        let final_type = input_prelude_type._get_final_type();
        let inner_call_type = _same_type_hard_mode(output_type, &final_type, full_name_map);
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
        clean::Type::ResolvedPath { .. } => {
            _same_type_resolved_path(output_type, input_type, full_name_map)
        }
        //范型
        clean::Type::Generic(_generic) => {
            //TODO:范型处理，暂不考虑
            CallType::_NotCompatible
        }
        //基本类型
        //TODO:暂不考虑两次转换，如char和任意宽度的数字，但考虑char和u8的转换
        clean::Type::Primitive(primitive_type) => _same_type_primitive(primitive_type, input_type),
        clean::Type::BareFunction(_bare_function) => {
            //TODO:有需要的时候在考虑
            CallType::_NotCompatible
        }
        clean::Type::Tuple(_inner_types) => CallType::_NotCompatible,
        clean::Type::Slice(_inner_type) => CallType::_NotCompatible,
        clean::Type::Array(_inner_type, _) => CallType::_NotCompatible,
        clean::Type::Never | clean::Type::Infer => CallType::_NotCompatible,
        clean::Type::RawPointer(_, type_) => {
            _same_type_raw_pointer(type_, input_type, full_name_map)
        }
        clean::Type::BorrowedRef { type_, .. } => {
            _same_type_borrowed_ref(type_, input_type, full_name_map)
        }
        clean::Type::QPath { .. } => {
            //TODO:有需要的时候再考虑
            CallType::_NotCompatible
        }
        clean::Type::ImplTrait(_) => {
            //TODO:有需要的时候在考虑
            CallType::_NotCompatible
        }
    }
}

//test if types are the same type
//输出类型是ResolvedPath的情况
fn _same_type_resolved_path(
    output_type: &clean::Type,
    input_type: &clean::Type,
    full_name_map: &FullNameMap,
) -> CallType {
    //处理output type 是 prelude type的情况
    if prelude_type::_prelude_type_need_special_dealing(output_type, full_name_map) {
        let output_prelude_type = PreludeType::from_type(output_type, full_name_map);
        let final_output_type = output_prelude_type._get_final_type();
        let inner_call_type = _same_type_hard_mode(&final_output_type, input_type, full_name_map);
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
        clean::Type::ResolvedPath { .. } => {
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

//输出类型是Primitive的情况
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
                                    input_primitive_type.as_str().to_string(),
                                );
                            }
                        }
                        //输入类型是字符类型，当输出类型是U8的时候，可以as强转
                        PrimitiveType::Char => {
                            if *output_primitive_type == PrimitiveType::U8 {
                                return CallType::_AsConvert(
                                    input_primitive_type.as_str().to_string(),
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
                    return CallType::_AsConvert(inner_primitive_type.as_str().to_string());
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

//输出类型是RawPointer的情况
fn _same_type_raw_pointer(
    type_: &Box<clean::Type>,
    input_type: &clean::Type,
    full_name_map: &FullNameMap,
) -> CallType {
    let inner_type = &**type_;
    let inner_compatible = _same_type_hard_mode(inner_type, input_type, full_name_map);
    match inner_compatible {
        CallType::_NotCompatible => {
            return CallType::_NotCompatible;
        }
        _ => {
            return CallType::_UnsafeDeref(Box::new(inner_compatible));
        }
    }
}

//输出类型是BorrowedRef的情况
fn _same_type_borrowed_ref(
    type_: &Box<clean::Type>,
    input_type: &clean::Type,
    full_name_map: &FullNameMap,
) -> CallType {
    let inner_type = &**type_;
    let inner_compatible = _same_type_hard_mode(inner_type, input_type, full_name_map);
    match inner_compatible {
        CallType::_NotCompatible => {
            return CallType::_NotCompatible;
        }
        _ => {
            //如果是可以copy的类型，那么直接解引用;否则的话则认为是不能兼容的
            if _copy_type(inner_type) {
                return CallType::_Deref(Box::new(inner_compatible));
            } else {
                //TODO:是否需要考虑可以clone的情况？
                return CallType::_NotCompatible;
            }
        }
    }
}

//作为下个函数的输入类型：second type
//处理输入类型是引用的情况
pub fn _borrowed_ref_in_same_type(
    mutability: &Mutability,
    type_: &Box<clean::Type>,
    output_type: &clean::Type,
    full_name_map: &FullNameMap,
) -> CallType {
    let inner_type = &**type_;
    let inner_compatible = _same_type_hard_mode(output_type, inner_type, full_name_map);
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

//处理输入类型是裸指针的情况
pub fn _raw_pointer_in_same_type(
    mutability: &Mutability,
    type_: &Box<clean::Type>,
    output_type: &clean::Type,
    full_name_map: &FullNameMap,
) -> CallType {
    let inner_type = &**type_;
    let inner_compatible = _same_type_hard_mode(output_type, inner_type, full_name_map);
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

//判断一个类型是否是按照copy语义来进行穿参的
pub fn _copy_type(type_: &clean::Type) -> bool {
    match type_ {
        clean::Type::ResolvedPath { .. } => {
            //TODO:结构体可能是可以copy的，要看有没有实现copy trait
            return false;
        }
        clean::Type::Generic(_) => {
            //TODO:范型可能是可以copy的，要看有没有copy trait的约束
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
        clean::Type::BareFunction(_) | clean::Type::Never | clean::Type::Infer => return false,
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
            //TODO:暂时不确定
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
            //TODO:不确定,遇到再看
            return false;
        }
        clean::Type::ImplTrait(_) => {
            //TODO:不确定，遇到再看
            return false;
        }
    }
}

//判断move会发生的条件：
//目前逻辑有些问题
//输入类型不是copy_type，并且调用方式是Direct call, Deref ，UnsafeDeref
pub fn _move_condition(input_type: &clean::Type, call_type: &CallType) -> bool {
    if call_type._contains_move_call_type() {
        return true;
    }
    if !_copy_type(input_type) {
        match call_type {
            CallType::_DirectCall
            | CallType::_Deref(..)
            | CallType::_UnsafeDeref(..)
            | CallType::_UnwrapOption(..)
            | CallType::_UnwrapResult(..) => {
                return true;
            }
            _ => {}
        }
    }
    return false;
}

pub fn is_fuzzable_type(ty_: &clean::Type, full_name_map: &FullNameMap) -> bool {
    let fuzzable = fuzzable_type::fuzzable_call_type(ty_, full_name_map);
    match fuzzable {
        FuzzableCallType::NoFuzzable => false,
        _ => true,
    }
}

pub fn _is_mutable_borrow_occurs(input_type_: &clean::Type, call_type: &CallType) -> bool {
    //TODO:暂时先这样处理，后面等调整了result处理的逻辑再进行处理
    if call_type._contains_move_call_type() {
        return false;
    }

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

pub fn _is_immutable_borrow_occurs(input_type: &clean::Type, call_type: &CallType) -> bool {
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

pub fn _need_mut_tag(call_type: &CallType) -> bool {
    match call_type {
        CallType::_MutBorrowedRef(..) | CallType::_MutRawPointer(..) => true,
        _ => false,
    }
}

pub fn _resolved_path_equal_without_lifetime(ltype: &clean::Type, rtype: &clean::Type) -> bool {
    if let clean::Type::ResolvedPath { path: lpath, did: ldid, is_generic: lis_generic, .. } = ltype
    {
        if let clean::Type::ResolvedPath {
            path: rpath, did: rdid, is_generic: ris_generic, ..
        } = rtype
        {
            if *lis_generic || *ris_generic {
                return false;
            }
            if *ldid != *rdid {
                return false;
            }
            let clean::Path { global: lglobal, res: lres, segments: lsegments } = lpath;
            let clean::Path { global: rglobal, res: rres, segments: rsegments } = rpath;
            let lsegment_len = lsegments.len();
            let rsegment_len = rsegments.len();
            if *lglobal != *rglobal || *lres != *rres || lsegment_len != rsegment_len {
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

fn new_segments_without_lifetime(
    old_path_segments: &Vec<clean::PathSegment>,
) -> Vec<clean::PathSegment> {
    let mut new_segments_without_lifetime = Vec::new();
    for old_path_segment in old_path_segments {
        let segment_name = &old_path_segment.name;
        let generic_args = &old_path_segment.args;
        if let clean::GenericArgs::AngleBracketed { args, bindings } = generic_args {
            let mut new_args = Vec::new();
            for arg in args {
                match arg {
                    clean::GenericArg::Lifetime(..) => {}
                    clean::GenericArg::Const(..) | clean::GenericArg::Type(..) => {
                        new_args.push(arg.clone());
                    }
                }
            }
            let new_generic_args =
                clean::GenericArgs::AngleBracketed { args: new_args, bindings: bindings.clone() };
            let new_path_segment =
                clean::PathSegment { name: segment_name.clone(), args: new_generic_args };
            new_segments_without_lifetime.push(new_path_segment);
        } else {
            new_segments_without_lifetime.push(old_path_segment.clone());
        }
    }
    new_segments_without_lifetime
}

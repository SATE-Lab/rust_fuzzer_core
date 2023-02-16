//如果构造一个fuzzable的变量
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FuzzableCallType {
    NoFuzzable,
    Primitive(PrimitiveType),
    Tuple(Vec<Box<FuzzableCallType>>),
    Slice(Box<FuzzableCallType>),
    Array(Box<FuzzableCallType>),
    ConstRawPoiner(Box<FuzzableCallType>, clean::Type),
    MutRawPoiner(Box<FuzzableCallType>, clean::Type),
    STR,
    BorrowedRef(Box<FuzzableCallType>),
    MutBorrowedRef(Box<FuzzableCallType>),
    ToOption(Box<FuzzableCallType>),
}

/// 代表可以通过字节序列转化的过程fuzz的类型
#[allow(dead_code)]
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum FuzzType {
    NoFuzzable,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    U64,
    I64,
    F64,
    U128,
    I128,
    Usize,
    Isize,
    Char,
    Bool,
    Str,
    Slice(Box<FuzzType>),
    Tuple(Vec<Box<FuzzType>>),
}

impl FuzzType {
    //是否大小固定
    #[allow(dead_code)]
    pub fn is_fixed_size(&self) -> bool {
        use FuzzType::*;
        match self {
            Str | Slice(_) => false,
            Tuple(inners) => inners.iter().all(|x| x.is_fixed_size()),
            _ => true,
        }
    }

    //最小的尺寸，单位字节
    pub fn min_size(&self) -> usize {
        use FuzzType::*;
        match self {
            NoFuzzable => 0,
            I8 | U8 | Bool => 1,
            I16 | U16 => 2,
            I32 | U32 | F32 | Char => 4,
            I64 | U64 | F64 => 8,
            I128 | U128 => 16,
            Usize | Isize => std::mem::size_of::<usize>(), // 暂时当成64bit系统
            Slice(inner) => inner.min_size(),
            Str => 1,
            Tuple(inners) => {
                let mut total_length = 0;
                for inner in inners {
                    total_length = total_length + inner.min_size();
                }
                total_length
            }
        }
    }

    /// 固定部分的尺寸
    pub fn fixed_size_part_size(&self) -> usize {
        use FuzzType::*;
        match self {
            NoFuzzable => 0,
            Str => 0,
            Slice(..) => 0,
            Tuple(inners) => {
                let mut fixed_part = 0;
                for inner in inners {
                    let inner_length = inner.fixed_size_part_size();
                    fixed_part = fixed_part + inner_length;
                }
                return fixed_part;
            }
            _ => self.min_size(),
        }
    }

    /// 计算长度不固定的参数的个数，主要是需要迭代考虑元组的内部
    pub fn dynamic_size_parts_number(&self) -> usize {
        use FuzzType::*;
        match self {
            Str => 1,
            Slice(_) => 1,
            Tuple(inners) => {
                let mut inner_numbers = 0;
                for inner in inners {
                    let inner_number = inner.dynamic_size_parts_number();
                    inner_numbers = inner_numbers + inner_number;
                }
                inner_numbers
            }
            _ => 0,
        }
    }

    #[allow(dead_code)]
    /// 获得类型名称，用于指定参数类型
    pub fn get_type_string(&self) -> String {
        let mut res = String::default();
        match self {
            FuzzType::NoFuzzable => "",
            FuzzType::U8 => "u8",
            FuzzType::I8 => "i8",
            FuzzType::U16 => "u16",
            FuzzType::I16 => "i16",
            FuzzType::U32 => "u32",
            FuzzType::I32 => "i32",
            FuzzType::F32 => "f32",
            FuzzType::U64 => "u64",
            FuzzType::I64 => "i64",
            FuzzType::F64 => "f64",
            FuzzType::U128 => "u128",
            FuzzType::I128 => "i128",
            FuzzType::Usize => "usize",
            FuzzType::Isize => "isize",
            FuzzType::Char => "char",
            FuzzType::Bool => "bool",
            FuzzType::Str => "&str",
            FuzzType::Slice(inner) => {
                res.push_str("&[");
                res.push_str(inner.get_type_string().as_str());
                res.push_str("]");
                res.as_str()
            }
            FuzzType::Tuple(inners) => {
                res.push_str("(");

                for (i, inner) in inners.iter().enumerate() {
                    if i != 0 {
                        res.push_str(" ,");
                    }
                    let type_string = inner.get_type_string();
                    res.push_str(type_string.as_str());
                }
                res.push_str(")");
                res.as_str()
            }
        }
        .to_string()
    }
}

//判断一个类型是不是fuzzable的，以及如何调用相应的fuzzable变量
pub fn fuzzable_call_type(ty_: &clean::Type, full_name_map: &FullNameMap) -> FuzzableCallType {
    match ty_ {
        clean::Type::ResolvedPath { .. } => {
            let prelude_type = PreludeType::from_type(ty_, full_name_map);
            //result类型的变量不应该作为fuzzable的变量。只考虑作为别的函数的返回值
            match &prelude_type {
                PreludeType::NotPrelude(..) | PreludeType::PreludeResult { .. } => {
                    FuzzableCallType::NoFuzzable
                }
                PreludeType::PreludeOption(inner_type_) => {
                    let inner_fuzzable_call_type = fuzzable_call_type(inner_type_, full_name_map);
                    match inner_fuzzable_call_type {
                        FuzzableCallType::NoFuzzable => {
                            return FuzzableCallType::NoFuzzable;
                        }
                        _ => {
                            return FuzzableCallType::ToOption(Box::new(inner_fuzzable_call_type));
                        }
                    }
                }
            }
        }
        clean::Type::Generic(s) => {
            println!("generic type = {:?}", s);
            FuzzableCallType::NoFuzzable
        }
        clean::Type::Primitive(primitive_type) => {
            FuzzableCallType::Primitive(primitive_type.clone())
        }
        clean::Type::BareFunction(..) => FuzzableCallType::NoFuzzable,
        clean::Type::Tuple(types) => {
            let mut vec = Vec::new();
            for inner_type in types {
                let inner_fuzzable = fuzzable_call_type(inner_type, full_name_map);
                match inner_fuzzable {
                    FuzzableCallType::NoFuzzable => {
                        return FuzzableCallType::NoFuzzable;
                    }
                    _ => {
                        vec.push(Box::new(inner_fuzzable));
                    }
                }
            }
            return FuzzableCallType::Tuple(vec);
        }
        clean::Type::Slice(inner_type) => {
            let inner_ty_ = &**inner_type;
            let inner_fuzzable = fuzzable_call_type(inner_ty_, full_name_map);
            match inner_fuzzable {
                FuzzableCallType::NoFuzzable => {
                    return FuzzableCallType::NoFuzzable;
                }
                _ => {
                    return FuzzableCallType::Slice(Box::new(inner_fuzzable));
                }
            }
        }
        clean::Type::Array(inner_type, ..) => {
            let inner_ty_ = &**inner_type;
            let inner_fuzzable = fuzzable_call_type(inner_ty_, full_name_map);
            match inner_fuzzable {
                FuzzableCallType::NoFuzzable => {
                    return FuzzableCallType::NoFuzzable;
                }
                _ => {
                    return FuzzableCallType::Array(Box::new(inner_fuzzable));
                }
            }
        }
        clean::Type::RawPointer(mutability, type_) => {
            let inner_type = &**type_;
            let inner_fuzzable = fuzzable_call_type(inner_type, full_name_map);
            match inner_fuzzable {
                FuzzableCallType::NoFuzzable => {
                    return FuzzableCallType::NoFuzzable;
                }
                _ => match mutability {
                    Mutability::Mut => {
                        return FuzzableCallType::MutRawPoiner(
                            Box::new(inner_fuzzable),
                            inner_type.clone(),
                        );
                    }
                    Mutability::Not => {
                        return FuzzableCallType::ConstRawPoiner(
                            Box::new(inner_fuzzable),
                            inner_type.clone(),
                        );
                    }
                },
            }
        }
        clean::Type::BorrowedRef { lifetime, mutability, type_, .. } => {
            let inner_type = &**type_;
            //特别处理&str的情况，这时候可以返回一个字符串作为fuzzable的变量
            if *inner_type == clean::Type::Primitive(PrimitiveType::Str)
                && *mutability == Mutability::Not
            {
                if let Some(lifetime_) = lifetime {
                    let lifetime_string = lifetime_.0.as_str();
                    if lifetime_string == "'static" {
                        //如果是static的话，由于无法构造出来，所以只能认为是不可fuzzable的
                        return FuzzableCallType::NoFuzzable;
                    }
                }
                return FuzzableCallType::STR;
            }
            let inner_fuzzable = fuzzable_call_type(inner_type, full_name_map);
            match inner_fuzzable {
                FuzzableCallType::NoFuzzable => {
                    return FuzzableCallType::NoFuzzable;
                }
                _ => match mutability {
                    Mutability::Mut => {
                        return FuzzableCallType::MutBorrowedRef(Box::new(inner_fuzzable));
                    }
                    Mutability::Not => {
                        return FuzzableCallType::BorrowedRef(Box::new(inner_fuzzable));
                    }
                },
            }
        }
        clean::Type::QPath { .. } => {
            return FuzzableCallType::NoFuzzable;
        }
        clean::Type::ImplTrait(..) => {
            return FuzzableCallType::NoFuzzable;
        }
        clean::Type::Never | clean::Type::Infer => {
            return FuzzableCallType::NoFuzzable;
        }
    }
}

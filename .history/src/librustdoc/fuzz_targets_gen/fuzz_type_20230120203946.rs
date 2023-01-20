/// 代表可以通过字节序列转化的过程fuzz的类型
#[allow(dead_code)]
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum FuzzType {
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
    pub fn is_fixed_size(&self) -> bool {
        match self {
            FuzzType::U8 => todo!(),
            FuzzType::I8 => todo!(),
            FuzzType::U16 => todo!(),
            FuzzType::I16 => todo!(),
            FuzzType::U32 => todo!(),
            FuzzType::I32 => todo!(),
            FuzzType::F32 => todo!(),
            FuzzType::U64 => todo!(),
            FuzzType::I64 => todo!(),
            FuzzType::F64 => todo!(),
            FuzzType::U128 => todo!(),
            FuzzType::I128 => todo!(),
            FuzzType::Usize => todo!(),
            FuzzType::Isize => todo!(),
            FuzzType::Char => todo!(),
            FuzzType::Bool => todo!(),
            FuzzType::Str => todo!(),
            FuzzType::Slice(_) => todo!(),
            FuzzType::Tuple(_) => todo!(),
        }
    }
}

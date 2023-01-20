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
            FuzzType::U8
            |FuzzType::I8 
            |FuzzType::U16
            |FuzzType::I16
            |FuzzType::U32
            |FuzzType::I32
            |FuzzType::F32
            |FuzzType::U64
            |FuzzType::I64
            |FuzzType::F64
            |FuzzType::U128
            |FuzzType::I128
            |FuzzType::Usize
            |FuzzType::Isize
            |FuzzType::Char
            |FuzzType::Bool=>false
            FuzzType::Str
            FuzzType::Slice(_)
            FuzzType::Tuple(_)
        }
    }
}

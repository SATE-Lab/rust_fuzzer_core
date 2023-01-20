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
        use FuzzType::*;
        match self {
            Str | Slice(_) => false,
            Tuple(inners) => inners.iter().all(|x| x.is_fixed_size()),
            _ => true,
        }
    }

    pub fn fixed_size(&self) -> usize {
        use FuzzType::*;
        matc
    }
}

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

    pub fn min_size(&self) -> usize {
        use FuzzType::*;

                match self {
                            I8
                            |U8
                            |Bool => 1,
                            I16 | U16 => 2,
                            I32
                            | U32
                            | Char
                            | F32 => 4,
                            I64
                            | U64
                            | F64
                            // 暂时当成64bit系统
                            | Usize
                            | Isize => 8,
                            I128 | U128 => 16,
                            _ => 0,
                    
                
                    Slice(inner_fuzzable) => inner_fuzzable.min_size(),
                    Str => 1,
                    Tuple(inner_fuzzables) => {
                        let mut total_length = 0;
                        for inner_fuzzable in inner_fuzzables {
                            total_length = total_length + inner_fuzzable._min_length();
                        }
                        total_length
                    }
                }
                
            }}

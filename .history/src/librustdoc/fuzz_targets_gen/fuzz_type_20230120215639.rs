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
                            clean::PrimitiveType::I16 | clean::PrimitiveType::U16 => 2,
                            clean::PrimitiveType::I32
                            | clean::PrimitiveType::U32
                            | clean::PrimitiveType::Char
                            | clean::PrimitiveType::F32 => 4,
                            //TODO:在我的64位机器上，isize,usize的宽度为8个字节
                            clean::PrimitiveType::I64
                            | clean::PrimitiveType::U64
                            | clean::PrimitiveType::F64
                            | clean::PrimitiveType::Usize
                            | clean::PrimitiveType::Isize => 8,
                            clean::PrimitiveType::I128 | clean::PrimitiveType::U128 => 16,
                            _ => 0,
                        }
                    }
                    FuzzableType::RefSlice(inner_fuzzable) => inner_fuzzable._min_length(),
                    FuzzableType::RefStr => 1,
                    FuzzableType::Tuple(inner_fuzzables) => {
                        let mut total_length = 0;
                        for inner_fuzzable in inner_fuzzables {
                            total_length = total_length + inner_fuzzable._min_length();
                        }
                        total_length
                    }
                }
            }
        }
    }
}

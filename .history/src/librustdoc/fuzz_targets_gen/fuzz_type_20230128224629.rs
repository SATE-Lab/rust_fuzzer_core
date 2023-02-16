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

    //计算长度不固定的参数的个数，主要是需要迭代考虑元组的内部
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

    pub fn get_type_name(&self) -> String {
        match self {}
    }
}

/// 代表可以通过字节序列转化的过程fuzz的类型
#[allow(dead_code)]
pub enum FuzzType {
    InValid,
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
    _U128,
    _I128,
    _Usize,
    _Isize,
    _Char,
    _Bool,
    _Str,
    _Slice(FuzzType),
    _Tuple(..) => "",
}

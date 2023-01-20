use crate::fuzz_targets_gen::fuzz_type::FuzzType;
use rustc_data_structures::fx::FxHashSet;

#[allow(dead_code)]
struct ToAfl {
    tys: Vec<FuzzType>,
    //用来去重
    map: FxHashSet<FuzzType>,
}

#[allow(dead_code)]
impl ToAfl {
    pub fn new(tys: Vec<FuzzType>) -> Self {
        ToAfl { tys, map: FxHashSet::default() }
    }

    pub fn generate_helper_functions(&self) -> String {
        self::get_de
    }

    //对某一个类型，获得它的生成函数依赖的类型
    fn get_dependency_fuzz_type(ty: &FuzzType) -> Vec<FuzzType> {
        let types = Vec::new();

        use FuzzType::*;
        match ty {
            U8 | I8 | Slice(..) | Str | F32 | F64 => {
                // 没有依赖
            }
            Bool | U16 => {
                let mut u8_dependency = Self::get_dependency_fuzz_type(&U8);
                types.append(&mut u8_dependency);
            }

            I16 => {
                let mut i8_dependency = Self::get_dependency_fuzz_type(&I8);
                types.append(&mut i8_dependency);
            }
            U32 => {
                let mut u16_dependency = Self::get_dependency_fuzz_type(&U16);
                types.append(&mut u16_dependency);
            }
            I32 => {
                let mut i16_dependency = Self::get_dependency_fuzz_type(&I16);
                types.append(&mut i16_dependency);
            }
            U64 => {
                let mut u32_dependency = Self::get_dependency_fuzz_type(&U32);
                types.append(&mut u32_dependency);
            }
            I64 => {
                let mut i32_dependency = Self::get_dependency_fuzz_type(&I32);
                types.append(&mut i32_dependency);
            }
            U128 => {
                let mut u64_dependency = Self::get_dependency_fuzz_type(&U64);
                types.append(&mut u64_dependency);
            }
            I128 => {
                let mut i64_dependency = Self::get_dependency_fuzz_type(&I64);
                types.append(&mut i64_dependency);
            }
            Usize => {
                let mut u64_dependency = Self::get_dependency_fuzz_type(&U64);
                types.append(&mut u64_dependency);
            }
            Isize => {
                let mut i64_dependency = Self::get_dependency_fuzz_type(&I64);
                types.append(&mut i64_dependency);
            }
            Char => {
                let mut u32_dependency = Self::get_dependency_fuzz_type(&U32);
                types.append(&mut u32_dependency);
            }
            Tuple(eles) => {
                for ele in *eles {
                    types.append(&mut Self::get_dependency_fuzz_type(&ele));
                }
            }
        }
        types
    }

    //进行去重操作
    fn collect_all_unique_fuzz_type(&self) {
        for ty in self.tys {
            if !self.map.contains(&ty) {
                self.map.insert(ty.clone());
            }
        }
    }

    fn generate_all_u8_to_other_type_functions(&self) -> String {
        let tys = (&self).tys.clone();

        //一个HashSet，存储可fuzz的类型
        let mut map = FxHashSet::default();

        //遍历，如果有对应类型，就不用再生成了
        let mut res = String::default();

        let have_slice = false;
        for ty in tys {
            res += &match ty {
                FuzzType::Slice(_) => {
                    if !have_slice {
                        Self::generate_u8_to_other_type_function(ty)
                    } else {
                        String::default()
                    }
                }
                other => {
                    if map.contains(other) {
                        map.insert(other);
                        Self::generate_u8_to_other_type_function(ty)
                    } else {
                        String::default()
                    }
                }
            }
        }
        res
    }

    fn generate_u8_to_other_type_function(fuzz_type: &FuzzType) -> String {
        let mut s = String::default();
        use FuzzType::*;
        match fuzz_type {
            U8 => _data_to_u8(),
            I8 => _data_to_i8(),
            U16 => _data_to_u16(),
            I16 => _data_to_i16(),
            U32 => _data_to_u32(),
            I32 => _data_to_i32(),
            F32 => _data_to_f32(),
            U64 => _data_to_u64(),
            I64 => _data_to_i64(),
            F64 => _data_to_f64(),
            U128 => _data_to_u128(),
            I128 => _data_to_i128(),
            Usize => _data_to_usize(),
            Isize => _data_to_isize(),
            Char => _data_to_char(),
            Bool => _data_to_bool(),
            Str => _data_to_str(),
            Slice(_) => _data_to_slice(),
            Tuple(elements) => {
                for element in elements {
                    s += &Self::generate_u8_to_other_type_function(element);
                }
                s.as_str()
            }
        }
        .to_string()
    }
}

fn _data_to_u8() -> &'static str {
    "fn _to_u8(data:&[u8], index:usize)->u8 {
    data[index]
}\n"
}

fn _data_to_i8() -> &'static str {
    "fn _to_i8(data:&[u8], index:usize)->i8 {    
    data[index] as i8
}\n"
}

fn _data_to_u16() -> &'static str {
    "fn _to_u16(data:&[u8], index:usize)->u16 {
    let data0 = _to_u8(data, index) as u16;
    let data1 = _to_u8(data, index+1) as u16;
    data0 << 8 | data1
}\n"
}

fn _data_to_i16() -> &'static str {
    "fn _to_i16(data:&[u8], index:usize)->i16 {
    let data0 = _to_i8(data, index) as i16;
    let data1 = _to_i8(data, index+1) as i16;
    data0 << 8 | data1
}\n"
}

fn _data_to_u32() -> &'static str {
    "fn _to_u32(data:&[u8], index:usize)->u32 {
    let data0 = _to_u16(data, index) as u32;
    let data1 = _to_u16(data, index+2) as u32;
    data0 << 16 | data1
}\n"
}

fn _data_to_i32() -> &'static str {
    "fn _to_i32(data:&[u8], index:usize)->i32 {
    let data0 = _to_i16(data, index) as i32;
    let data1 = _to_i16(data, index+2) as i32;
    data0 << 16 | data1
}\n"
}

fn _data_to_f32() -> &'static str {
    "fn _to_f32(data:&[u8], index: usize) -> f32 {
    let data_slice = &data[index..index+4];
    use std::convert::TryInto;
    let data_array:[u8;4] = data_slice.try_into().expect(\"slice with incorrect length\");
    f32::from_le_bytes(data_array)
}\n"
}

fn _data_to_u64() -> &'static str {
    "fn _to_u64(data:&[u8], index:usize)->u64 {
    let data0 = _to_u32(data, index) as u64;
    let data1 = _to_u32(data, index+4) as u64;
    data0 << 32 | data1
}\n"
}

fn _data_to_i64() -> &'static str {
    "fn _to_i64(data:&[u8], index:usize)->i64 {
    let data0 = _to_i32(data, index) as i64;
    let data1 = _to_i32(data, index+4) as i64;
    data0 << 32 | data1
}\n"
}

fn _data_to_f64() -> &'static str {
    "fn _to_f64(data:&[u8], index: usize) -> f64 {
    let data_slice = &data[index..index+8];
    use std::convert::TryInto;
    let data_array:[u8;8] = data_slice.try_into().expect(\"slice with incorrect length\");
    f64::from_le_bytes(data_array)
}\n"
}

fn _data_to_u128() -> &'static str {
    "fn _to_u128(data:&[u8], index:usize)->u128 {
    let data0 = _to_u64(data, index) as u128;
    let data1 = _to_u64(data, index+8) as u128;
    data0 << 64 | data1
}\n"
}

fn _data_to_i128() -> &'static str {
    "fn _to_i128(data:&[u8], index:usize)->i128 {
    let data0 = _to_i64(data, index) as i128;
    let data1 = _to_i64(data, index+8) as i128;
    data0 << 64 | data1
}\n"
}

fn _data_to_usize() -> &'static str {
    "fn _to_usize(data:&[u8], index:usize)->usize {
    _to_u64(data, index) as usize
}\n"
}

fn _data_to_isize() -> &'static str {
    "fn _to_isize(data:&[u8], index:usize)->isize {
    _to_i64(data, index) as isize
}\n"
}

fn _data_to_char() -> &'static str {
    "fn _to_char(data:&[u8], index: usize)->char {
    let char_value = _to_u32(data,index);
    match char::from_u32(char_value) {
        Some(c)=>c,
        None=>{
            use std::process;
            process::exit(0);
        }
    }
}\n"
}

fn _data_to_bool() -> &'static str {
    "fn _to_bool(data:&[u8], index: usize)->bool {
    let bool_value = _to_u8(data, index);
    if bool_value %2 == 0 {
        true
    } else {
        false
    }
}\n"
}

pub fn _data_to_str() -> &'static str {
    "fn _to_str(data:&[u8], start_index: usize, end_index: usize)->&str {
    let data_slice = &data[start_index..end_index];
    use std::str;
    match str::from_utf8(data_slice) {
        Ok(s)=>s,
        Err(_)=>{
            use std::process;
            process::exit(0);
        }
    }
}\n"
}

pub fn _data_to_slice() -> &'static str {
    "fn _to_slice<T>(data:&[u8], start_index: usize, end_index: usize)->&[T] {
    let data_slice = &data[start_index..end_index];
    let (_, shorts, _) = unsafe {data_slice.align_to::<T>()};
    shorts
}\n"
}

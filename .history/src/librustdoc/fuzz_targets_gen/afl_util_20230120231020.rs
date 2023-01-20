use crate::fuzz_targets_gen::fuzz_type::FuzzType;
use rustc_data_structures::fx::FxHashSet;

/// afl会随机给一个类型为`&[u8]`的随机数据data，我们需要通过这个来生成辅助函数
pub trait AflTypeHelper {
    /// 获取类型的名称，比如i32, slice，用于生成对应的转换函数声明和使用
    fn get_type_name(&self) -> String;

    /// 生成u8类型的slice到其他类型的转换函数的名字，比如 `_to_i64`
    fn generate_u8_to_other_type_function_name(&self) -> String;

    /// 生成转换函数内容，比如 `fn _to_i64(...){...}`
    fn generate_u8_to_other_type_function(&self) -> String;
}

impl AflTypeHelper for FuzzType {
    /// 获取类型名称，Tuple是空
    fn get_type_name(&self) -> String {
        match self {
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
            FuzzType::Str => "str",
            FuzzType::Slice(_) => "slice",
            FuzzType::Tuple(_) => {
                // FIXME:
                ""
            }
        }
        .to_string()
    }

    /// 获得函数签名，比如to_u32
    fn generate_u8_to_other_type_function_name(&self) -> String {
        use FuzzType::*;
        match self {
            Slice(inner) => {
                //不考虑内部还是slice或者str的情况
                //tuple里面也不会出现slice或者
                let inner_type_name = inner.get_type_name();
                format!(
                    "_to_{slice_type_name}::<{inner_type_name}>",
                    slice_type_name = self.get_type_name(),
                    inner_type_name = inner_type_name
                )
            }

            Tuple(_) => {
                //不需要，因为内部的都会被一一生成
                String::new()
            }
            _ => {
                format!("_to_{type_name}", type_name = self.get_type_name())
            }
        }
    }

    ///生成函数
    fn generate_u8_to_other_type_function(&self) -> String {
        let mut s = String::default();
        use FuzzType::*;
        match self {
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

/// 流程如下：
/// `[tys]`
/// -> get_all_dependency_fuzz_types 求出依赖
/// -> `[all_tys]`
/// -> collect_all_unique_fuzz_types
/// -> `[map]` -> generate_all_u8_to_other_type_functions
/// -> `result`
#[allow(dead_code)]
struct AflForType {
    //原始的类型集合
    tys: Vec<FuzzType>,

    //包含依赖的类型集合，比如Tuple(i32,i64)依赖i32和i64，所以需要生成对应的辅助函数
    all_tys: Vec<FuzzType>,

    //all_tys去重后的集合
    map: FxHashSet<FuzzType>,
}

#[allow(dead_code)]
impl AflForType {
    pub fn generate(tys: Vec<FuzzType>) -> String {
        let x = Self::new(tys);
        x.generate_helper_functions()
    }

    fn new(tys: Vec<FuzzType>) -> Self {
        AflForType { tys, all_tys: Vec::new(), map: FxHashSet::default() }
    }

    /// pub函数，用来获取对应的参数
    fn generate_helper_functions(&mut self) -> String {
        self.get_all_dependency_fuzz_types();
        self.collect_all_unique_fuzz_types();
        self.generate_all_u8_to_other_type_functions()
    }

    /// 对某一个类型，获得它的生成函数依赖的类型
    fn get_dependency_fuzz_type(ty: &FuzzType) -> Vec<FuzzType> {
        let mut types = Vec::new();

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
                for ele in eles {
                    types.append(&mut Self::get_dependency_fuzz_type(ele));
                }
            }
        }
        types
    }

    fn get_all_dependency_fuzz_types(&mut self) {
        for ty in &self.tys {
            //因为tuple是由内部元素组成的，它不需要生成函数
            if let FuzzType::Tuple(_) = ty {
                continue;
            }
            let mut vec = Self::get_dependency_fuzz_type(&ty);
            self.all_tys.append(&mut vec);
        }
    }

    //进行去重操作
    fn collect_all_unique_fuzz_types(&mut self) {
        for ty in &self.tys {
            if !self.map.contains(&ty) {
                self.map.insert(ty.clone());
            }
        }
    }

    fn generate_all_u8_to_other_type_functions(&self) -> String {
        //遍历，如果有对应类型，就不用再生成了
        let mut res = String::default();

        let mut have_slice = false;
        for ty in &self.all_tys {
            res += &match ty {
                FuzzType::Slice(_) if !have_slice => {
                    have_slice = true;
                    ty.generate_u8_to_other_type_function()
                }
                FuzzType::Slice(_) | FuzzType::Tuple(_) => {
                    //Tuple什么也不要做
                    String::default()
                }
                _ => ty.generate_u8_to_other_type_function(),
            }
        }
        res
    }
}

#[allow(dead_code)]
trait AflForParams {
    fn generate_param_initial_statement(
        &self,
        param_index: usize,
        fixed_start_index: usize,
        dynamic_start_index: usize,
        dynamic_param_index: usize,
        total_dynamic_param_numbers: usize,
        dynamic_param_length: &String,
    ) -> String;

    fn generate_param_initial_rhs(
        &self,
        fixed_start_index: usize,
        dynamic_start_index: usize,
        dynamic_param_index: usize,
        total_dynamic_param_numbers: usize,
        dynamic_param_length: &String,
    ) -> String;
}

impl AflForParams for FuzzType {
    fn generate_param_initial_statement(
        &self,
        param_index: usize,
        fixed_start_index: usize,
        dynamic_start_index: usize,
        dynamic_param_index: usize,
        total_dynamic_param_numbers: usize,
        dynamic_param_length: &String,
    ) -> String {
        let rhs = self.generate_param_initial_rhs(
            fixed_start_index,
            dynamic_start_index,
            dynamic_param_index,
            total_dynamic_param_numbers,
            dynamic_param_length,
        );
        format!("let _param{param_index} = {rhs};", param_index = param_index, rhs = rhs)
    }

    fn generate_param_initial_rhs(
        &self,
        fixed_start_index: usize,
        dynamic_start_index: usize,
        dynamic_param_index: usize,
        total_dynamic_param_numbers: usize,
        dynamic_param_length: &String,
    ) -> String {
        use FuzzType::*;
        match self {
            Bool | U8 | I8 | U16 | I16 | U32 | I32 | Char | U64 | I64 | U128 | I128 | Usize
            | Isize | F32 | F64 => {
                format!(
                    "{afl_helper_function_name}(data, {fixed_start_index})",
                    afl_helper_function_name = self.generate_u8_to_other_type_function_name(),
                    fixed_start_index = fixed_start_index
                )
            }
            Str | Slice(..) => {
                let latter_index = if dynamic_param_index == total_dynamic_param_numbers - 1 {
                    format!("data.len()")
                } else {
                    format!(
                        "{dynamic_start_index} + {dynamic_param_index} * {dynamic_param_length}",
                        dynamic_start_index = dynamic_start_index,
                        dynamic_param_index = dynamic_param_index + 1,
                        dynamic_param_length = dynamic_param_length
                    )
                };
                format!(
                    "{afl_helper_function_name}(data, {dynamic_start_index} + {dynamic_param_index} * {dynamic_param_length}, {latter_index})",
                    afl_helper_function_name = self.generate_u8_to_other_type_function_name(),
                    dynamic_start_index = dynamic_start_index,
                    dynamic_param_index = dynamic_param_index,
                    dynamic_param_length = dynamic_param_length,
                    latter_index = latter_index
                )
            }
            Tuple(inners) => {
                let mut res = "(".to_string();
                let inner_afl_helpers_number = inners.len();

                let mut inner_fixed_start_index = fixed_start_index;
                let mut inner_dynamic_param_index = dynamic_param_index;
                for i in 0..inner_afl_helpers_number {
                    if i != 0 {
                        res.push_str(", ");
                    }

                    let inner = inners[i];

                    let inner_rhs = inner.generate_param_initial_rhs(
                        inner_fixed_start_index,
                        dynamic_start_index,
                        inner_dynamic_param_index,
                        total_dynamic_param_numbers,
                        dynamic_param_length,
                    );
                    res.push_str(inner_rhs.as_str());
                    inner_fixed_start_index = inner_fixed_start_index + inner.fixed_part_size();
                    inner_dynamic_param_index =
                        inner_dynamic_param_index + inner._dynamic_length_param_number();
                }
                res.push_str(")");
                res
            }
        }
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

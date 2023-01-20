use crate::fuzz_targets_gen::fuzz_type::FuzzType;
use rustc_data_structures::fx::FxHashSet;

trait AflHelper {
    fn get_type_name(&self);
    fn get_afl_helper_function_name(&self);
    fn generate_all_helper_function(&self) -> String;
}

#[allow(dead_code)]
struct AflForType {
    tys: Vec<FuzzType>,

    all_tys: Vec<FuzzType>,

    //用来去重
    map: FxHashSet<FuzzType>,
}

#[allow(dead_code)]
impl AflForType {
    pub fn new(tys: Vec<FuzzType>) -> Self {
        AflForType { tys, all_tys: Vec::new(), map: FxHashSet::default() }
    }

    pub fn _generate_param_initial_statement(
        &self,
        param_index: usize,
        fixed_start_index: usize,
        dynamic_start_index: usize,
        dynamic_param_index: usize,
        total_dynamic_param_numbers: usize,
        dynamic_param_length: &String,
        origin_fuzzable_type: &FuzzableType,
    ) -> String {
        match self {
            NoHelper => {
                format!("No helper")
            }
            _ => {
                let rhs = self._generate_param_initial_rhs(
                    fixed_start_index,
                    dynamic_start_index,
                    dynamic_param_index,
                    total_dynamic_param_numbers,
                    dynamic_param_length,
                    origin_fuzzable_type,
                );
                format!("let _param{param_index} = {rhs};", param_index = param_index, rhs = rhs)
            }
        }
    }

    pub fn _generate_param_initial_rhs(
        &self,
        fixed_start_index: usize,
        dynamic_start_index: usize,
        dynamic_param_index: usize,
        total_dynamic_param_numbers: usize,
        dynamic_param_length: &String,
        origin_fuzzable_type: &FuzzableType,
    ) -> String {
        match self {
            Bool | U8 | I8 | U16 | I16 | U32 | I32 | Char | U64 | I64 | U128 | I128 | Usize
            | Isize | F32 | F64 => {
                format!(
                    "{afl_function_name}(data, {fixed_start_index})",
                    afl_function_name = self._to_function_name(),
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
                    "{afl_function_name}(data, {dynamic_start_index} + {dynamic_param_index} * {dynamic_param_length}, {latter_index})",
                    afl_function_name = self._to_function_name(),
                    dynamic_start_index = dynamic_start_index,
                    dynamic_param_index = dynamic_param_index,
                    dynamic_param_length = dynamic_param_length,
                    latter_index = latter_index
                )
            }
            Tuple(inner_afl_helpers) => {
                if let FuzzableType::Tuple(inner_fuzzables) = origin_fuzzable_type {
                    let mut res = "(".to_string();
                    let inner_afl_helpers_number = inner_afl_helpers.len();

                    let mut inner_fixed_start_index = fixed_start_index;
                    let mut inner_dynamic_param_index = dynamic_param_index;
                    for i in 0..inner_afl_helpers_number {
                        if i != 0 {
                            res.push_str(", ");
                        }
                        let inner_afl_helper = &inner_afl_helpers[i];
                        let inner_origin_fuzzable_type = &inner_fuzzables[i];
                        let inner_rhs = inner_afl_helper._generate_param_initial_rhs(
                            inner_fixed_start_index,
                            dynamic_start_index,
                            inner_dynamic_param_index,
                            total_dynamic_param_numbers,
                            dynamic_param_length,
                            inner_origin_fuzzable_type,
                        );
                        res.push_str(inner_rhs.as_str());
                        inner_fixed_start_index = inner_fixed_start_index
                            + inner_origin_fuzzable_type._fixed_part_length();
                        inner_dynamic_param_index = inner_dynamic_param_index
                            + inner_origin_fuzzable_type._dynamic_length_param_number();
                    }
                    res.push_str(")");
                    res
                } else {
                    "Type not match in afl_util".to_string()
                }
            }
            NoHelper => {
                format!("No helper")
            }
        }
    }

    /// pub函数，用来获取对应的参数
    pub fn generate_helper_functions(&mut self) -> String {
        self.get_all_dependency_fuzz_types();
        self.collect_all_unique_fuzz_types();
        self.generate_all_u8_to_other_type_functions()
    }

    //对某一个类型，获得它的生成函数依赖的类型
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

    fn generate_all_u8_to_other_type_functions(&self) -> String {
        //遍历，如果有对应类型，就不用再生成了
        let mut res = String::default();

        let mut have_slice = false;
        for ty in &self.all_tys {
            res += &match ty {
                FuzzType::Slice(_) if !have_slice => {
                    have_slice = true;
                    Self::generate_u8_to_other_type_function(&ty)
                }
                FuzzType::Slice(_) | FuzzType::Tuple(_) => {
                    //Tuple什么也不要做
                    String::default()
                }
                _ => Self::generate_u8_to_other_type_function(&ty),
            }
        }
        res
    }
}

#[allow(dead_code)]
struct AflForParams {
    ty: FuzzType,
}

impl AflForParams {
    pub fn generate_param_initial_statement(
        &self,
        ty: FuzzType,
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
            origin_fuzzable_type,
        );
        format!("let _param{param_index} = {rhs};", param_index = param_index, rhs = rhs)
    }

    pub fn generate_param_initial_rhs(
        &self,
        fixed_start_index: usize,
        dynamic_start_index: usize,
        dynamic_param_index: usize,
        total_dynamic_param_numbers: usize,
        dynamic_param_length: &String,
        origin_fuzzable_type: &FuzzableType,
    ) -> String {
        use FuzzType::*;
        match self.ty {
            Bool | U8 | I8 | U16 | I16 | U32 | I32 | Char | U64 | I64 | U128 | I128 | Usize
            | Isize | F32 | F64 => {
                format!(
                    "{afl_function_name}(data, {fixed_start_index})",
                    afl_function_name = self._to_function_name(),
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
                    "{afl_function_name}(data, {dynamic_start_index} + {dynamic_param_index} * {dynamic_param_length}, {latter_index})",
                    afl_function_name = self._to_function_name(),
                    dynamic_start_index = dynamic_start_index,
                    dynamic_param_index = dynamic_param_index,
                    dynamic_param_length = dynamic_param_length,
                    latter_index = latter_index
                )
            }
            Tuple(inner_afl_helpers) => {
                if let FuzzableType::Tuple(inner_fuzzables) = origin_fuzzable_type {
                    let mut res = "(".to_string();
                    let inner_afl_helpers_number = inner_afl_helpers.len();

                    let mut inner_fixed_start_index = fixed_start_index;
                    let mut inner_dynamic_param_index = dynamic_param_index;
                    for i in 0..inner_afl_helpers_number {
                        if i != 0 {
                            res.push_str(", ");
                        }
                        let inner_afl_helper = &inner_afl_helpers[i];
                        let inner_origin_fuzzable_type = &inner_fuzzables[i];
                        let inner_rhs = inner_afl_helper._generate_param_initial_rhs(
                            inner_fixed_start_index,
                            dynamic_start_index,
                            inner_dynamic_param_index,
                            total_dynamic_param_numbers,
                            dynamic_param_length,
                            inner_origin_fuzzable_type,
                        );
                        res.push_str(inner_rhs.as_str());
                        inner_fixed_start_index = inner_fixed_start_index
                            + inner_origin_fuzzable_type._fixed_part_length();
                        inner_dynamic_param_index = inner_dynamic_param_index
                            + inner_origin_fuzzable_type._dynamic_length_param_number();
                    }
                    res.push_str(")");
                    res
                } else {
                    "Type not match in afl_util".to_string()
                }
            }
            NoHelper => {
                format!("No helper")
            }
        }
    }

    fn get_helper_function_name(&self) -> String {
        match self.ty {
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

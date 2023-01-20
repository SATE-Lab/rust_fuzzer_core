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

pub fn _data_to_i64() -> &'static str {
    "fn _to_i64(data:&[u8], index:usize)->i64 {
    let data0 = _to_i32(data, index) as i64;
    let data1 = _to_i32(data, index+4) as i64;
    data0 << 32 | data1
}\n"
}

pub fn _data_to_f64() -> &'static str {
    "fn _to_f64(data:&[u8], index: usize) -> f64 {
    let data_slice = &data[index..index+8];
    use std::convert::TryInto;
    let data_array:[u8;8] = data_slice.try_into().expect(\"slice with incorrect length\");
    f64::from_le_bytes(data_array)
}\n"
}

pub fn _data_to_u128() -> &'static str {
    "fn _to_u128(data:&[u8], index:usize)->u128 {
    let data0 = _to_u64(data, index) as u128;
    let data1 = _to_u64(data, index+8) as u128;
    data0 << 64 | data1
}\n"
}

pub fn _data_to_i128() -> &'static str {
    "fn _to_i128(data:&[u8], index:usize)->i128 {
    let data0 = _to_i64(data, index) as i128;
    let data1 = _to_i64(data, index+8) as i128;
    data0 << 64 | data1
}\n"
}

pub fn _data_to_usize() -> &'static str {
    "fn _to_usize(data:&[u8], index:usize)->usize {
    _to_u64(data, index) as usize
}\n"
}

pub fn _data_to_isize() -> &'static str {
    "fn _to_isize(data:&[u8], index:usize)->isize {
    _to_i64(data, index) as isize
}\n"
}

pub fn _data_to_char() -> &'static str {
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

pub fn _data_to_bool() -> &'static str {
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

//会有big endian和 little endian的问题，不过只是去fuzz的话，应该没啥影响
pub fn _data_to_slice() -> &'static str {
    "fn _to_slice<T>(data:&[u8], start_index: usize, end_index: usize)->&[T] {
    let data_slice = &data[start_index..end_index];
    let (_, shorts, _) = unsafe {data_slice.align_to::<T>()};
    shorts
}\n"
}

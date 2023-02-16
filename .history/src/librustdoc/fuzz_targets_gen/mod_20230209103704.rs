mod afl_function_util;
mod afl_param_util;
mod api_sequence;
mod api_util;
mod call_type;
mod context;
mod extract_dep;
mod extract_seq;
mod function;
mod fuzz_type;
mod prelude_type;

pub(crate) use context::Context;

#[macro_use]
extern crate lazy_static;

//use crate::fuzz_targets_gen::afl_param_util;

use super::afl_param_util::{AflForParams, AflForType};
use super::api_sequence::ApiSequence;
use super::fuzz_type::FuzzType;
use rustc_data_structures::fx::FxHashSet;

/// 生成代测试文件
#[allow(dead_code)]
struct AflFunctionHelper<'tcx> {
    api_seq: ApiSequence<'tcx>,
    fuzz_params: Vec<FuzzType>,
    pub fuzz_param_mut_tag: FxHashSet<usize>, //表示哪些fuzzable的变量需要带上mut标记
    pub _unsafe_tag: bool,                    //标志这个调用序列是否需要加上unsafe标记
}
#[allow(dead_code)]
impl AflFunctionHelper<'_> {
    fn is_fuzzable_need_mut_tag(&self, i: usize) -> bool {
        self.fuzz_param_mut_tag.contains(&i)
    }

    /// 生成AFL可以接收的测试文件
    pub fn generate_afl_file(&self, test_index: usize) -> String {
        let mut res = self.generate_afl_except_main(test_index);
        res.push_str(self.generate_afl_main(test_index).as_str());
        res
    }

    /// 生成afl测试文件中除了main以外的部分，包括：
    /// 1. 导入外部crate
    /// 2. 生
    /// 3. afl类型转换函数
    /// 4. 真正的测试函数
    pub fn generate_afl_except_main(&self, test_index: usize) -> String {
        let mut res = String::new();
        //加入可能需要开启的feature gate
        /*let feature_gates = afl_util::_get_feature_gates_of_sequence(&self.fuzzable_params);

        if feature_gates.is_some() {
            for feature_gate in &feature_gates.unwrap() {
                let feature_gate_line = format!("{feature_gate}\n", feature_gate = feature_gate);
                res.push_str(feature_gate_line.as_str());
            }
        }*/

        res.push_str("#[macro_use]\n");
        res.push_str("extern crate afl;\n");
        res.push_str(format!("extern crate {};\n", self.api_seq.crate_name).as_str());

        /*let prelude_helper_functions = self._prelude_helper_functions();
        if let Some(prelude_functions) = prelude_helper_functions {
            res.push_str(prelude_functions.as_str());
        }*/

        let all_helper_function_definition_string = AflForType::generate(self.fuzz_params.clone());
        res.push_str(all_helper_function_definition_string.as_str());

        res.push_str(self.generate_test_function(test_index, 0).as_str());
        res.push('\n');
        res
    }

    /// 关键部分，生成测试函数，其中包含了需要被调用的序列！
    pub fn generate_test_function(&self, test_index: usize, indent_size: usize) -> String {
        let mut res = String::new();

        let param_prefix_str = "_param";
        //let local_prefix_str = "_local";

        //生成对trait的引用
        //let using_traits = self.generate_using_traits_string(indent_size);
        //res.push_str(using_traits.as_str());

        //生成函数头
        let function_header =
            self.generate_function_header(test_index, indent_size, param_prefix_str);
        res.push_str(function_header.as_str());

        //加入函数体开头的大括号
        res.push_str("{\n");

        /*
        //加入函数体
        if self._unsafe_tag {
            let unsafe_indent = generate_indent(indent_size + 1);
            res.push_str(unsafe_indent.as_str());
            res.push_str("unsafe {\n");

            let unsafe_function_body =
                self.generate_function_body_string(indent_size + 4, param_prefix, local_prefix);
            res.push_str(unsafe_function_body.as_str());

            res.push_str(unsafe_indent.as_str());
            res.push_str("}\n");
        } else {
            let function_body =
                self.generate_function_body_string(indent_size, param_prefix, local_prefix);
            res.push_str(function_body.as_str());
        }
        */

        //加入函数体结尾的大括号
        let indent = generate_indent(indent_size);
        res.push_str(format!("{indent}}}\n", indent = indent).as_str());

        res
    }

    /// 生成fn test_functioni(param1: xxx, ...)
    fn generate_function_header(
        &self,
        test_index: usize,
        tab_num: usize,
        param_prefix_str: &str,
    ) -> String {
        let indent = generate_indent(tab_num);

        //生成具体的函数签名
        let mut res = String::new();

        //      fn test_function2(
        res.push_str(
            format!(
                "{indent}fn test_function{test_index} (",
                indent = indent,
                test_index = test_index
            )
            .as_str(),
        );

        //加入所有的fuzzable变量
        //第一个参数特殊处理?
        for (i, param) in self.fuzz_params.iter().enumerate() {
            if i != 0 {
                res.push_str(", ");
            }
            if self.is_fuzzable_need_mut_tag(i) {
                res.push_str("mut ");
            }
            res.push_str(param_prefix_str);
            res.push_str(i.to_string().as_str());
            res.push_str(": ");
            res.push_str(param.get_type_string().as_str());
        }
        res.push_str(") ");
        res
    }

    pub fn generate_function_body_string(
        &self,
        outer_indent: usize,
        param_prefix: &str,
        local_param_prefix: &str,
    ) -> String {
        let mut res = String::new();

        let body_indent = generate_indent(outer_indent + 1);

        //let dead_code = self._dead_code();

        //api_calls
        for (i, api_call) in self.api_seq.sequence.iter().enumerate() {
            //准备参数

            let mut param_strings = Vec::new();
            for param in api_call.params {
                let call_type_array = param.call_type._split_at_unwrap_call_type();
                //println!("call_type_array = {:?}",call_type_array);

                // 比如： _param0 _local1
                let param_ident_string = param.get_param_string(param_prefix, local_param_prefix);
                let call_type_array_len = call_type_array.len();

                if call_type_array_len == 1 {
                    // 如果只需要一次转换，那就直接...

                    let call_type = &call_type_array[0];
                    let param_string = call_type._to_call_string(&param_name, full_name_map);
                    param_strings.push(param_string);
                } else {
                    let mut former_param_name = param_name.clone();
                    let mut helper_index = 1;
                    let mut former_helper_line = String::new();
                    for k in 0..call_type_array_len - 1 {
                        let call_type = &call_type_array[k];
                        let helper_name = format!(
                            "{}{}_param{}_helper{}",
                            local_param_prefix, i, j, helper_index
                        );
                        let helper_line = format!(
                            "{}let mut {} = {};\n",
                            body_indent,
                            helper_name,
                            call_type._to_call_string(&former_param_name, full_name_map)
                        );
                        if helper_index > 1 {
                            if !api_util::_need_mut_tag(call_type) {
                                former_helper_line = former_helper_line.replace("let mut ", "let ");
                            }
                            res.push_str(former_helper_line.as_str());
                        }
                        helper_index = helper_index + 1;
                        former_param_name = helper_name;
                        former_helper_line = helper_line;
                    }
                    let last_call_type = call_type_array.last().unwrap();
                    if !api_util::_need_mut_tag(last_call_type) {
                        former_helper_line = former_helper_line.replace("let mut ", "let ");
                    }
                    res.push_str(former_helper_line.as_str());
                    let param_string =
                        last_call_type._to_call_string(&former_param_name, full_name_map);
                    param_strings.push(param_string);
                }
            }
            res.push_str(body_indent.as_str());
            //如果不是最后一个调用
            let api_function_index = api_call.func.1;
            let api_function = &_api_graph.api_functions[api_function_index];
            if dead_code[i] || api_function._has_no_output() {
                res.push_str("let _ = ");
            } else {
                let mut_tag = if self._is_function_need_mut_tag(i) { "mut " } else { "" };
                res.push_str(format!("let {}{}{} = ", mut_tag, local_param_prefix, i).as_str());
            }
            let (api_type, function_index) = &api_call.func;
            match api_type {
                ApiType::BareFunction => {
                    let api_function_full_name =
                        &_api_graph.api_functions[*function_index].full_name;
                    res.push_str(api_function_full_name.as_str());
                }
            }

            //生成括号里的实在参数：(_param1, _local1)
            res.push('(');

            for (k, param_string) in param_strings.iter().enumerate() {
                if k != 0 {
                    res.push_str(" ,");
                }
                res.push_str(param_string.as_str());
            }
            res.push_str(");\n");
        }
        res
    }

    /// afl的main函数
    /// fn main() {
    ///     fuzz!(|data: &[u8]|{
    ///         ...
    ///     });
    /// }
    #[allow(dead_code)]
    pub fn generate_afl_main(&self, test_index: usize) -> String {
        let mut res = String::new();
        let indent = generate_indent(1);
        res.push_str("fn main() {\n");
        res.push_str(indent.as_str());
        res.push_str("fuzz!(|data: &[u8]| {\n");
        res.push_str(self.generate_afl_closure(1, test_index).as_str());
        res.push_str(indent.as_str());
        res.push_str("});\n");
        res.push_str("}\n");
        res
    }

    /// 闭包中的函数体，主要用来初始化参数和调用test_function_i `fuzz!(|data: &[u8]|{...});`
    //#[allow(dead_code)]
    fn generate_afl_closure(&self, outer_tab: usize, test_index: usize) -> String {
        let mut res = String::new();
        let indent = generate_indent(outer_tab + 1);

        res.push_str(format!("{indent}//actual body emit\n", indent = indent).as_str());

        // 获取test_function所有参数的最小的长度
        let min_len = self.fuzz_params_min_length();
        res.push_str(
            format!(
                "{indent}if data.len() < {min_len} {{return;}}\n",
                indent = indent,
                min_len = min_len
            )
            .as_str(),
        );

        //固定部分结束的地方就是动态部分开始的地方
        let dynamic_param_start_index = self.fuzz_params_fixed_size_part_length();
        let fuzz_params_dynamic_size_parts_number = self.fuzz_params_dynamic_size_parts_number();
        let dynamic_length_name = "dynamic_length";

        // let dynamic_length = (data.len() - dynamic_param_param_start_index) / dynamic_param_number
        // 计算每个dynamic part的长度！！！
        let every_dynamic_length = format!(
            "let {dynamic_length_name} = (data.len() - {dynamic_param_start_index}) / {fuzz_params_dynamic_size_parts_number}",
            dynamic_length_name = dynamic_length_name,
            dynamic_param_start_index = dynamic_param_start_index,
            fuzz_params_dynamic_size_parts_number = fuzz_params_dynamic_size_parts_number
        );

        if !self.is_all_fuzz_params_fixed_size() {
            res.push_str(
                format!(
                    "{indent}{every_dynamic_length};\n",
                    indent = indent,
                    every_dynamic_length = every_dynamic_length
                )
                .as_str(),
            );
        }

        //每次迭代固定长度开始的位置
        let mut fixed_start_index = 0;
        //当前这是第几个动态长度的变量
        let mut dynamic_index = 0;

        for (i, param_type) in self.fuzz_params.iter().enumerate() {
            let param_init_statement = param_type.generate_param_initial_statement(
                i,
                fixed_start_index,
                dynamic_param_start_index,
                dynamic_index,
                fuzz_params_dynamic_size_parts_number,
                &dynamic_length_name.to_string(),
            );
            res.push_str(
                format!(
                    "{indent}{param_init_statement}\n",
                    indent = indent,
                    param_init_statement = param_init_statement
                )
                .as_str(),
            );
            fixed_start_index += param_type.fixed_size_part_size();
            dynamic_index += param_type.dynamic_size_parts_number();
        }

        //参数初始化完成，可以调用test_function
        let mut test_function_call =
            format!("{indent}test_function{test_index}(", indent = indent, test_index = test_index);
        for i in 0..self.fuzz_params.len() {
            if i != 0 {
                test_function_call.push_str(" ,");
            }
            test_function_call.push_str(format!("_param{}", i).as_str());
        }
        test_function_call.push_str(");\n");
        res.push_str(test_function_call.as_str());

        res
    }

    fn fuzz_params_min_length(&self) -> usize {
        let mut min_length = 0;
        for param_type in &self.fuzz_params {
            min_length += param_type.min_size();
        }
        min_length
    }

    fn fuzz_params_fixed_size_part_length(&self) -> usize {
        let mut fixed_size_part_length = 0;
        for param_type in &self.fuzz_params {
            fixed_size_part_length += param_type.fixed_size_part_size();
        }
        fixed_size_part_length
    }

    fn fuzz_params_dynamic_size_parts_number(&self) -> usize {
        let mut num = 0;
        for param_type in &self.fuzz_params {
            num += param_type.dynamic_size_parts_number();
        }
        num
    }

    fn is_all_fuzz_params_fixed_size(&self) -> bool {
        self.fuzz_params.iter().all(|x| x.is_fixed_size())
    }
}

/// 生成每行代码前面的空格
fn generate_indent(tab_num: usize) -> String {
    let mut indent = String::new();
    for _ in 0..tab_num {
        indent.push_str("    ");
    }
    indent
}

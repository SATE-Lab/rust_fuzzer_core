//use crate::fuzz_targets_gen::afl_param_util;

/// 生成代测试文件
#[allow(dead_code)]
struct AflFunctionHelper {}

impl AflFunctionHelper {
    pub fn generate_main() -> String {
        "".to_string()
    }

    fn generate_main_closure(outer_tab: i32) -> String {
        let extra_indent = 4;
        let mut res = String::new();
        let indent = generate_indent(outer_indent + 1);
        res.push_str(format!("{indent}//actual body emit\n", indent = indent).as_str());

        let op = if self._is_fuzzables_fixed_length() { "!=" } else { "<" };
        let min_len = self._fuzzables_min_length();
        res.push_str(
            format!(
                "{indent}if data.len() {op} {min_len} {{return;}}\n",
                indent = indent,
                op = op,
                min_len = min_len
            )
            .as_str(),
        );

        let dynamic_param_start_index = self._fuzzable_fixed_part_length();
        let dynamic_param_number = self._dynamic_length_param_number();
        let dynamic_length_name = "dynamic_length";
        let every_dynamic_length = format!(
            "let {dynamic_length_name} = (data.len() - {dynamic_param_start_index}) / {dynamic_param_number}",
            dynamic_length_name = dynamic_length_name,
            dynamic_param_start_index = dynamic_param_start_index,
            dynamic_param_number = dynamic_param_number
        );
        if !self._is_fuzzables_fixed_length() {
            res.push_str(
                format!(
                    "{indent}{every_dynamic_length};\n",
                    indent = indent,
                    every_dynamic_length = every_dynamic_length
                )
                .as_str(),
            );
        }

        let mut fixed_start_index = 0; //当前固定长度的变量开始分配的位置
        let mut dynamic_param_index = 0; //当前这是第几个动态长度的变量

        let fuzzable_param_number = self.fuzzable_params.len();
        for i in 0..fuzzable_param_number {
            let fuzzable_param = &self.fuzzable_params[i];
            let afl_helper = _AflHelpers::_new_from_fuzzable(fuzzable_param);
            let param_initial_line = afl_helper._generate_param_initial_statement(
                i,
                fixed_start_index,
                dynamic_param_start_index,
                dynamic_param_index,
                dynamic_param_number,
                &dynamic_length_name.to_string(),
                fuzzable_param,
            );
            res.push_str(
                format!(
                    "{indent}{param_initial_line}\n",
                    indent = indent,
                    param_initial_line = param_initial_line
                )
                .as_str(),
            );
            fixed_start_index = fixed_start_index + fuzzable_param._fixed_part_length();
            dynamic_param_index =
                dynamic_param_index + fuzzable_param._dynamic_length_param_number();
        }

        let mut test_function_call =
            format!("{indent}test_function{test_index}(", indent = indent, test_index = test_index);
        for i in 0..fuzzable_param_number {
            if i != 0 {
                test_function_call.push_str(" ,");
            }
            test_function_call.push_str(format!("_param{}", i).as_str());
        }
        test_function_call.push_str(");\n");
        res.push_str(test_function_call.as_str());

        res
    }
}

fn generate_indent() -> String {
    let mut indent = String::new();
    for _ in 0..indent_size {
        indent.push(' ');
    }
    indent
}

//use crate::fuzz_targets_gen::afl_param_util;

use super::afl_param_util::AflForParams;
use super::fuzz_type::FuzzType;

/// 生成代测试文件
#[allow(dead_code)]
struct AflFunctionHelper {
    fuzz_param_types: Vec<FuzzType>,
}

impl AflFunctionHelper {
    pub fn generate_main() -> String {
        "".to_string()
    }

    fn generate_main_closure(&self, outer_tab: usize) -> String {
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

        let fuzz_params_number = self.fuzz_param_types.len();
        for (i, param_type) in self.fuzz_param_types.iter().enumerate() {
            let param_statement = param_type.generate_param_initial_statement(
                i,
                fixed_start_index,
                dynamic_param_start_index,
                dynamic_index,
                dynamic_param_number,
                &dynamic_length_name.to_string(),
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

    fn fuzz_params_min_length(&self) -> usize {
        let min_length = 0;
        for param_type in self.fuzz_param_types {
            min_length += param_type.min_size();
        }
        min_length
    }

    fn fuzz_params_fixed_size_part_length(&self) -> usize {
        let fixed_size_part_length = 0;
        for param_type in self.fuzz_param_types {
            fixed_size_part_length += param_type.fixed_size_part_size();
        }
        fixed_size_part_length
    }

    fn fuzz_params_dynamic_size_parts_number(&self) -> usize {
        let num = 0;
        for param_type in self.fuzz_param_types {
            num += param_type.dynamic_size_parts_number();
        }
        num
    }

    fn is_all_fuzz_params_fixed_size(&self) -> bool {
        self.fuzz_param_types.iter().all(|x| x.is_fixed_size())
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

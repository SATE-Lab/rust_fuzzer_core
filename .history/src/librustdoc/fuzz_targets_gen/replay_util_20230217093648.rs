pub fn _read_crash_file_data() -> &'static str {
    "fn _read_data()-> Vec<u8> {
        use std::env;
        use std::process::exit;
        // 解析参数: args[0]是该程序name, args[1]是crash_file的路径
        let args:Vec<String> = env::args().collect();
        if args.len() < 2 {
            println!(\"No crash filename provided\");
            exit(-1);
        }
        use std::path::PathBuf;
        let crash_file_name = &args[1];
        let crash_path = PathBuf::from(crash_file_name);
        if !crash_path.is_file() {
            println!(\"Not a valid crash file\");
            exit(-1);
        }
        use std::fs;
        let data = fs::read(crash_path).unwrap();
        data
    }\n"
}

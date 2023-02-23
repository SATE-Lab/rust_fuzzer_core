use crate::fuzz_targets_gen::api_graph::ApiGraph;
use rustc_data_structures::fx::FxHashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

lazy_static! {
    static ref CRATE_TEST_DIR: FxHashMap<&'static str, &'static str> = {
        let mut m = FxHashMap::default();
        m.insert("url", "/Users/yxz/workspace/fuzz/fuzz_dir/url_afl_work");
        m.insert("regex_syntax", "/Users/yxz/workspace/fuzz/fuzz_dir/regex-syntax-afl-work");
        m.insert("semver_parser", "/Users/yxz/workspace/fuzz/fuzz_dir/semver-parser-afl-work");
        m.insert("bat", "/Users/yxz/workspace/fuzz/fuzz_dir/bat-afl-work");
        m.insert("xi_core_lib", "/Users/yxz/workspace/fuzz/fuzz_dir/xi-core-lib-afl-work");
        m.insert("proc_macro2", "/Users/yxz/workspace/fuzz/fuzz_dir/proc-macro2-afl-work");
        m.insert("clap", "/Users/yxz/workspace/fuzz/fuzz_dir/clap-afl-work");
        m.insert("regex", "/Users/yxz/workspace/fuzz/fuzz_dir/regex-afl-work");
        m.insert("serde_json", "/Users/yxz/workspace/fuzz/fuzz_dir/serde-json-afl-work");
        m.insert("tui", "/Users/yxz/workspace/fuzz/fuzz_dir/tui-afl-work");
        m.insert("semver", "/Users/yxz/workspace/fuzz/fuzz_dir/semver-afl-work");
        m.insert("http", "/Users/yxz/workspace/fuzz/fuzz_dir/http-afl-work");
        m.insert("flate2", "/Users/yxz/workspace/fuzz/fuzz_dir/flate2-afl-work");
        m.insert("time", "/Users/yxz/workspace/fuzz/fuzz_dir/time-afl-work");

        //fudge-like-directories
        //m.insert("fudge_like_url", "/home/jjf/fudge_like_work/url-work");
        //m.insert("fudge_like_regex", "/home/jjf/fudge_like_work/regex-work");
        //m.insert("fudge_like_time", "/home/jjf/fudge_like_work/time-work");

        //fudge-directories
        //m.insert("fudge_regex", "/home/jjf/fudge_work/regex-work");
        //m.insert("fudge_url", "/home/jjf/fudge_work/url-work");
        //m.insert("fudge_time", "/home/jjf/fudge_work/time-work");
        m
    };
}

lazy_static! {
    static ref RANDOM_TEST_DIR: FxHashMap<&'static str, &'static str> = {
        let mut m = FxHashMap::default();
        m.insert("regex", "/Users/yxz/workspace/fuzz/random_work/regex-work");
        m.insert("url", "/Users/yxz/workspace/fuzz/random_work/url-work");
        m.insert("time", "/Users/yxz/workspace/fuzz/random_work/time-work");
        m
    };
}

lazy_static! {
    static ref LIBFUZZER_FUZZ_TARGET_DIR: FxHashMap<&'static str, &'static str> = {
        let mut m = FxHashMap::default();
        m.insert("url", "/Users/yxz/workspace/fuzz/libfuzzer_work/url-libfuzzer-targets");
        m.insert(
            "regex_syntax",
            "/Users/yxz/workspace/fuzz/libfuzzer_work/regex-syntax-libfuzzer-targets",
        );
        m.insert("syn", "/Users/yxz/workspace/fuzz//libfuzzer_work/syn-libfuzzer-targets");
        m.insert("semver_parser", "/Users/yxz/workspace/fuzz/libfuzzer_work/sem-libfuzzer-targets");
        m
    };
}

lazy_static! {
    static ref RANDOM_TEST_FILE_NUMBERS: FxHashMap<&'static str, usize> = {
        let mut m = FxHashMap::default();
        m.insert("url", 61);
        m.insert("regex", 67);
        m.insert("time", 118);
        m
    };
}

static _AFL_DIR: &'static str = "afl_files";
static _REPRODUCE_FILE_DIR: &'static str = "replay_files";
static _LIBFUZZER_DIR: &'static str = "libfuzzer_files";
static MAX_TEST_FILE_NUMBER: usize = 300;
static DEFAULT_RANDOM_FILE_NUMBER: usize = 100;

pub(crate) fn can_write_to_file(crate_name: &String, random_strategy: bool) -> bool {
    if !random_strategy && CRATE_TEST_DIR.contains_key(crate_name.as_str()) {
        return true;
    }

    if random_strategy && RANDOM_TEST_DIR.contains_key(crate_name.as_str()) {
        return true;
    }

    return false;
}

pub(crate) fn can_generate_libfuzzer_target(crate_name: &String) -> bool {
    if LIBFUZZER_FUZZ_TARGET_DIR.contains_key(crate_name.as_str()) {
        return true;
    } else {
        return false;
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FileHelper {
    pub(crate) crate_name: String,
    pub(crate) test_dir: String,
    pub(crate) test_files: Vec<String>,
    pub(crate) reproduce_files: Vec<String>,
    pub(crate) libfuzzer_files: Vec<String>,
}

impl FileHelper {
    /// 进行初始化工作
    pub(crate) fn new(api_graph: &ApiGraph<'_>, random_strategy: bool) -> Self {
        let crate_name = api_graph._crate_name.clone();

        //按照不同策略生成在不同的文件夹里
        let test_dir = if !random_strategy {
            CRATE_TEST_DIR.get(crate_name.as_str()).unwrap().to_string()
        } else {
            RANDOM_TEST_DIR.get(crate_name.as_str()).unwrap().to_string()
        };
        println!("test_dir is [{}]", test_dir);
        let mut sequence_count = 0;
        let mut test_files = Vec::new();
        let mut reproduce_files = Vec::new();
        let mut libfuzzer_files = Vec::new();
        //let chosen_sequences = api_graph._naive_choose_sequence(MAX_TEST_FILE_NUMBER);
        let chosen_sequences = if !random_strategy {
            api_graph._heuristic_choose(MAX_TEST_FILE_NUMBER, true)
        } else {
            let random_size = if RANDOM_TEST_FILE_NUMBERS.contains_key(crate_name.as_str()) {
                (RANDOM_TEST_FILE_NUMBERS.get(crate_name.as_str()).unwrap()).clone()
            } else {
                DEFAULT_RANDOM_FILE_NUMBER
            };
            api_graph._first_choose(random_size)
        };
        //println!("chosen sequences number: {}", chosen_sequences.len());

        for sequence in &chosen_sequences {
            if sequence_count >= MAX_TEST_FILE_NUMBER {
                break;
            }
            let test_file = sequence._to_afl_test_file(api_graph, sequence_count);
            test_files.push(test_file);
            let reproduce_file = sequence._to_replay_crash_file(api_graph, sequence_count);
            reproduce_files.push(reproduce_file);
            let libfuzzer_file = sequence._to_libfuzzer_test_file(api_graph, sequence_count);
            libfuzzer_files.push(libfuzzer_file);
            sequence_count = sequence_count + 1;
        }
        FileHelper { crate_name, test_dir, test_files, reproduce_files, libfuzzer_files }
    }

    pub(crate) fn write_files(&self) {
        let test_path = PathBuf::from(&self.test_dir);
        if test_path.is_file() {
            fs::remove_file(&test_path).unwrap();
        }
        let test_file_path = test_path.clone().join(_AFL_DIR);
        ensure_empty_dir(&test_file_path);
        let reproduce_file_path = test_path.clone().join(_REPRODUCE_FILE_DIR);
        ensure_empty_dir(&reproduce_file_path);

        write_to_files(&self.crate_name, &test_file_path, &self.test_files, "test");
        //暂时用test file代替一下，后续改成真正的reproduce file
        write_to_files(&self.crate_name, &reproduce_file_path, &self.reproduce_files, "replay");
    }

    pub(crate) fn write_libfuzzer_files(&self) {
        let libfuzzer_dir = LIBFUZZER_FUZZ_TARGET_DIR.get(self.crate_name.as_str()).unwrap();
        let libfuzzer_path = PathBuf::from(libfuzzer_dir);
        if libfuzzer_path.is_file() {
            fs::remove_file(&libfuzzer_path).unwrap();
        }
        let libfuzzer_files_path = libfuzzer_path.join(_LIBFUZZER_DIR);
        ensure_empty_dir(&libfuzzer_files_path);
        write_to_files(
            &self.crate_name,
            &libfuzzer_files_path,
            &self.libfuzzer_files,
            "fuzz_target",
        );
    }
}

// 每个contents[i]的内容，写入文件【prefix_cratenamei.rs】
fn write_to_files(crate_name: &String, path: &PathBuf, contents: &Vec<String>, prefix: &str) {
    let file_number = contents.len();
    for i in 0..file_number {
        let filename = format!("{}_{}{}.rs", prefix, crate_name, i);
        let full_filename = path.join(filename);
        let mut file = fs::File::create(full_filename).unwrap();
        file.write_all(contents[i].as_bytes()).unwrap();
    }
}

//在创建之前先删掉原来的文件内容
fn ensure_empty_dir(path: &PathBuf) {
    if path.is_file() {
        fs::remove_file(path).unwrap();
    }
    if path.is_dir() {
        fs::remove_dir_all(path).unwrap();
    }
    fs::create_dir_all(path).unwrap();
}

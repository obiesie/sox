use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use regex::Regex;


lazy_static::lazy_static! {
    static ref EXPECTED_OUTPUT_PATTERN: Regex = Regex::new(r"// expect: ?(.*)").unwrap();
    static ref EXPECTED_ERROR_PATTERN: Regex = Regex::new(r"// (Error.*)").unwrap();
    static ref ERROR_LINE_PATTERN: Regex = Regex::new(r"// \[((java|c) )?line (\d+)\] (Error.*)").unwrap();
    static ref EXPECTED_RUNTIME_ERROR_PATTERN: Regex = Regex::new(r"// expect runtime error: (.+)").unwrap();
    static ref SYNTAX_ERROR_PATTERN: Regex = Regex::new(r"\[.*line (\d+)\] (Error.+)").unwrap();
    static ref STACK_TRACE_PATTERN: Regex = Regex::new(r"\[line (\d+)\]").unwrap();
    static ref NON_TEST_PATTERN: Regex = Regex::new(r"// nontest").unwrap();
}

static PATHS_TESTS_TO_RUN: [&str; 1] = ["tests/while/return_closure.sox"];

fn fetch_all_test_suite() -> Vec<String>{
    todo!()
}
#[test]
fn test_compiler(){
    let mut test_paths = vec![];
    if PATHS_TESTS_TO_RUN.is_empty(){
        test_paths = fetch_all_test_suite();
    } else{
        test_paths = PATHS_TESTS_TO_RUN.iter().map(|v| v.to_string()).collect::<Vec<String>>();
    }
    for test_path in test_paths {
        let hay = fs::read_to_string(test_path.as_str())
            .expect("Failed to read file at {test_path}");
        let caps = EXPECTED_OUTPUT_PATTERN.captures_iter(hay.as_str());
        let error_caps = SYNTAX_ERROR_PATTERN.captures_iter(hay.as_str());

        let mut expected_outputs = vec![];
        let mut expected_error_outputs = vec![];

        for cap in caps {
            let expected_output = cap.get(1).unwrap().as_str();
            expected_outputs.push(expected_output);
        }

        for error_cap in error_caps{
            let t = error_cap.get(0).unwrap().as_str();
            expected_error_outputs.push(format!("{}", t));
        }
        let run_output = Command::new("target/debug/sox")
            .arg(test_path)
            .output().unwrap();

        let output = String::from_utf8_lossy(&run_output.stdout);
        let output_strs = output.split("\n").filter(|v| *v != "").collect::<Vec<&str>>();
        if !expected_outputs.is_empty() {
            assert_eq!(expected_outputs, output_strs);
        } else if !expected_error_outputs.is_empty() {
            assert_eq!(expected_error_outputs, output_strs);
        }
    }
}
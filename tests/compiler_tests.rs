use std::fs;
use std::iter::zip;
use std::process::Command;

use regex::Regex;
use walkdir::WalkDir;

lazy_static::lazy_static! {
    static ref EXPECTED_OUTPUT_PATTERN: Regex = Regex::new(r"// expect: ?(.*)").unwrap();
    static ref EXPECTED_ERROR_PATTERN: Regex = Regex::new(r"// (Error.*)").unwrap();
    static ref ERROR_LINE_PATTERN: Regex = Regex::new(r"// \[((java|c) )?line (\d+)\] (Error.*)").unwrap();
    static ref EXPECTED_RUNTIME_ERROR_PATTERN: Regex = Regex::new(r"// expect runtime error: (.+)").unwrap();
    static ref SYNTAX_ERROR_PATTERN: Regex = Regex::new(r"\[.*line (\d+)\] (Error.+)").unwrap();
    static ref STACK_TRACE_PATTERN: Regex = Regex::new(r"\[line (\d+)\]").unwrap();
    static ref NON_TEST_PATTERN: Regex = Regex::new(r"// nontest").unwrap();
}


static TEST_SUITES: [&str; 1] = ["bool"];

#[test]
fn test_compiler(){
    let mut test_paths = vec![];
    for suite in TEST_SUITES {
        for entry in WalkDir::new(format!("tests/{suite}")) {
            match entry {
                Ok(entry) => {
                    if entry.metadata().unwrap().is_file() {
                        test_paths.push(entry.path().to_string_lossy().to_string());
                        // println!("{}", entry.path().display())
                    }
                },
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }

   println!("Paths are {:?}", test_paths);
    for test_path in test_paths {
        // if test_path != "tests/assignment/global.sox".to_string(){
        //     continue;
        // } 
        println!("test_path={}", test_path);
        let hay = fs::read_to_string(test_path.to_string())
            .expect("Failed to read file at {test_path}");
        let caps = EXPECTED_OUTPUT_PATTERN.captures_iter(hay.as_str());
        let syntax_error_caps = SYNTAX_ERROR_PATTERN.captures_iter(hay.as_str());
        let runtime_error_caps = EXPECTED_RUNTIME_ERROR_PATTERN.captures_iter(hay.as_str());
        let mut expected_outputs = vec![];

        for cap in caps {
            let expected_output = cap.get(1).unwrap().as_str();
            expected_outputs.push(expected_output.to_string());
        }

        for error_cap in syntax_error_caps {
            let t = error_cap.get(0).unwrap().as_str();
            expected_outputs.push(format!("{}", t));
        }
        
        for runtime_error_cap in runtime_error_caps {
            let inst = runtime_error_cap.get(0).unwrap().as_str();
            expected_outputs.push(format!("{}", inst));
        }
        let run_output = Command::new("target/debug/sox")
            .arg(test_path)
            .output().unwrap();

        let output = String::from_utf8_lossy(&run_output.stdout);
        let output_strs = output.split("\n").filter(|v| *v != "").map(|v| v.to_string()).collect::<Vec<String>>();
        let failures = validate_outputs(expected_outputs, &output_strs);
       
        assert_eq!(failures, vec![]);
    }
}


fn validate_outputs<T: ToString + PartialEq>(expected_outputs: Vec<T>, outputs: &Vec<T>) -> Vec<(String, String)>{
    let mut failures = vec![];
    for (expected_output, output) in zip(expected_outputs, outputs){
        if expected_output != *output {
            failures.push((expected_output.to_string(), output.to_string()));
        }
    }
    failures
}
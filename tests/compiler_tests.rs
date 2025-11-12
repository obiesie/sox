use polars::df;
use polars::frame::DataFrame;
use polars::prelude::*;
use regex::Regex;
use std::fs;
use std::iter::zip;
use std::process::Command;
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
static ALL_TEST_SUITES: [&str; 17] = [
    "assignment",
    "block",
    "bool",
    "call",
    "class",
    "closure",
    "for",
    "function",
    "if",
    "number",
    "operator",
    "print",
    "while",
    "closure",
    "comments",
    "constructors",
    "logical_operator",
];
static TEST_SUITES: [&str; 1] = ["function"];

const SOX_EXECUTABLE: &str = "target/debug/sox";
const RESULT_CSV_PATH: &str = "result.csv";
const TEST_PATH_COL: &str = "Test Path";
const TEST_PASSED_COL: &str = "Test Passed?";

#[test]
fn test_compiler() {
    let test_paths = get_test_paths();

    let results: Vec<(String, bool)> = test_paths
        .iter()
        .map(|path| (path.clone(), run_and_validate_test(path)))
        .collect();

    let test_paths_series: Vec<String> = results.iter().map(|(path, _)| path.clone()).collect();
    let test_passed_series: Vec<bool> = results.iter().map(|(_, passed)| *passed).collect();

    let mut res_df: DataFrame = df!(
        TEST_PATH_COL => &test_paths_series,
        TEST_PASSED_COL => &test_passed_series,
    )
        .unwrap();

    println!("{}", res_df);

    let mut file = std::fs::File::create(RESULT_CSV_PATH).unwrap();
    CsvWriter::new(&mut file).finish(&mut res_df).unwrap();

    let failed_df = res_df
        .lazy()
        .filter(col(TEST_PASSED_COL).eq(lit(false)))
        .collect()
        .unwrap();

    if failed_df.shape().0 > 0 {
        println!("failed tests: \n {}", failed_df);
    }

    assert_eq!(failed_df.shape().0, 0, "Some tests failed");
}

fn get_test_paths() -> Vec<String> {
    let test_suites = if TEST_SUITES.is_empty() {
        ALL_TEST_SUITES.as_ref()
    } else {
        TEST_SUITES.as_ref()
    };

    let mut test_paths = Vec::new();
    for suite in test_suites {
        for entry in WalkDir::new(format!("tests/{}", suite)) {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        test_paths.push(entry.path().to_string_lossy().to_string());
                    }
                }
                Err(e) => eprintln!("Error walking directory: {}", e),
            }
        }
    }
    test_paths
}

fn run_and_validate_test(test_path: &str) -> bool {
    let content =
        fs::read_to_string(test_path).unwrap_or_else(|_| panic!("Failed to read file at {}", test_path));
    let expected_outputs = extract_expected_outputs(&content);

    let run_output = Command::new(SOX_EXECUTABLE)
        .arg(test_path)
        .output()
        .expect("Failed to execute sox command");

    let output = String::from_utf8_lossy(&run_output.stdout);
    let output_strs = output
        .lines()
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .collect::<Vec<String>>();

    let failures = validate_outputs(&expected_outputs, &output_strs);
    if !failures.is_empty() {
        println!("Failures for {}: {:?}", test_path, failures);
    }
    failures.is_empty()
}

fn extract_expected_outputs(content: &str) -> Vec<String> {
    let expected = EXPECTED_OUTPUT_PATTERN
        .captures_iter(content)
        .map(|cap| cap.get(1).unwrap().as_str().to_string());
    let syntax_errors = SYNTAX_ERROR_PATTERN
        .captures_iter(content)
        .map(|cap| cap.get(0).unwrap().as_str().to_string());
    let runtime_errors = EXPECTED_RUNTIME_ERROR_PATTERN
        .captures_iter(content)
        .map(|cap| cap.get(1).unwrap().as_str().to_string());
    expected.chain(syntax_errors).chain(runtime_errors).collect()
}

fn validate_outputs<T: ToString + PartialEq>(
    expected_outputs: &Vec<T>,
    outputs: &Vec<T>,
) -> Vec<(String, String)> {
    let mut failures = vec![];
    for (expected_output, output) in zip(expected_outputs, outputs) {
        if *expected_output != *output {
            failures.push((expected_output.to_string(), output.to_string()));
        }
    }
    failures
}

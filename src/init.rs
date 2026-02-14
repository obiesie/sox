use crate::runtime::Runtime;
use std::io::Write;
use std::{fs, io};

pub fn run_file(file_path: String) {
    let contents =
        fs::read_to_string(file_path).expect("Failed to read content of provided file path");
    run(contents)
}

pub fn run_prompt() {
    let stdin = io::stdin();
    let mut runtime = Runtime::new();

    println!("Welcome to sox");

    loop {
        print!(">>> ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut buffer = String::new();

        if stdin.read_line(&mut buffer).expect("Failed to read line") == 0 {
            break;
        }
        let static_buffer = buffer.trim().to_string().leak();
        runtime.interpret(static_buffer);
    }
}

pub fn run(source: String) {
    let mut runtime = Runtime::new();
    let static_source = source.leak();
    runtime.interpret(static_source);
}

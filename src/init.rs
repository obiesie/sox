use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::resolver::Resolver;
use std::io::Write;
use std::{fs, io};

pub fn run_file(file_path: String) {
    let contents =
        fs::read_to_string(file_path).expect("Failed to read content of provided file path");
    run(contents, true)
}

pub fn run_prompt() {
    let stdin = io::stdin();
    let mut interpreter = Interpreter::new();
    println!("Welcome to sox");

    loop {
        print!(">>> ");
        let _ = io::stdout().flush();
        let mut buffer = String::new();
        stdin.read_line(&mut buffer).unwrap();
        if buffer.is_empty() {
            break;
        }
        let tokens = Lexer::lex(buffer.as_str());
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        if let Ok(ast) = ast {
            interpreter.interpret(&ast);
        } else {
            println!("Error - {:?}", ast.err().unwrap());
        }
    }
}

pub fn run(source: String, enable_var_resolution: bool) {
    let tokens = Lexer::lex(source.as_str());
    let mut parser = Parser::new(tokens);
    let mut var_resolver = Resolver::new();

    let ast = parser.parse();

    let mut interpreter = Interpreter::new();

    if ast.is_ok() {
        if enable_var_resolution {
            let resolved_data = var_resolver.resolve(&ast.as_ref().unwrap());
            

            interpreter._locals = resolved_data.unwrap();
        }
        interpreter.interpret(&ast.unwrap())
    }
}

use std::{fs};
use crate::environment::{StoreMode};
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::resolver::Resolver;

pub fn run_file(file_path: String) {
    let contents =
        fs::read_to_string(file_path).expect("Failed to read content of provided file path");
    run(contents, true, StoreMode::Vec)
}

pub fn run_prompt() {
   todo!()
}

pub fn run(source: String, enable_var_resolution: bool, env_store_mode: StoreMode) {
    let tokens = Lexer::lex(source.as_str());
    let mut parser = Parser::new(tokens);
    let mut var_resolver = Resolver::new();

    let ast = parser.parse();

    let mut interpreter = Interpreter::new(env_store_mode);

    if ast.is_ok() {
        if enable_var_resolution {
            let resolved_data = var_resolver.resolve(&ast.as_ref().unwrap());
            interpreter.locals = Some(resolved_data.unwrap());
            interpreter.interpret(&ast.unwrap());

        } else {
            interpreter.interpret(&ast.unwrap());
        }
    }
}

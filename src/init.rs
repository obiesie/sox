use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::resolver::Resolver;
use std::io::Write;
use std::{fs, io};
use crate::vm::chunk::Chunk;
use crate::vm::vm::VirtualMachine;

pub fn run_file(file_path: String) {
    let contents =
        fs::read_to_string(file_path).expect("Failed to read content of provided file path");
    run(contents)
}

pub fn run_prompt() {
    let stdin = io::stdin();
    let mut interpreter = Interpreter::new();
    let resolver = Resolver::new();
    println!("Welcome to sox");

    loop {
        print!(">>> ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut buffer = String::new();

        if stdin.read_line(&mut buffer).expect("Failed to read line") == 0 {
            break;
        }
        let static_buffer = buffer.trim().to_string().leak();
        interpret_with_vm(static_buffer, &mut interpreter);
        //parse_and_interpret_with_resolver(static_buffer, &mut resolver, &mut interpreter);
    }
}

pub fn run(source: String) {
    let var_resolver = Resolver::new();
    let mut interpreter = Interpreter::new();
    let static_source = source.leak();
    interpret_with_vm(static_source, &mut interpreter);
    //parse_and_interpret_with_resolver(static_source, &mut var_resolver, &mut interpreter);
}

fn parse_and_interpret_with_resolver(
    source: &'static str,
    resolver: &mut Resolver,
    interpreter: &mut Interpreter,
) {
    let tokens = Lexer::lex(source);
    let mut parser = Parser::new(tokens);
    let ast = parser.parse();

    match ast {
        Ok(ast) => match resolver.resolve(&ast) {
            Ok(data) => {
                interpreter.locals = data;
                interpreter.interpret(&ast);
            }
            Err(e) => {
                println!("Resolution error: {}", e);
            }
        },
        Err(e) => {
            println!("Parsing error: {:?}", e);
        }
    }
}


fn interpret_with_vm(source: &'static str, interpreter: &mut Interpreter) {
    let chunk = Chunk::default();
    let mut vm = VirtualMachine::new(chunk);
    vm.interpret(interpreter, source);
}

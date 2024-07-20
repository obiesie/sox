use std::io;
use std::io::Write;
use std::{env, fs, process};

use log::LevelFilter;

use sox::interpreter::Interpreter;
use sox::lexer::Lexer;
use sox::parser::Parser;
use sox::resolver::Resolver;

fn main() {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Debug)
        .init();

    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        println!("Usage: sox [script]");
        // 64 is the exit code used when args passed to a script are incorrect
        process::exit(64);
    } else if args.len() == 2 {
        run_file(args.get(1).unwrap().to_string());
    } else {
        run_prompt();
    }
}

fn run_file(file_path: String) {
    let contents =
        fs::read_to_string(file_path).expect("Failed to read content of provided file path");
    run(contents, true)
}

fn run_prompt() {
    let stdin = io::stdin();
    println!("Welcome to sox");

    loop {
        print!("> ");
        let _ = io::stdout().flush();
        let mut buffer = String::new();
        stdin.read_line(&mut buffer).unwrap();
        if buffer.is_empty() {
            break;
        }
        run(buffer, false);
    }
}

fn run(source: String, enable_var_resolution: bool) {
    let tokens = Lexer::lex(source.as_str());
    let mut parser = Parser::new(tokens);
    let mut var_resolver = Resolver::new();
    
    let ast = parser.parse();
    
    let mut interpreter = Interpreter::new();

    if ast.is_ok() {
        if enable_var_resolution{
            let resolved_data = var_resolver.resolve(&ast.as_ref().unwrap());
            
            interpreter.locals = resolved_data.unwrap();
        } 
        interpreter.interpret(&ast.unwrap())
    }
}

use crate::interpreter::Interpreter;
use crate::vm::vm::VirtualMachine;
use std::io::Write;
use std::{fs, io};

pub fn run_file(file_path: String) {
    let contents =
        fs::read_to_string(file_path).expect("Failed to read content of provided file path");
    run(contents)
}

pub fn run_prompt() {
    let stdin = io::stdin();
    let mut vm = VirtualMachine::new();
    let interpreter = Interpreter::new();

    println!("Welcome to sox");

    loop {
        print!(">>> ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut buffer = String::new();

        if stdin.read_line(&mut buffer).expect("Failed to read line") == 0 {
            break;
        }
        let static_buffer = buffer.trim().to_string().leak();
        vm.interpret(&interpreter, static_buffer);
    }
}

pub fn run(source: String) {
    let mut interpreter = Interpreter::new();
    let static_source = source.leak();
    interpret_with_vm(static_source, &mut interpreter);
}

fn interpret_with_vm(source: &'static str, interpreter: &mut Interpreter) {
    let mut vm = VirtualMachine::new();
    vm.interpret(interpreter, source);
}

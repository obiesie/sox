use crate::builtins::bool::SoxBool;
use crate::interpreter::Interpreter;
use crate::object::core::{SoxObjectRef, SoxRef};
use crate::vm::chunk::{Chunk, OpCode};
use crate::vm::compiler::Compiler;


macro_rules! read_instr {
    ($a:expr) => {{
        let instruction = $a.chunk.code[$a.ip];
        let opcode: OpCode = instruction.try_into().unwrap();
        $a.ip += 1;
        opcode
    }};
}

macro_rules! read_constant {
    ($a:expr) => {{
        let instruction = read_instr!($a);
        let constant = $a.chunk.constants[instruction as usize].clone();
        constant
    }};
}

macro_rules! pop_stack {
    ($a:expr) => {{
        $a.stack_top -= 1;
        let v = $a.stack.pop().unwrap();
        v
    }};
}

macro_rules! binary_op {
    ($vm:expr, $interpreter:expr, $slot_op:ident, $slot_name:ident) => {{
        let b = pop_stack!($vm);
        let a = pop_stack!($vm);
        let operation = a.typ().slots.$slot_name.as_ref().unwrap().$slot_op.unwrap();
        let res = (operation)(a, b, $interpreter).unwrap();
        push_stack!($vm, res);
    }};
}

macro_rules! push_stack {
    ($a:expr, $b:expr) => {{
        $a.stack.push( $b );
        $a.stack_top += 1;
    }};
}
pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

pub struct VirtualMachine {
    chunk: Chunk,
    ip: usize,
    stack: Vec<SoxObjectRef>,
    stack_top: usize,
}

impl VirtualMachine {
    pub fn new(chunk: Chunk) -> VirtualMachine {
        VirtualMachine {
            chunk,
            ip: 0,
            stack: Vec::with_capacity(256),
            stack_top: 0,
        }
    }

    pub fn interpret(&mut self, i : &Interpreter, source: &'static str)  {
        let mut compiler = Compiler::new();
        let chunk = compiler.compile(source, i);
        chunk.and_then(|chunk| {
            self.chunk = chunk;
            self.run(i);
            let value = pop_stack!(self);
            let repr_str = value.repr(i);
            println!("{}", repr_str.unwrap().as_str());

            Ok(())
        }).expect("Error executing bytecode.");
    }
    
    
    pub fn run(&mut self, i : &Interpreter) {
        while self.ip < self.chunk.code.len() {
            self.chunk.disassemble_instruction(self.ip);
            let inst = read_instr!(self);
            match inst {
                OpCode::OpAdd => {
                    binary_op!(self, i, add, number);
                    
                }
                OpCode::OpSubtract => {
                    binary_op!(self, i, minus, number);
                }
                OpCode::OpMultiply => {
                    binary_op!(self, i, star, number);
                }
                OpCode::OpDivide => {
                    binary_op!(self, i, slash, number);
                }
                OpCode::OpReturn => {
                    let val = pop_stack!(self);
                    println!("return value is {:?}", val.repr(i).unwrap());
                    break;
                }
                OpCode::OpConstant => {
                    let constant = read_constant!(self);
                    push_stack!(self, constant);
                    continue;
                }
                OpCode::OpNegate => {
                    let val = pop_stack!(self);
                    let neg_op = val.typ().slots.number.as_ref().unwrap().neg.unwrap();
                    let res = (neg_op)(val, i).unwrap();
                    push_stack!(self, res);
                    continue;
                },
                OpCode::OpNone => {
                    push_stack!(self, SoxObjectRef::from(i.none.clone()))
                } 
                OpCode::OpTrue => {
                    push_stack!(self, SoxObjectRef::from(SoxRef::new_ref(SoxBool::from(true), i.types.bool_type.to_owned())))
                } 
                OpCode::OpFalse => {
                    push_stack!(self, SoxObjectRef::from(SoxRef::new_ref(SoxBool::from(false), i.types.bool_type.to_owned())))
                }
                OpCode::OpNot => {
                    let val = pop_stack!(self);
                    let bool_val = val.try_into_rust_bool(i);
                    let not_val = SoxObjectRef::from(SoxRef::new_ref(SoxBool::new(!bool_val), i.types.bool_type.to_owned()));
                    push_stack!(self, not_val);
                    continue; 
                }
                OpCode::OpEqual => {
                    binary_op!(self, i, eq, comparable)
                }
                OpCode::OpGreater => {
                    binary_op!(self, i, gt, comparable)
                }
                OpCode::OpLess => {
                    binary_op!(self, i, lt, comparable)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_run() {
        let interpreter = Interpreter::new();
        let chunk = Chunk::test_chunk(&interpreter);
        let mut vm = VirtualMachine::new(chunk);
        vm.run(&interpreter);;
    }
}

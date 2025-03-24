use std::collections::HashMap;
use crate::builtins::bool::SoxBool;
use crate::interpreter::Interpreter;
use crate::object::core::{SoxObjectRef, SoxRef};
use crate::vm::chunk::{Chunk, OpCode};
use crate::vm::compiler::Compiler;
use std::mem;
use crate::builtins::string::SoxString;

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

macro_rules! read_short {
    ($a:expr) => {{
        $a.ip += 2;
        let byte1 = $a.chunk.code[$a.ip - 2];
        let byte2 = $a.chunk.code[$a.ip - 1];
        (byte1 as u16) << 8 | byte2 as u16
    }}; 
}

macro_rules! pop_stack {
    ($a:expr) => {{
        if $a.stack_top == 0 {
            panic!("Stack underflow.");
        }
        $a.stack_top -= 1;
        let v = $a.stack.pop().unwrap();
        v
    }};
}

macro_rules! peek_stack {
    ($a:expr) => {{
        if $a.stack_top == 0 {
            panic!("Stack underflow.");
        }
        let v = $a.stack.last().unwrap();
        *v
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
        $a.stack.push($b);
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
    globals: HashMap<String, SoxObjectRef>,
}

impl VirtualMachine {
    pub fn new() -> VirtualMachine {
        let chunk = Chunk::default();
        VirtualMachine {
            chunk,
            ip: 0,
            stack: Vec::with_capacity(256),
            stack_top: 0,
            globals: HashMap::new(),
        }
    }

    pub fn interpret(&mut self, i: &Interpreter, source: &'static str) {
        let mut compiler = Compiler::new();
        let chunk = compiler.compile(source, mem::take(&mut self.chunk), i);
        chunk
            .and_then(|chunk| {
                self.chunk = chunk;
                self.run(i);
                if self.stack_top > 0 {
                    let value = pop_stack!(self);
                    let repr_str = value.repr(i);
                    println!("{}", repr_str.unwrap().as_str()); 
                }
                

                Ok(())
            })
            .expect("Error executing bytecode.");
    }

    pub fn run(&mut self, i: &Interpreter) {
        while self.ip < self.chunk.code.len() {
            let inst = read_instr!(self);
            match inst {
                OpCode::OpAdd => {
                    println!("Add");
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
                }
                OpCode::OpNone => {
                    push_stack!(self, SoxObjectRef::from(i.none.clone()))
                }
                OpCode::OpTrue => {
                    push_stack!(
                        self,
                        SoxObjectRef::from(SoxRef::new_ref(
                            SoxBool::from(true),
                            i.types.bool_type.to_owned()
                        ))
                    )
                }
                OpCode::OpFalse => {
                    push_stack!(
                        self,
                        SoxObjectRef::from(SoxRef::new_ref(
                            SoxBool::from(false),
                            i.types.bool_type.to_owned()
                        ))
                    )
                }
                OpCode::OpNot => {
                    let val = pop_stack!(self);
                    let bool_val = val.try_into_rust_bool(i);
                    let not_val = SoxObjectRef::from(SoxRef::new_ref(
                        SoxBool::new(!bool_val),
                        i.types.bool_type.to_owned(),
                    ));
                    push_stack!(self, not_val);
                    continue;
                }
                OpCode::OpEqual => {
                    binary_op!(self, i, eq, comparable);
                }
                OpCode::OpGreater => {
                    binary_op!(self, i, gt, comparable);
                }
                OpCode::OpLess => {
                    println!("Less");
                    binary_op!(self, i, lt, comparable);
                },
                OpCode::OpPrint => {
                    let val = pop_stack!(self);
                    let repr_str = val.repr(i);
                    println!("{}", repr_str.unwrap().as_str());
                },
                OpCode::OpPop => {
                    pop_stack!(self);
                },
                OpCode::OpDefineGlobal => {
                    let v = read_constant!(self);
                    let name = v.payload::<SoxString>().unwrap();
                    let nv = name.value.to_string();
                    self.globals.insert(nv, peek_stack!(self));
                    pop_stack!(self);
                },
                OpCode::OpGetGlobal => {
                    let v = read_constant!(self);
                    let name = v.payload::<SoxString>().unwrap(); 
                    if let Some(val) = self.globals.get(&name.value) {
                        push_stack!(self, val.clone());
                    } else{
                        panic!("Global variable {} not found", name.value);
                    }
                },
                OpCode::OpSetGlobal => {
                    let v = read_constant!(self);
                    let name = v.payload::<SoxString>().unwrap();
                    
                    if let Some(val) = self.globals.get_mut(&name.value) {
                        *val = peek_stack!(self);
                    } else {
                        panic!("Attempted to set undefined global variable {}", name.value);
                    }
                },
                OpCode::OpGetLocal => {
                    let slot = read_instr!(self);
                    let slot = slot as u8;
                    push_stack!(self, self.stack[slot as usize].clone());
                    
                },
                OpCode::OpSetLocal => {
                    let slot = read_instr!(self);
                    let slot = slot as u8;
                    self.stack[slot as usize] = peek_stack!(self);
                },
                OpCode::OpJumpIfFalse => {
                    let offset = read_short!(self);
                    let val = peek_stack!(self);
                    if !val.try_into_rust_bool(i) {
                        self.ip += offset as usize;
                    }
                },
                OpCode::OpJump => {
                    let offset = read_short!(self);
                    self.ip += offset as usize;
                },
                OpCode::OpLoop => {
                    let offset = read_short!(self);
                    self.ip -= offset as usize;
                }
            }
        }
    }
}
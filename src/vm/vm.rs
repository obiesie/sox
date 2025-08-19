use crate::builtins::bool::SoxBool;
use crate::builtins::exceptions::RuntimeError;
use crate::builtins::function::SoxFunc;
use crate::builtins::module::SoxModule;
use crate::builtins::string::SoxString;
use crate::interpreter::Interpreter;
use crate::object::core::{SoxObjectRef, SoxRef};
use crate::vm::callframe::CallFrame;
use crate::vm::chunk::{Chunk, OpCode};
use crate::vm::compiler::{CompiledUnit, CompiledUnitType, Compiler, Parser};
use std::collections::HashMap;
use std::mem;

macro_rules! read_instr {
    ($a:expr) => {{
        let frame = &mut $a.call_frame_stack[$a.call_frame_count - 1];
        let instruction = frame.co.as_ref().unwrap().code[frame.ip];
        frame.ip += 1;
        instruction
    }};
}

macro_rules! read_constant {
    ($a:expr, $i:expr) => {{
        let instruction = read_instr!($a);
        let constant = $a.call_frame_stack[$a.call_frame_count - 1]
            .co
            .as_ref()
            .unwrap()
            .constants[instruction as usize]
            .clone();
        constant
    }};
}

macro_rules! read_short {
    ($a:expr) => {{
        $a.call_frame_stack[$a.call_frame_count - 1].ip += 2;
        let frame = &mut $a.call_frame_stack[$a.call_frame_count - 1];
        let byte1 = frame.co.as_ref().unwrap().code[frame.ip - 2];
        let byte2 = frame.co.as_ref().unwrap().code[frame.ip - 1];
        (byte1 as u16) << 8 | byte2 as u16
    }};
}

macro_rules! pop_stack {
    ($a:expr) => {{
        if $a.value_stack_top == 0 {
            panic!("Stack underflow.");
        }
        $a.value_stack_top -= 1;
        let v = $a.value_stack.pop().unwrap();
        v
    }};
}

macro_rules! peek_stack {
    ($a:expr) => {{
        if $a.value_stack_top == 0 {
            panic!("Stack underflow.");
        }
        let v = $a.value_stack.last().unwrap();
        *v
    }};
    ($a:expr, $i:expr) => {{
        if $a.value_stack_top < $i {
            panic!("Stack underflow.");
        }
        let p = $a.value_stack_top - $i - 1;
        let v = $a.value_stack.get(p).unwrap();
        *v
    }};
}

macro_rules! binary_op {
    ($vm:expr, $interpreter:expr, $slot_op:ident, $slot_name:ident) => {{
        let b = pop_stack!($vm);
        let a = pop_stack!($vm);
        //println!("{:?}", a.repr($interpreter).unwrap());
        //println!("{:?}", b.repr($interpreter).unwrap());
        let operation = a.typ().slots.$slot_name.as_ref().unwrap().$slot_op.unwrap();
        let res = (operation)(a, b, $interpreter).unwrap();
        //println!("{:?}", res.repr($interpreter).unwrap());
        push_stack!($vm, res, $interpreter);
    }};
}

macro_rules! push_stack {
    ($a:expr, $b:expr, $i:expr) => {{
        //println!("Pushing {} onto the stack", $b.repr($i).unwrap());
        $a.value_stack.push($b);
        $a.value_stack_top += 1;
    }};
}
pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

pub struct VirtualMachine {
    chunk: Chunk,
    call_frame_stack: Vec<CallFrame>,
    call_frame_count: usize,
    value_stack: Vec<SoxObjectRef>,
    value_stack_top: usize,
    globals: HashMap<String, SoxObjectRef>,
    frames_max: usize,
    frame: Option<CallFrame>,
}

impl VirtualMachine {
    pub fn new() -> VirtualMachine {
        let chunk = Chunk::default();
        let mut call_frame_stack = Vec::with_capacity(64);
        for _ in 0..64 {
            let frame = CallFrame::new_frame();
            call_frame_stack.push(frame);
        }

        VirtualMachine {
            chunk,
            value_stack: Vec::with_capacity(256),
            call_frame_stack,
            call_frame_count: 0,
            value_stack_top: 0,
            globals: HashMap::new(),
            frames_max: 64,
            frame: None,
        }
    }

    fn reset_stack(&mut self) {
        self.value_stack.clear();
        self.value_stack_top = 0;
        self.call_frame_count = 0;
    }

    fn runtime_error(&mut self, message: &str) {
        eprintln!("{}", message);
        let mut idx = (self.call_frame_count - 1) as isize;
        while idx >= 0 {
            let frame = &self.call_frame_stack[idx as usize];
            let instruction = frame.ip - 1;
            let line = frame.co.as_ref().unwrap().get_line(instruction);
            let mut unit_name = frame.co.as_ref().unwrap().name.clone();
            if unit_name != "<module>".to_string() {
                unit_name.push_str("()");
            }
            eprintln!("[line {}] in {unit_name}", line);
            idx -= 1;
        }

        self.reset_stack();
    }

    pub fn interpret(&mut self, i: &Interpreter, source: &'static str) {
        // Alias improves readability throughout the function (Introduce variable).
        let interp = i;

        let parser = Parser::new(source);
        // Compile the source into a module, reusing the current chunk by moving it out (as before).
        let mut compiler = Compiler::new("__main__".to_string(), parser);
        let compiled_module = compiler.compile(source, mem::take(&mut self.chunk), interp);

        // Update VM's active chunk from the compiled module; clone once and reuse (Introduce variable, reduce duplication).
        let compiled_chunk = compiled_module.co.clone();
        self.chunk = compiled_chunk.clone();

        // Push the compiled module object onto the VM stack (Introduce variable for clarity).
        let module_obj = SoxObjectRef::from(SoxRef::new_ref(
            compiled_module,
            interp.types.mod_type.to_owned(),
        ));

        push_stack!(self, module_obj, interp);

        // Prepare and initialize a new call frame in an orderly manner (Introduce variable, organize setup).
        let frame_index = self.call_frame_count;
        let frame = &mut self.call_frame_stack[frame_index];
        self.call_frame_count += 1;
        frame.ip = 0;
        frame.co = Some(compiled_chunk);
        frame.value_stack_base_addr = self.value_stack_top;

        // Execute the VM loop for this frame.
        self.run(interp);

        // If there is a result on the stack, print its representation safely.
        if self.value_stack_top > 0 {
            let value = pop_stack!(self);
            if let Ok(repr_str) = value.repr(interp) {
                println!("{repr_str}");
            }
        }
    }

    fn call(
        &mut self,
        func: &SoxFunc,
        arg_count: usize,
        i: &Interpreter,
    ) -> Result<bool, RuntimeError> {
        if arg_count != func.arity {
            return Err(RuntimeError {
                msg: format!("Expected {:?} arguments but got {arg_count}.", func.arity),
            });
        }
        if self.call_frame_count == self.frames_max {
            return Err(RuntimeError {
                msg: "Stack overflow.".to_string(),
            });
        }
        self.call_frame_count = self.call_frame_count + 1;
        let frame = self
            .call_frame_stack
            .get_mut(self.call_frame_count - 1)
            .unwrap();
        frame.co = func.chunk.clone();
        frame.ip = 0;
        //frame.func = Some(&func);
        frame.value_stack_base_addr = self.value_stack_top - arg_count - 1;
        Ok(true)
    }
    pub fn call_value(&mut self, callee: SoxObjectRef, arg_count: usize, i: &Interpreter) -> bool {
        let c = callee.payload::<SoxFunc>();
        if c.is_some() {
            let t = c.unwrap();
            let v = self.call(t, arg_count, i);
            let r = match v {
                Ok(r) => true,
                Err(e) => {
                    self.runtime_error(&e.msg);
                    false
                }
            };
            r
        } else {
            false
        }
    }

    pub fn run(&mut self, i: &Interpreter) -> InterpretResult {
        loop {
            let instruction = read_instr!(self);
            let opcode: OpCode = instruction.try_into().unwrap();
            match opcode {
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
                    let frame = self
                        .call_frame_stack
                        .get_mut(self.call_frame_count - 1)
                        .unwrap();
                    self.call_frame_count -= 1;
                    if self.call_frame_count == 0 {
                        pop_stack!(self);
                        return InterpretResult::InterpretOk;
                    }
                    self.value_stack.truncate(frame.value_stack_base_addr);
                    self.value_stack_top = frame.value_stack_base_addr;
                    push_stack!(self, val, i);
                    continue;
                }
                OpCode::OpConstant => {
                    let constant = read_constant!(self, i);
                    push_stack!(self, constant, i);
                    continue;
                }
                OpCode::OpNegate => {
                    let val = pop_stack!(self);
                    let neg_op = val.typ().slots.number.as_ref().unwrap().neg.unwrap();
                    let res = (neg_op)(val, i).unwrap();
                    push_stack!(self, res, i);
                    continue;
                }
                OpCode::OpNone => {
                    push_stack!(self, SoxObjectRef::from(i.none.clone()), i)
                }
                OpCode::OpTrue => {
                    push_stack!(
                        self,
                        SoxObjectRef::from(SoxRef::new_ref(
                            SoxBool::from(true),
                            i.types.bool_type.to_owned()
                        )),
                        i
                    )
                }
                OpCode::OpFalse => {
                    push_stack!(
                        self,
                        SoxObjectRef::from(SoxRef::new_ref(
                            SoxBool::from(false),
                            i.types.bool_type.to_owned()
                        )),
                        i
                    )
                }
                OpCode::OpNot => {
                    let val = pop_stack!(self);
                    let bool_val = val.try_into_rust_bool(i);
                    let not_val = SoxObjectRef::from(SoxRef::new_ref(
                        SoxBool::new(!bool_val),
                        i.types.bool_type.to_owned(),
                    ));
                    push_stack!(self, not_val, i);
                    continue;
                }
                OpCode::OpEqual => {
                    binary_op!(self, i, eq, comparable);
                }
                OpCode::OpGreater => {
                    binary_op!(self, i, gt, comparable);
                }
                OpCode::OpLess => {
                    binary_op!(self, i, lt, comparable);
                }
                OpCode::OpPrint => {
                    let val = pop_stack!(self);
                    let repr_str = val.repr(i);
                    println!("{}", repr_str.unwrap().as_str());
                }
                OpCode::OpPop => {
                    pop_stack!(self);
                }
                OpCode::OpDefineGlobal => {
                    let v = read_constant!(self, i);
                    let name = v.payload::<SoxString>().unwrap();
                    let nv = name.value.to_string();
                    self.globals.insert(nv, peek_stack!(self));
                    pop_stack!(self);
                }
                OpCode::OpGetGlobal => {
                    let v = read_constant!(self, i);
                    let name = v.payload::<SoxString>().unwrap();

                    if let Some(val) = self.globals.get(&name.value) {
                        push_stack!(self, val.clone(), i);
                    } else {
                        panic!("Global variable {} not found", name.value);
                    }
                }
                OpCode::OpSetGlobal => {
                    let v = read_constant!(self, i);
                    let name = v.payload::<SoxString>().unwrap();

                    if let Some(val) = self.globals.get_mut(&name.value) {
                        *val = peek_stack!(self);
                    } else {
                        panic!("Attempted to set undefined global variable {}", name.value);
                    }
                }
                OpCode::OpGetLocal => {
                    let slot = read_instr!(self);
                    let slot = slot as u8;
                    let addr = self.call_frame_stack[self.call_frame_count - 1]
                        .relative_slot(slot as usize);
                    push_stack!(self, self.value_stack[addr].clone(), i);
                }
                OpCode::OpSetLocal => {
                    let slot = read_instr!(self);
                    let slot = slot as u8;
                    self.value_stack[self.call_frame_stack[self.call_frame_count - 1]
                        .relative_slot(slot as usize)] = peek_stack!(self);
                }
                OpCode::OpJumpIfFalse => {
                    let offset = read_short!(self);
                    let val = peek_stack!(self);
                    if !val.try_into_rust_bool(i) {
                        self.call_frame_stack[self.call_frame_count - 1].ip += offset as usize;
                    }
                }
                OpCode::OpJump => {
                    let offset = read_short!(self);
                    self.call_frame_stack[self.call_frame_count - 1].ip += offset as usize;
                }
                OpCode::OpLoop => {
                    let offset = read_short!(self);
                    self.call_frame_stack[self.call_frame_count - 1].ip -= offset as usize;
                }
                OpCode::OpCall => {
                    let arg_count = read_instr!(self) as usize;
                    if !self.call_value(peek_stack!(self, arg_count), arg_count, i) {
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
            }
        }
        InterpretResult::InterpretOk
    }
}

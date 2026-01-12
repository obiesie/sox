use crate::builtins::bool::SoxBool;
use crate::builtins::chunk::OpCode;
use crate::builtins::closure::SoxUpvalue;
use crate::builtins::exceptions::RuntimeError;
use crate::builtins::function::SoxFunction;
use crate::builtins::string::SoxString;
use crate::interpreter::Interpreter;
use crate::object::core::{SoxObjectInner, SoxObjectRef, SoxRef};
use crate::vm::callframe::CallFrame;
use crate::vm::compiler::{Compiler, Parser};
use log::info;
use std::collections::HashMap;

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
            .constants[instruction as usize];
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
        $a.value_stack.pop().expect("Stack underflow.")
    }};
}

macro_rules! peek_stack {
    ($a:expr) => {{
        *$a.value_stack.last().expect("Stack underflow.")
    }};
    ($a:expr, $i:expr) => {{
        let len = $a.value_stack.len();
        if len <= $i {
            panic!("Stack underflow.");
        }
        let p = len - $i - 1;
        $a.value_stack[p]
    }};
}

macro_rules! binary_op {
    ($vm:expr, $interpreter:expr, $slot_op:ident, $slot_name:ident) => {{
        let b = pop_stack!($vm);
        let a = pop_stack!($vm);
        let operation = a.typ().slots.$slot_name.as_ref().unwrap().$slot_op.unwrap();
        let res = (operation)(a, b, $interpreter).unwrap();
        push_stack!($vm, res, $interpreter);
    }};
}

macro_rules! push_stack {
    ($a:expr, $b:expr, $i:expr) => {{
        $a.value_stack.push($b);
    }};
}
#[derive(PartialEq)]
pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

pub struct VirtualMachine {
    call_frame_stack: Vec<CallFrame>,
    call_frame_count: usize,
    value_stack: Vec<SoxObjectRef>,
    globals: HashMap<String, SoxObjectRef>,
    frames_max: usize,
    open_upvalues: Vec<SoxObjectRef>,
}

impl VirtualMachine {
    pub fn new() -> VirtualMachine {
        let mut call_frame_stack = Vec::with_capacity(64);
        for _ in 0..64 {
            let frame = CallFrame::new_frame();
            call_frame_stack.push(frame);
        }
        let globals = HashMap::new();
        VirtualMachine {
            value_stack: Vec::with_capacity(256),
            call_frame_stack,
            call_frame_count: 0,
            globals,
            frames_max: 64,
            open_upvalues: Vec::new(),
        }
    }

    fn reset_stack(&mut self) {
        self.value_stack.clear();
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
        let parser = Parser::new(source);
        let mut compiler = Compiler::new("__main__".to_string(), parser);
        let compiled_module = match compiler.compile(i) {
            Ok(module) => module,
            Err(err) => {
                println!("{}", err);
                return;
            }
        };

        let compiled_chunk = compiled_module.co.clone();

        let module_obj = SoxObjectRef::from(SoxRef::new_ref(
            compiled_module,
            i.types.mod_type.to_owned(),
        ));

        push_stack!(self, module_obj, i);

        let frame_index = self.call_frame_count;
        let frame = &mut self.call_frame_stack[frame_index];
        self.call_frame_count += 1;
        frame.ip = 0;
        frame.co = Option::from(compiled_chunk);
        frame.value_stack_base_addr = self.value_stack.len() - 1;

        let result = self.run(i);

        if result == InterpretResult::InterpretOk && !self.value_stack.is_empty() {
            let value = pop_stack!(self);
            if let Ok(repr_str) = value.repr(i) {
                println!("{repr_str}");
            }
        }
    }

    fn call(
        &mut self,
        func: &SoxFunction,
        arg_count: usize,
        upvalues: Vec<SoxObjectRef>,
        _i: &Interpreter,
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
        info!(
            "Calling function {} with {} arguments",
            func.name, arg_count
        );
        self.call_frame_count = self.call_frame_count + 1;
        let frame = self
            .call_frame_stack
            .get_mut(self.call_frame_count - 1)
            .unwrap();
        frame.co = Some(func.chunk.clone());
        frame.ip = 0;
        frame.upvalues = upvalues;
        frame.value_stack_base_addr = self.value_stack.len() - arg_count - 1;
        Ok(true)
    }

    pub fn call_value(&mut self, callee: SoxObjectRef, arg_count: usize, i: &Interpreter) -> bool {
        let result = if let Some(func) = callee.payload::<SoxFunction>() {
            let upvalues = func.upvalues.clone();
            self.call(func, arg_count, upvalues, i)
        } else {
            let type_name = callee.typ().name.clone().unwrap_or("unknown".to_string());
            self.runtime_error(&format!("{} object is not callable.", type_name));
            return false;
        };

        match result {
            Ok(success) => success,
            Err(e) => {
                self.runtime_error(&e.msg);
                false
            }
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
                    let frame_base = frame.value_stack_base_addr;
                    self.call_frame_count -= 1;
                    if self.call_frame_count == 0 {
                        pop_stack!(self);
                        return InterpretResult::InterpretOk;
                    }
                    self.close_upvalues(frame_base, i);
                    self.value_stack.truncate(frame_base);
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
                        eprintln!("NameError: name '{}' is not defined.", name.value);
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
                OpCode::OpSetGlobal => {
                    let v = read_constant!(self, i);
                    let name = v.payload::<SoxString>().unwrap();

                    if let Some(val) = self.globals.get_mut(&name.value) {
                        *val = peek_stack!(self);
                    } else {
                        eprintln!("NameError: name '{}' is not defined.", name.value);
                        return InterpretResult::InterpretRuntimeError;
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
                    let callee = peek_stack!(self, arg_count);
                    if !self.call_value(callee, arg_count, i) {
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
                OpCode::OpGetUpvalue => {
                    let slot = read_instr!(self);
                    let upval_index = slot as usize;
                    let upval_obj = self.call_frame_stack[self.call_frame_count - 1].upvalues
                        [upval_index]
                        .payload::<SoxUpvalue>()
                        .unwrap();
                    let val = if let Some(closed) = &upval_obj.closed {
                        (**closed).clone()
                    } else {
                        self.value_stack[upval_obj.location]
                    };
                    push_stack!(self, val, i);
                }
                OpCode::OpSetUpvalue => {
                    let slot = read_instr!(self);
                    let upval_index = slot as usize;
                    let value = peek_stack!(self);
                    let upvalue_ref =
                        self.call_frame_stack[self.call_frame_count - 1].upvalues[upval_index];
                    let upvalue = unsafe {
                        &mut (*(upvalue_ref.ptr.as_ptr() as *mut SoxObjectInner<SoxUpvalue>))
                            .payload
                    };
                    if let Some(closed) = &mut upvalue.closed {
                        **closed = value;
                    } else {
                        self.value_stack[upvalue.location] = value;
                    }
                }
                OpCode::OpCloseUpvalue => {
                    self.close_upvalues(self.value_stack.len() - 1, i);
                    pop_stack!(self);
                }
                OpCode::OpClosure => {
                    let func_obj = read_constant!(self, i);
                    let func = func_obj.payload::<SoxFunction>().unwrap();
                    let mut upvalues = Vec::with_capacity(func.upvalue_count);

                    for _ in 0..func.upvalue_count {
                        let is_local = read_instr!(self);
                        let index = read_instr!(self);
                        let upvalue = if is_local == 1 {
                            let location = self.call_frame_stack[self.call_frame_count - 1]
                                .value_stack_base_addr
                                + index as usize;
                            self.capture_upvalue(location, i)
                        } else {
                            self.call_frame_stack[self.call_frame_count - 1].upvalues
                                [index as usize]
                        };
                        upvalues.push(upvalue);
                    }
                    let closure = func.with_upvalues(upvalues);
                    push_stack!(
                        self,
                        SoxObjectRef::from(SoxRef::new_ref(
                            closure,
                            i.types.function_type.to_owned()
                        )),
                        i
                    );
                }
            }
        }
    }

    pub fn close_upvalues(&mut self, value_stack_idx: usize, _: &Interpreter) {
        let mut i = 0;
        while i < self.open_upvalues.len() {
            let upvalue_ref = self.open_upvalues[i];
            let upvalue_ptr = upvalue_ref.ptr;

            let should_close = {
                let upvalue = upvalue_ref
                    .payload::<SoxUpvalue>()
                    .expect("Expected upvalue payload");
                upvalue.location >= value_stack_idx
            };

            if should_close {
                self.open_upvalues.remove(i);
                if let Some(upvalue_obj) = unsafe {
                    upvalue_ptr
                        .as_ptr()
                        .cast::<SoxObjectInner<SoxUpvalue>>()
                        .as_mut()
                } {
                    let value = self.value_stack[upvalue_obj.payload.location];
                    upvalue_obj.payload.closed = Some(Box::new(value));
                }
            } else {
                i += 1;
            }
        }
    }

    pub fn capture_upvalue(&mut self, index: usize, i: &Interpreter) -> SoxObjectRef {
        for upvalue_ref in self.open_upvalues.iter() {
            let upvalue = upvalue_ref.payload::<SoxUpvalue>().unwrap();
            if upvalue.location == index {
                return *upvalue_ref;
            }
        }

        let new_upvalue_obj = SoxUpvalue::new(index);
        let upvalue_type = i.types.upvalue_type.to_owned();
        let new_upvalue_ref = SoxObjectRef::from(SoxRef::new_ref(new_upvalue_obj, upvalue_type));

        self.open_upvalues.push(new_upvalue_ref);
        new_upvalue_ref
    }
}

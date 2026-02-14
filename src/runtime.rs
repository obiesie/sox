use crate::builtins::bool::SoxBool;
use crate::builtins::chunk::{Chunk, OpCode};
use crate::builtins::closure::SoxUpvalue;
use crate::builtins::core::{SoxObjectPayload, SoxResult, ToSoxResult};
use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::float::SoxFloat;
use crate::builtins::function::{SoxFn, SoxFunction};
use crate::builtins::int::SoxInt;
use crate::builtins::method::FuncArgs;
use crate::builtins::none::SoxNone;
use crate::builtins::r#type::{SoxInstance, SoxType};
use crate::builtins::string::SoxString;
use crate::catalog::TypeLibrary;
use crate::environment::{EnvRef, Environment};
use crate::expr::{Expr, ExprVisitor};
use crate::heap::Heap;
use crate::object::core::{SoxObjectInner, SoxObjectRef, SoxRef};
use crate::stmt::{Stmt, StmtVisitor};
use crate::token::{Literal, Token};
use crate::token_type::TokenType;
use crate::vm::callframe::CallFrame;
use crate::vm::compiler::{Compiler, Parser};
use log::info;
use std::collections::HashMap;

// VM execution macros - operate on Runtime's stack and frames
macro_rules! read_instr {
    ($rt:expr) => {{
        let frame = &mut $rt.call_frame_stack[$rt.call_frame_count - 1];
        let co = frame.co.as_ref().unwrap().payload::<Chunk>().unwrap();
        let instruction = co.code[frame.ip];
        frame.ip += 1;
        instruction
    }};
}

macro_rules! read_constant {
    ($rt:expr) => {{
        let instruction = read_instr!($rt);
        let frame = &mut $rt.call_frame_stack[$rt.call_frame_count - 1];
        let co = frame.co.as_ref().unwrap().payload::<Chunk>().unwrap();
        let constant = co.constants[instruction as usize];
        constant
    }};
}

macro_rules! read_short {
    ($rt:expr) => {{
        $rt.call_frame_stack[$rt.call_frame_count - 1].ip += 2;
        let frame = &mut $rt.call_frame_stack[$rt.call_frame_count - 1];
        let co = frame.co.as_ref().unwrap().payload::<Chunk>().unwrap();
        let byte1 = co.code[frame.ip - 2];
        let byte2 = co.code[frame.ip - 1];
        (byte1 as u16) << 8 | byte2 as u16
    }};
}

macro_rules! pop_stack {
    ($rt:expr) => {{
        $rt.stack.pop().expect("Stack underflow.")
    }};
}

macro_rules! peek_stack {
    ($rt:expr) => {{
        *$rt.stack.last().expect("Stack underflow.")
    }};
    ($rt:expr, $i:expr) => {{
        let len = $rt.stack.len();
        if len <= $i {
            panic!("Stack underflow.");
        }
        let p = len - $i - 1;
        $rt.stack[p]
    }};
}

macro_rules! binary_op {
    ($rt:expr, $slot_op:ident, $slot_name:ident) => {{
        let b = pop_stack!($rt);
        let a = pop_stack!($rt);
        let operation = a.typ().slots.$slot_name.as_ref().unwrap().$slot_op.unwrap();
        let res = (operation)(a, b, $rt).unwrap();
        push_stack!($rt, res);
    }};
}

macro_rules! push_stack {
    ($rt:expr, $val:expr) => {{
        $rt.stack.push($val);
    }};
}

#[derive(PartialEq)]
pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

/// Unified runtime state - combines Interpreter and VirtualMachine.
/// This struct holds all state needed for execution and GC.
pub struct Runtime {
    // Memory management
    pub heap: Heap,
    pub types: TypeLibrary,

    // VM execution state
    pub stack: Vec<SoxObjectRef>,
    pub call_frame_stack: Vec<CallFrame>,
    pub call_frame_count: usize,
    pub frames_max: usize,
    pub open_upvalues: Vec<SoxObjectRef>,
    pub globals: HashMap<SoxString, SoxObjectRef>,
    pub compiler: Option<Compiler>,

    // Tree-walk interpreter state
    pub environment: Environment,
    pub locals: HashMap<Token, (usize, usize)>,

    // Singletons
    pub none: SoxRef<SoxNone>,

    // Compilation roots (for objects held by the compiler during build)
    pub compiler_roots: Vec<SoxObjectRef>,
}

impl Runtime {
    pub fn new() -> Self {
        let types = TypeLibrary::init();
        let mut heap = Heap::new();
        let none = heap.alloc(SoxNone {}, types.none_type.to_owned());

        // Initialize call frame stack
        let mut call_frame_stack = Vec::with_capacity(64);
        for _ in 0..64 {
            call_frame_stack.push(CallFrame::new_frame());
        }

        Runtime {
            heap,
            types,
            stack: Vec::with_capacity(256),
            call_frame_stack,
            call_frame_count: 0,
            frames_max: 64,
            open_upvalues: Vec::new(),
            globals: HashMap::new(),
            environment: Environment::new(),
            locals: Default::default(),
            none,
            compiler_roots: Vec::new(),
            compiler: None,
        }
    }

    /// Central allocation function - handles GC check and delegates to heap.
    pub fn alloc<T: SoxObjectPayload>(&mut self, payload: T, typ: SoxRef<SoxType>) -> SoxRef<T> {
        self.maybe_gc();
        self.heap.alloc(payload, typ)
    }

    fn mark_roots(&self, mark: &mut dyn FnMut(SoxObjectRef)) {
        // VM stack values are roots
        for obj in &self.stack {
            mark(obj.clone());
        }

        // Global variable values are roots
        for (_key, obj) in &self.globals {
            mark(obj.clone());
        }

        // Open upvalues are roots
        for obj in &self.open_upvalues {
            mark(obj.clone());
        }

        // Call frames are roots (they hold references to closures/functions)
        for frame in &self.call_frame_stack[..self.call_frame_count] {
            if let Some(co) = &frame.co {
                mark(co.clone());
            }
        }

        // None singleton
        mark(SoxObjectRef::from(self.none.clone()));

        // Compiler roots (for objects held by the compiler during build)
        for obj in &self.compiler_roots {
            mark(obj.clone());
        }
    }

    /// Check if GC should run and collect if needed.
    pub fn maybe_gc(&mut self) {
        if self.heap.should_collect() {
            // Split borrow: take heap out so we can pass &self to mark_roots
            let mut heap = std::mem::take(&mut self.heap);
            heap.collect(|mark| {
                self.mark_roots(mark);
            });
            self.heap = heap;
        }
    }

    // Convenience allocation methods
    pub fn new_string(&mut self, s: String) -> SoxRef<SoxString> {
        self.alloc(SoxString::from(s), self.types.str_type.to_owned())
    }

    pub fn new_int(&mut self, i: i64) -> SoxRef<SoxInt> {
        self.alloc(SoxInt::from(i), self.types.int_type.to_owned())
    }

    pub fn new_float(&mut self, f: f64) -> SoxRef<SoxFloat> {
        self.alloc(SoxFloat::from(f), self.types.float_type.to_owned())
    }

    pub fn new_bool(&mut self, b: bool) -> SoxRef<SoxBool> {
        self.alloc(SoxBool::from(b), self.types.bool_type.to_owned())
    }

    pub fn new_none(&mut self) -> SoxRef<SoxNone> {
        self.alloc(SoxNone {}, self.types.none_type.to_owned())
    }

    pub fn runtime_error_obj(&mut self, msg: String) -> SoxObjectRef {
        let error = Exception::Err(RuntimeError { msg });
        SoxObjectRef::from(self.heap.alloc(error, self.types.exception_type.to_owned()))
    }

    /// Create a runtime error - method version (for i.runtime_error(...) calls)
    pub fn runtime_error(&mut self, msg: String) -> SoxObjectRef {
        self.runtime_error_obj(msg)
    }

    // ===== VM Execution Methods =====

    fn reset_stack(&mut self) {
        self.stack.clear();
        self.call_frame_count = 0;
    }

    fn vm_runtime_error(&mut self, message: &str) {
        eprintln!("{}", message);
        let mut idx = (self.call_frame_count - 1) as isize;
        while idx >= 0 {
            let frame = &self.call_frame_stack[idx as usize];
            let instruction = frame.ip - 1;
            let co = frame.co.as_ref().unwrap().payload::<Chunk>().unwrap();
            let line = co.get_line(instruction);
            let mut unit_name = co.name.clone();
            if unit_name == "" {
                unit_name = "test_script".to_string();
            }
            eprintln!("[line {}] in {}", line, unit_name);
            idx -= 1;
        }

        self.reset_stack();
    }

    pub fn interpret(&mut self, source: &'static str) {
        let parser = Parser::new(source);
        let mut compiler = Compiler::new("__main__".to_string(), parser);
        let compiled_module = match compiler.compile(self) {
            Ok(module) => module,
            Err(err) => {
                println!("{}", err);
                return;
            }
        };

        let compiled_chunk = compiled_module.co.clone();

        let module_obj = SoxObjectRef::from(SoxRef::new_ref(
            compiled_module,
            self.types.mod_type.to_owned(),
        ));

        push_stack!(self, module_obj);

        let frame_index = self.call_frame_count;
        let frame = &mut self.call_frame_stack[frame_index];
        self.call_frame_count += 1;
        frame.ip = 0;
        frame.co = Some(SoxObjectRef::from(compiled_chunk));
        frame.value_stack_base_addr = self.stack.len() - 1;

        let result = self.run();

        if result == InterpretResult::InterpretOk && !self.stack.is_empty() {
            let value = pop_stack!(self);
            if let Ok(repr_str) = value.repr(self) {
                println!("{repr_str}");
            }
        }
    }

    fn call(
        &mut self,
        func: &SoxFunction,
        arg_count: usize,
        upvalues: Vec<SoxObjectRef>,
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
        frame.co = Some(SoxObjectRef::from(func.chunk.clone()));
        frame.ip = 0;
        frame.upvalues = upvalues;
        frame.value_stack_base_addr = self.stack.len() - arg_count - 1;
        Ok(true)
    }

    fn call_value(&mut self, callee: SoxObjectRef, arg_count: usize) -> bool {
        let result = if let Some(func) = callee.payload::<SoxFunction>() {
            let upvalues = func.upvalues.clone();
            self.call(func, arg_count, upvalues)
        } else {
            let type_name = callee.typ().name.clone().unwrap_or("unknown".to_string());
            self.vm_runtime_error(&format!("{} object is not callable.", type_name));
            return false;
        };

        match result {
            Ok(success) => success,
            Err(e) => {
                self.vm_runtime_error(&e.msg);
                false
            }
        }
    }

    pub fn run(&mut self) -> InterpretResult {
        loop {
            let instruction = read_instr!(self);
            let opcode: OpCode = instruction.try_into().unwrap();
            match opcode {
                OpCode::OpAdd => {
                    binary_op!(self, add, number);
                }
                OpCode::OpSubtract => {
                    binary_op!(self, minus, number);
                }
                OpCode::OpMultiply => {
                    binary_op!(self, star, number);
                }
                OpCode::OpDivide => {
                    binary_op!(self, slash, number);
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
                    self.close_upvalues(frame_base);
                    self.stack.truncate(frame_base);
                    push_stack!(self, val);
                    continue;
                }
                OpCode::OpConstant => {
                    let constant = read_constant!(self);
                    push_stack!(self, constant);
                    continue;
                }
                OpCode::OpNegate => {
                    let val = pop_stack!(self);
                    let neg_op = val.typ().slots.number.as_ref().unwrap().neg.unwrap();
                    let res = (neg_op)(val, self).unwrap();
                    push_stack!(self, res);
                    continue;
                }
                OpCode::OpNone => {
                    push_stack!(self, SoxObjectRef::from(self.none.clone()))
                }
                OpCode::OpTrue => {
                    let val = self.new_bool(true);
                    push_stack!(self, SoxObjectRef::from(val))
                }
                OpCode::OpFalse => {
                    let val = self.new_bool(false);
                    push_stack!(self, SoxObjectRef::from(val))
                }
                OpCode::OpNot => {
                    let val = pop_stack!(self);
                    let bool_val = val.try_into_rust_bool(self);
                    let val = self.new_bool(!bool_val);
                    let not_val = SoxObjectRef::from(val);
                    push_stack!(self, not_val);
                    continue;
                }
                OpCode::OpEqual => {
                    binary_op!(self, eq, comparable);
                }
                OpCode::OpGreater => {
                    binary_op!(self, gt, comparable);
                }
                OpCode::OpLess => {
                    binary_op!(self, lt, comparable);
                }
                OpCode::OpPrint => {
                    let val = pop_stack!(self);
                    let repr_str = val.repr(self);
                    println!("{}", repr_str.unwrap().as_str());
                }
                OpCode::OpPop => {
                    pop_stack!(self);
                }
                OpCode::OpDefineGlobal => {
                    let key = read_constant!(self);
                    let key_name = key.payload::<SoxString>().unwrap();

                    self.globals.insert(key_name.clone(), peek_stack!(self));
                    pop_stack!(self);
                }
                OpCode::OpGetGlobal => {
                    let v = read_constant!(self);
                    let name = v.payload::<SoxString>().unwrap();

                    if let Some(val) = self.globals.get(name) {
                        push_stack!(self, val.clone());
                    } else {
                        eprintln!("NameError: name '{}' is not defined.", name.value);
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
                OpCode::OpSetGlobal => {
                    let v = read_constant!(self);
                    let name = v.payload::<SoxString>().unwrap();

                    if let Some(val) = self.globals.get_mut(&name) {
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
                    push_stack!(self, self.stack[addr].clone());
                }
                OpCode::OpSetLocal => {
                    let slot = read_instr!(self);
                    let slot = slot as u8;
                    self.stack[self.call_frame_stack[self.call_frame_count - 1]
                        .relative_slot(slot as usize)] = peek_stack!(self);
                }
                OpCode::OpJumpIfFalse => {
                    let offset = read_short!(self);
                    let val = peek_stack!(self);
                    if !val.try_into_rust_bool(self) {
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
                    if !self.call_value(callee, arg_count) {
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
                        self.stack[upval_obj.location]
                    };
                    push_stack!(self, val);
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
                        self.stack[upvalue.location] = value;
                    }
                }
                OpCode::OpCloseUpvalue => {
                    self.close_upvalues(self.stack.len() - 1);
                    pop_stack!(self);
                }
                OpCode::OpClosure => {
                    let func_obj = read_constant!(self);
                    let func = func_obj.payload::<SoxFunction>().unwrap();
                    let mut upvalues = Vec::with_capacity(func.upvalue_count);

                    for _ in 0..func.upvalue_count {
                        let is_local = read_instr!(self);
                        let index = read_instr!(self);
                        let upvalue = if is_local == 1 {
                            let location = self.call_frame_stack[self.call_frame_count - 1]
                                .value_stack_base_addr
                                + index as usize;
                            self.capture_upvalue(location)
                        } else {
                            self.call_frame_stack[self.call_frame_count - 1].upvalues
                                [index as usize]
                        };
                        upvalues.push(upvalue);
                    }
                    let closure = func.with_upvalues(upvalues);
                    let allocated_closure =
                        self.alloc(closure, self.types.function_type.to_owned());
                    push_stack!(self, SoxObjectRef::from(allocated_closure));
                }
            }
        }
    }

    pub fn close_upvalues(&mut self, value_stack_idx: usize) {
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
                    let value = self.stack[upvalue_obj.payload.location];
                    upvalue_obj.payload.closed = Some(Box::new(value));
                }
            } else {
                i += 1;
            }
        }
    }

    pub fn capture_upvalue(&mut self, index: usize) -> SoxObjectRef {
        for upvalue_ref in self.open_upvalues.iter() {
            let upvalue = upvalue_ref.payload::<SoxUpvalue>().unwrap();
            if upvalue.location == index {
                return *upvalue_ref;
            }
        }

        let new_upvalue_obj = SoxUpvalue::new(index);
        let upvalue_type = self.types.upvalue_type.to_owned();
        let new_upvalue_ref = SoxObjectRef::from(SoxRef::new_ref(new_upvalue_obj, upvalue_type));

        self.open_upvalues.push(new_upvalue_ref);
        new_upvalue_ref
    }

    // ===== Tree-walk Interpreter Methods =====

    pub fn interpret_stmts(&mut self, statements: &Vec<Stmt>) {
        let mut stmts_iter = statements.iter().peekable();
        while let Some(stmt) = stmts_iter.next() {
            let result = self.execute(stmt);
            if result.is_err() {
                let obj = result.unwrap_err();

                let repr_str = obj.repr(self);
                println!("{}", repr_str.unwrap().as_str());

                break;
            }
            let result_value = result.unwrap();
            if stmts_iter.peek().is_none() {
                if !result_value.payload_is::<SoxNone>() {
                    let repr_str = result_value.repr(self);
                    println!("{}", repr_str.unwrap().as_str());
                }
            }
        }
    }

    fn evaluate(&mut self, expr: &Expr) -> SoxResult {
        expr.accept(self)
    }

    fn execute(&mut self, stmt: &Stmt) -> SoxResult {
        stmt.accept(self)
    }

    pub fn execute_block(
        &mut self,
        statements: Vec<&Stmt>,
        ns_ref: Option<EnvRef>,
    ) -> SoxResult<()> {
        if let Some(ns_ref) = ns_ref {
            self.environment.active = ns_ref.clone();
        } else {
            self.environment.new_local_env();
        }
        for statement in statements {
            let res = self.execute(statement);
            if let Err(v) = res {
                self.environment.pop().expect("TODO: panic message");
                return Err(v);
            }
        }
        self.environment.pop().expect("TODO: panic message");
        Ok(())
    }

    fn lookup_variable(&mut self, name: &Token) -> SoxResult {
        if let Some(dist) = self.locals.get(name) {
            let (dst, binding_idx) = dist;
            let key = (name.lexeme.to_string(), *dst, *binding_idx);
            let val = self.environment.get(key);
            val
        } else {
            let val = self
                .environment
                .get_from_global_scope(name.lexeme.to_string(), self);
            val
        }
    }
}

// Tree-walk interpreter visitor macro
macro_rules! eval_slot_op {
    ($self:ident, $left_val:expr, $right_val:expr, $op_func:ident, $slot_attr: ident, $op_str:expr) => {{
        let exc = Err($self.runtime_error(format!(
            "Unsupported operand types for '{}' - {} and {}",
            $op_str,
            $left_val.typ().name.as_ref().unwrap().as_str(),
            $right_val.typ().name.as_ref().unwrap().as_str()
        )));

        if let Some(nm) = $left_val.typ().slots.$slot_attr.as_ref() {
            if let Some(func) = nm.$op_func {
                func($left_val, $right_val, $self)
            } else {
                exc
            }
        } else {
            exc
        }
    }};
}

impl StmtVisitor for &mut Runtime {
    type T = SoxResult;

    fn visit_expression_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Expression(expr) = stmt {
            let value = self.evaluate(expr);
            value
        } else {
            SoxObjectRef::from(self.none.clone()).to_sox_result(self)
        }
    }

    fn visit_print_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let return_value = if let Stmt::Print(expr) = stmt {
            let value = self.evaluate(expr);
            match value {
                Ok(v) => {
                    println!("{}", v.repr(&self)?.as_str());
                    Ok(SoxObjectRef::from(self.none.clone()))
                }
                Err(v) => Err(v),
            }
        } else {
            let err = SoxObjectRef::from(
                self.runtime_error(
                    "Evaluation failed - visited non print statement with visit_print_stmt."
                        .to_string(),
                ),
            );
            Err(err)
        };
        return_value
    }

    fn visit_decl_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let mut value = SoxObjectRef::from(self.new_none());
        if let Stmt::Var { name, initializer } = stmt {
            if let Some(initializer_stmt) = initializer {
                value = self.evaluate(initializer_stmt)?;
            }
            let name_ident = name.lexeme.to_string();

            self.environment.define(name_ident, value)
        } else {
            let err = SoxObjectRef::from(self.runtime_error("Evaluation failed - visiting a non declaration statement with visit_decl_stmt.".to_string()));
            return Err(err);
        };
        Ok(value)
    }

    fn visit_block_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Block(statements) = stmt {
            let stmts = statements.iter().map(|v| v).collect::<Vec<&Stmt>>();
            self.execute_block(stmts, None)?;
            Ok(SoxObjectRef::from(self.none.clone()))
        } else {
            Err(self.runtime_error(
                "Evaluation failed - visited non block statement with visit_block_stmt."
                    .to_string(),
            ))
        }
    }

    fn visit_if_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::If {
            condition,
            then_branch,
            else_branch,
        } = stmt
        {
            let cond_val = self.evaluate(condition)?;
            if cond_val.try_into_rust_bool(self) {
                self.execute(then_branch)?;
            } else if let Some(else_branch_stmt) = else_branch.as_ref() {
                self.execute(else_branch_stmt)?;
            }
        } else {
            return Err(self.runtime_error(
                "Evaluation failed - visited non if statement with visit_if_stmt".to_string(),
            ));
        }
        Ok(SoxObjectRef::from(self.none.clone()))
    }

    fn visit_while_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::While { condition, body } = stmt {
            let mut cond = self.evaluate(condition)?;
            while cond.try_into_rust_bool(self) {
                self.execute(body)?;
                cond = self.evaluate(&condition)?;
            }

            Ok(SoxObjectRef::from(self.none.clone()))
        } else {
            Err(self.runtime_error(
                "Evaluation failed -  visited non while statement with visit_while_stmt."
                    .to_string(),
            ))
        }
    }

    fn visit_function_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Function {
            name,
            params,
            body: _body,
        } = stmt
        {
            let stmt_clone = stmt.clone();
            let fo = SoxFn::new(
                name.lexeme.to_string(),
                stmt_clone,
                self.environment.active.clone(),
                params.len() as i8,
                false,
            );
            self.environment.define(
                name.lexeme.to_string(),
                SoxObjectRef::from(SoxRef::new_ref(fo, self.types.func_type.to_owned())),
            );
            Ok(SoxObjectRef::from(self.none.clone()))
        } else {
            Err(self.runtime_error(
                "Evaluation failed -  Calling a visit_function_stmt on non function node."
                    .to_string(),
            ))
        }
    }

    fn visit_return_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let mut return_value = SoxObjectRef::from(self.none.clone());
        if let Stmt::Return { keyword: _, value } = stmt {
            if let Some(value) = value {
                return_value = self.evaluate(value)?;
            }
        }
        let exc = SoxRef::new_ref(
            Exception::Return(return_value),
            self.types.exception_type.to_owned(),
        );
        Err(SoxObjectRef::from(exc))
    }

    fn visit_class_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let ret_val = if let Stmt::Class {
            name,
            superclass,
            methods,
        } = stmt
        {
            // get super class if exist
            let sc = if superclass.is_some() {
                let sc = self.evaluate(superclass.as_ref().unwrap());

                // TODO fix check returned value is a class
                if let Ok(v) = sc {
                    info!("Evaluated to a class");
                    Some(v)
                } else {
                    let re = self.runtime_error("Superclass must be a class.".to_string());
                    return Err(re);
                }
            } else {
                None
            };

            let none_val = self.none.clone().clone();
            let obj_ref = SoxObjectRef::from(none_val);
            self.environment.define(name.lexeme.to_string(), obj_ref);
            let prev_env_ref = self.environment.active.clone();

            if let Some(sc) = sc {
                self.environment.new_local_env();
                self.environment.define("super", sc)
            }

            let mut methods_map = HashMap::new();
            //setup methods
            for method in methods.iter() {
                if let Stmt::Function {
                    name,
                    body: _body,
                    params: _params,
                } = method
                {
                    let func = SoxFn {
                        name: name.lexeme.to_string(),
                        declaration: Box::new(method.clone()),
                        environment_ref: self.environment.active.clone(),
                        is_initializer: name.lexeme == "init".to_string(),
                        arity: _params.len() as i8,
                    };
                    let func_ref = SoxRef::new_ref(func, self.types.func_type.to_owned());
                    methods_map.insert(name.lexeme.into(), SoxObjectRef::from(func_ref));
                }
            }

            // set up class in environment
            let class_name = name.lexeme.to_string();
            // Handle superclass: if present use it, otherwise create class without base
            let base_class = if let Some(ref sc_obj) = sc {
                if let Some(typ) = sc_obj.payload::<SoxType>() {
                    Some(SoxRef::new_ref(
                        typ.clone(),
                        self.types.type_type.to_owned(),
                    ))
                } else {
                    // Superclass was provided but is not a type - return error
                    return Err(self.runtime_error("Superclass must be a class.".to_string()));
                }
            } else {
                None
            };
            let class = SoxType::new(
                class_name.to_string(),
                base_class,
                Default::default(),
                Default::default(),
                methods_map,
            );
            self.environment.active = prev_env_ref;
            let cls_obj = SoxRef::new_ref(class, self.types.type_type.to_owned());
            if let Err(e) = self
                .environment
                .find_and_assign(name.lexeme.to_string(), SoxObjectRef::from(cls_obj))
            {
                return Err(e);
            }

            Ok(SoxObjectRef::from(self.none.clone()))
        } else {
            let err =
                self.runtime_error("Calling a visit_class_stmt on non class type.".to_string());
            return Err(err);
        };
        ret_val
    }
}

impl ExprVisitor for &mut Runtime {
    type T = Result<SoxObjectRef, SoxObjectRef>;

    fn visit_assign_expr(&mut self, expr: &Expr) -> Self::T {
        let ret_val = if let Expr::Assign { name, value } = expr {
            let eval_val = self.evaluate(value)?;
            let dist = self.locals.get(&name);
            if dist.is_some() {
                let (dst, idx) = dist.unwrap();
                let key = (name.lexeme.to_string(), *dst, *idx);

                self.environment.assign(&key, eval_val.clone())?;
            } else {
                self.environment
                    .assign_in_global(name.lexeme.to_string(), eval_val.clone())?;
            };
            Ok(eval_val)
        } else {
            Err(self.runtime_error("Evaluation failed -  called visit_assign_expr to process non assignment statement.".to_string()))
        };
        ret_val
    }

    fn visit_literal_expr(&mut self, expr: &Expr) -> Self::T {
        let value = if let Expr::Literal { value } = expr {
            let obj = match value {
                Literal::String(s) => SoxObjectRef::from(self.new_string(s.to_string())),
                Literal::Integer(i) => SoxObjectRef::from(self.new_int(i.clone())),
                Literal::Float(f) => SoxObjectRef::from(self.new_float(f.0.clone())),
                Literal::Boolean(b) => SoxObjectRef::from(self.new_bool(b.clone())),
                Literal::None => SoxObjectRef::from(self.new_none()),
            };
            Ok(obj)
        } else {
            Err(self.runtime_error(
                "Evaluation failed - called visit_literal_expr on a non literal expression"
                    .to_string(),
            ))
        };
        value
    }

    fn visit_binary_expr(&mut self, expr: &Expr) -> Self::T {
        let value = if let Expr::Binary {
            left,
            operator,
            right,
        } = expr
        {
            let right_val = self.evaluate(right)?;
            let left_val = self.evaluate(left)?;

            match operator.token_type {
                TokenType::Minus => {
                    eval_slot_op!(self, left_val, right_val, minus, number, "-")
                }
                TokenType::Rem => {
                    eval_slot_op!(self, left_val, right_val, rem, number, "%")
                }
                TokenType::Plus => {
                    eval_slot_op!(self, left_val, right_val, add, number, "+")
                }
                TokenType::Star => {
                    eval_slot_op!(self, left_val, right_val, star, number, "*")
                }
                TokenType::Slash => {
                    eval_slot_op!(self, left_val, right_val, slash, number, "/")
                }
                TokenType::Less => {
                    eval_slot_op!(self, left_val, right_val, lt, comparable, "<")
                }
                TokenType::Greater => {
                    eval_slot_op!(self, left_val, right_val, gt, comparable, ">")
                }

                TokenType::EqualEqual => {
                    eval_slot_op!(self, left_val, right_val, eq, comparable, "==")
                }
                TokenType::BangEqual => {
                    eval_slot_op!(self, left_val, right_val, ne, comparable, "!=")
                }
                TokenType::LessEqual => {
                    eval_slot_op!(self, left_val, right_val, le, comparable, "<=")
                }
                TokenType::GreaterEqual => {
                    eval_slot_op!(self, left_val, right_val, ge, comparable, ">=")
                }

                _ => Err(self.runtime_error(
                    "Supplied token does not support binary operations.".to_string(),
                )),
            }
        } else {
            Err(self.runtime_error(
                "Evaluation failed - called visit_binary_expr on non binary expression".into(),
            ))
        };
        value
    }

    fn visit_grouping_expr(&mut self, expr: &Expr) -> Self::T {
        let value = if let Expr::Grouping { expr } = expr {
            Ok(self.evaluate(expr)?)
        } else {
            Err(self.runtime_error(
                "Evaluation failed - called visit_grouping_expr on a non-group node.".to_string(),
            ))
        };
        value
    }

    fn visit_unary_expr(&mut self, expr: &Expr) -> Self::T {
        let value = if let Expr::Unary { operator, right } = expr {
            let right = self.evaluate(right)?;
            match operator.token_type {
                TokenType::Minus => {
                    let value = if let Some(v) = right.payload::<SoxFloat>() {
                        let new_val = SoxRef::new_ref(
                            SoxFloat { value: -v.value },
                            self.types.float_type.to_owned(),
                        );
                        Ok(SoxObjectRef::from(new_val))
                    } else if let Some(v) = right.payload::<SoxInt>() {
                        let new_val = SoxRef::new_ref(
                            SoxInt { value: -v.value },
                            self.types.int_type.to_owned(),
                        );
                        Ok(SoxObjectRef::from(new_val))
                    } else {
                        Err(self.runtime_error(
                            "The unary operator (-) can only be applied to a numeric value."
                                .to_string(),
                        ))
                    };
                    value
                }

                TokenType::Bang => {
                    let value = right.try_into_rust_bool(self);
                    Ok(SoxObjectRef::from(SoxRef::new_ref(
                        SoxBool::from(!value),
                        self.types.bool_type.to_owned(),
                    )))
                }
                _ => Err(self.runtime_error("Unknown unary operator.".to_string())),
            }
        } else {
            let error = self.runtime_error(
                "Evaluation failed - called visit_unary_expr on a non unary expression".to_string(),
            );
            Err(error)
        };
        value
    }

    fn visit_logical_expr(&mut self, expr: &Expr) -> Self::T {
        fn should_short_circuit(
            operator: &Token,
            left_result: &SoxObjectRef,
            i: &mut Runtime,
        ) -> bool {
            let left_value = left_result.try_into_rust_bool(i);
            match operator.token_type {
                TokenType::Or => left_value,
                TokenType::And => !left_value,
                _ => unreachable!(), // Should not happen for logical expressions.
            }
        }

        if let Expr::Logical {
            left,
            operator,
            right,
        } = expr
        {
            let left_result = self.evaluate(left)?;
            if should_short_circuit(operator, &left_result, self) {
                return Ok(left_result);
            }
            self.evaluate(&right)
        } else {
            Err(self.runtime_error(
                "Evaluation failed - called visit_logical_expr on non logical expression."
                    .to_string(),
            ))
        }
    }

    fn visit_variable_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Variable { name } = expr {
            self.lookup_variable(name)
        } else {
            Err(self.runtime_error(
                "Evaluation failed - called visit_variable_expr on non variable expr.".into(),
            ))
        }
    }

    fn visit_call_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Call {
            callee,
            paren: _,
            arguments,
        } = expr
        {
            let callee_ = self.evaluate(callee)?;
            let mut args = vec![];
            for argument in arguments {
                let arg_val = self.evaluate(argument)?;
                args.push(arg_val);
            }
            let call_args = FuncArgs::new(args);
            let callee_type = callee_.typ();
            let callee_type_name = callee_type.name.clone().unwrap();
            let ret_val = match callee_type.slots.call {
                Some(fo) => {
                    let val = (fo)(callee_, call_args, self);
                    val
                }
                _ => {
                    Err(self.runtime_error(format!("{} object is not callable.", callee_type_name)))
                }
            };
            ret_val
        } else {
            Err(self.runtime_error("Can only call functions and classes".to_string()))
        }
    }
    fn visit_get_expr(&mut self, expr: &Expr) -> Self::T {
        let ret_val = if let Expr::Get { name, object } = expr {
            let object = self.evaluate(object)?;
            if let Some(inst) = object.payload::<SoxInstance>() {
                let inst_ref = SoxRef::new_ref(inst.clone(), self.types.obj_type.to_owned());
                SoxInstance::get(inst_ref, name.clone(), self)
            } else {
                Err(self.runtime_error("Only class instances have attributes".to_string()))
            }
        } else {
            Err(self.runtime_error("Calling visit_get_expr on none get expr".to_string()))
        };
        ret_val
    }

    fn visit_set_expr(&mut self, expr: &Expr) -> Self::T {
        let ret_val = if let Expr::Set {
            name,
            object,
            value,
        } = expr
        {
            let object = self.evaluate(object)?;
            if let Some(v) = object.payload::<SoxInstance>() {
                let value = self.evaluate(value)?;

                v.set(name.clone(), value.clone());
                Ok(value)
            } else {
                Err(self.runtime_error("Only instances have fields".to_string()))
            }
        } else {
            Err(self.runtime_error("Calling visit_set_expr on none set expr".to_string()))
        };
        ret_val
    }
    fn visit_this_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::This { keyword } = expr {
            let value = self.lookup_variable(keyword);
            value
        } else {
            Err(self.runtime_error("Calling visit_this_expr on none this expr".to_string()))
        }
    }
    fn visit_super_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Super { keyword, method } = expr {
            let (dist_to_ns, binding_idx) = match self.locals.get(&keyword) {
                Some(v) => v,
                None => {
                    return Err(self.runtime_error("Cannot resolve 'super' binding.".to_string()))
                }
            };
            let this_token = Token::new(TokenType::This, "this", Literal::None, 0);
            let (dist_to_ns2, binding_idx2) = match self.locals.get(&this_token) {
                Some(v) => v,
                None => {
                    return Err(self.runtime_error("Cannot resolve 'this' binding.".to_string()))
                }
            };

            let key = ("super".to_string(), *dist_to_ns, *binding_idx);
            let key2 = ("this".to_string(), *dist_to_ns2, *binding_idx2);

            let super_type = self.environment.get(key)?;
            let instance = self.environment.get(key2)?;

            let ty = super_type.payload::<SoxType>();
            let method = if let Some(v) = ty {
                let c = v;
                let method_name = method.lexeme;
                let method = c.find_method(method_name);
                let t = if let Some(m) = method {
                    if let Some(func) = m.payload::<SoxFn>() {
                        let bound_method = func.bind(instance, self)?;
                        Ok(bound_method)
                    } else {
                        Err(self.runtime_error(format!("Undefined property {}", method_name)))
                    }
                } else {
                    Err(self.runtime_error(format!("Undefined property {}", method_name)))
                };
                t
            } else {
                Err(self.runtime_error("Unable to resolve instance - this".to_string()))
            };
            method
        } else {
            Err(self.runtime_error("Calling visit_super_expr on none super expr".to_string()))
        }
    }
}

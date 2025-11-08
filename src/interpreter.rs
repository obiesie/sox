use crate::builtins::bool::SoxBool;
use crate::builtins::core::SoxResult;
use crate::builtins::core::ToSoxResult;
use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::float::SoxFloat;
use crate::builtins::function::SoxFn;
use crate::builtins::int::SoxInt;
use crate::builtins::method::FuncArgs;
use crate::builtins::none::SoxNone;
use crate::builtins::r#type::{SoxInstance, SoxType};
use crate::builtins::string::SoxString;
use crate::catalog::TypeLibrary;
use crate::environment::{EnvRef, Environment};
use crate::expr::Expr;
use crate::expr::ExprVisitor;
use crate::object::core::{SoxObjectRef, SoxRef};
use crate::stmt::{Stmt, StmtVisitor};
use crate::token::{Literal, Token};
use crate::token_type::TokenType;
use log::info;
use std::collections::HashMap;

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

pub struct Interpreter {
    pub environment: Environment,
    pub types: TypeLibrary,
    pub none: SoxRef<SoxNone>,
    pub locals: HashMap<Token, (usize, usize)>,
}

impl Interpreter {
    pub fn new() -> Self {
        let types = TypeLibrary::init();
        let none = SoxRef::new_ref(SoxNone {}, types.none_type.to_owned());
        let interpreter = Interpreter {
            environment: Environment::new(),
            types,
            none,
            locals: Default::default(),
        };
        interpreter
    }

    pub fn new_string(&self, s: String) -> SoxRef<SoxString> {
        let s = SoxRef::new_ref(SoxString::from(s), self.types.str_type.to_owned());
        s
    }

    pub fn new_int(&self, i: i64) -> SoxRef<SoxInt> {
        SoxRef::new_ref(SoxInt::from(i), self.types.int_type.to_owned())
    }

    pub fn new_float(&self, f: f64) -> SoxRef<SoxFloat> {
        SoxRef::new_ref(SoxFloat::from(f), self.types.float_type.to_owned())
    }

    pub fn new_bool(&self, b: bool) -> SoxRef<SoxBool> {
        SoxRef::new_ref(SoxBool::from(b), self.types.bool_type.to_owned())
    }

    pub fn new_none(&self) -> SoxRef<SoxNone> {
        SoxRef::new_ref(SoxNone {}, self.types.none_type.to_owned())
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>) {
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

    pub fn runtime_error(&self, msg: String) -> SoxObjectRef {
        let error = Exception::Err(RuntimeError { msg });
        SoxObjectRef::from(SoxRef::new_ref(error, self.types.exception_type.to_owned()))
    }
}

impl StmtVisitor for &mut Interpreter {
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
                //let obj_ref = SoxObjectRef::from(sc.as_ref().unwrap().clone());
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
            let t = sc
                .clone()
                .unwrap()
                .payload::<SoxType>()
                .clone()
                .unwrap()
                .clone();
            let class = SoxType::new(
                class_name.to_string(),
                Some(SoxRef::new_ref(t, self.types.type_type.to_owned())),
                Default::default(),
                Default::default(),
                methods_map,
            );
            self.environment.active = prev_env_ref;
            let cls_obj = SoxRef::new_ref(class, self.types.type_type.to_owned());
            self.environment
                .find_and_assign(name.lexeme.to_string(), SoxObjectRef::from(cls_obj))
                .expect("TODO: panic message");

            Ok(SoxObjectRef::from(self.none.clone()))
        } else {
            let err = self.runtime_error("Calling a visit_class_stmt on non class type.".into());
            return Err(err);
        };
        ret_val
    }
}

impl ExprVisitor for &mut Interpreter {
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

                _ => {
                    Err(self
                        .runtime_error("Supplied token does not support binary operations.".into()))
                }
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
                _ => Err(self.runtime_error("Unknown unary operator.".into())),
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
            i: &mut Interpreter,
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
            Err(self.runtime_error("Can only call functions and classes".into()))
        }
    }
    fn visit_get_expr(&mut self, expr: &Expr) -> Self::T {
        let ret_val = if let Expr::Get { name, object } = expr {
            let object = self.evaluate(object)?;
            if let Some(inst) = object.payload::<SoxInstance>() {
                let inst_ref = SoxRef::new_ref(inst.clone(), self.types.obj_type.to_owned());
                SoxInstance::get(inst_ref, name.clone(), self)
            } else {
                Err(self.runtime_error("Only class instances have attributes".into()))
            }
        } else {
            Err(self.runtime_error("Calling visit_get_expr on none get expr".into()))
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
                Err(self.runtime_error("Only instances have fields".into()))
            }
        } else {
            Err(self.runtime_error("Calling visit_set_expr on none set expr".into()))
        };
        ret_val
    }
    fn visit_this_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::This { keyword } = expr {
            let value = self.lookup_variable(keyword);
            value
        } else {
            Err(self.runtime_error("Calling visit_this_expr on none this expr".into()))
        }
    }
    fn visit_super_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Super { keyword, method } = expr {
            let (dist_to_ns, binding_idx) = self.locals.get(&keyword).unwrap();
            let this_token = Token::new(TokenType::This, "this", Literal::None, 0);
            let (dist_to_ns2, binding_idx2) = self.locals.get(&this_token).unwrap();

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
                Err(self.runtime_error("Unable to resolve instance - this".into()))
            };
            method
        } else {
            Err(self.runtime_error("Calling visit_super_expr on none super expr".to_string()))
        }
    }
}

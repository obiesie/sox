use std::collections::HashMap;

use log::info;

use crate::builtins::bool_::SoxBool;
use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::float::SoxFloat;
use crate::builtins::function::SoxFunction;
use crate::builtins::int::SoxInt;
use crate::builtins::method::FuncArgs;
use crate::builtins::none::SoxNone;
use crate::builtins::r#type::{SoxInstance, SoxType};
use crate::builtins::string::SoxString;
use crate::catalog::TypeLibrary;
use crate::core::SoxObjectPayload;
use crate::core::SoxRef;
use crate::core::{SoxObject, SoxResult};
use crate::environment::{EnvRef, Environment};
use crate::expr::Expr;
use crate::expr::ExprVisitor;
use crate::stmt::{Stmt, StmtVisitor};
use crate::token::{Literal, Token};
use crate::token_type::TokenType;

pub struct Interpreter {
    //pub envs: SlotMap<DefaultKey, Env>,
    pub environment: Environment,
    //pub active_env_ref: DefaultKey,
    //pub global_env_ref: DefaultKey,
    pub types: TypeLibrary,
    pub none: SoxRef<SoxNone>,
    pub _locals: HashMap<Token, (usize, usize)>,
}

impl Interpreter {
    pub fn new() -> Self {
        // let environment = Env::default();
        // let mut envs = SlotMap::new();
        // let active_env_ref = envs.insert(environment);
        let types = TypeLibrary::init();
        let none = SoxRef::new(SoxNone {});
        let interpreter = Interpreter {
            //envs,
            environment: Environment::new(),
            //active_env_ref,
            //global_env_ref: active_env_ref.clone(),
            types,
            none,
            _locals: Default::default(),
        };
        interpreter
    }

    pub fn new_string(&self, s: String) -> SoxObject {
        let str = SoxRef::new(SoxString::from(s));
        str.to_sox_object()
    }

    pub fn new_int(&self, i: i64) -> SoxObject {
        SoxInt::from(i).into_ref()
    }

    pub fn new_float(&self, f: f64) -> SoxObject {
        SoxFloat::from(f).into_ref()
    }

    pub fn new_bool(&self, b: bool) -> SoxObject {
        SoxBool::from(b).into_ref()
    }

    pub fn new_none(&self) -> SoxObject {
        SoxNone {}.into_ref()
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>) {
        let mut m = statements.iter().peekable();
        while let Some(stmt) = m.next() {
            let result = self.execute(stmt);
            if result.is_err() {
                println!("{}", result.unwrap_err().repr(&self));
                break;
            }
            let result_value = result.unwrap();
            if m.peek().is_none() {
                if let SoxObject::None(_) = result_value {
                } else {
                    println!("{}", result_value.repr(&self));
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
        // let active_env = self.active_env_mut();
        // if let Some(ns) = namespace {
        //     active_env.push(ns)?;
        // } else {
        //     active_env.new_namespace()?;
        // }
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
        if let Some(dist) = self._locals.get(name) {
            let (dst, binding_idx) = dist;
            let key = (name.lexeme.to_string(), *dst, *binding_idx);
            let val = self.environment.get(key);
            val
        } else {
            let val = self
                .environment
                .get_from_global_scope(name.lexeme.to_string());
            val
        }
    }

    pub fn runtime_error(msg: String) -> SoxObject {
        let error = Exception::Err(RuntimeError { msg });
        error.into_ref()
    }
}

impl StmtVisitor for &mut Interpreter {
    type T = SoxResult;

    fn visit_expression_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let mut return_value = Ok(self.none.into_ref());
        if let Stmt::Expression(expr) = stmt {
            let value = self.evaluate(expr);
            return_value = match value {
                Ok(v) => Ok(v),
                Err(v) => Err(v.into()),
            };
        }
        return_value
    }

    fn visit_print_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let return_value = if let Stmt::Print(expr) = stmt {
            let value = self.evaluate(expr);
            match value {
                Ok(v) => {
                    println!("{}", v.repr(&self));
                    Ok(self.none.into_ref())
                }
                Err(v) => Err(v.into()),
            }
        } else {
            Err(Interpreter::runtime_error(
                "Evaluation failed - visited non print statement with visit_print_stmt."
                    .to_string(),
            ))
        };
        return_value
    }

    fn visit_decl_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let mut value = SoxNone {}.into_ref();
        if let Stmt::Var { name, initializer } = stmt {
            //let v = self.lookup_variable(name);

            if let Some(initializer_stmt) = initializer {
                value = self.evaluate(initializer_stmt)?;
            }
            // let active_env = self.active_env_mut();
            let name_ident = name.lexeme.to_string();
            //
            // active_env.define(name_ident, value)

            self.environment.define(name_ident, value)
        } else {
            return Err(Interpreter::runtime_error(
                "Evaluation failed - visiting a non declaration statement with visit_decl_stmt."
                    .to_string(),
            ));
        };
        Ok(self.none.into_ref())
    }

    fn visit_block_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Block(statements) = stmt {
            let stmts = statements.iter().map(|v| v).collect::<Vec<&Stmt>>();

            self.execute_block(stmts, None)?;

            Ok(self.none.into_ref())
        } else {
            Err(Interpreter::runtime_error(
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
            return Err(Interpreter::runtime_error(
                "Evaluation failed - visited non if statement with visit_if_stmt".to_string(),
            ));
        }
        Ok(self.none.into_ref())
    }

    fn visit_while_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::While { condition, body } = stmt {
            let mut cond = self.evaluate(condition)?;
            while cond.try_into_rust_bool(self) {
                self.execute(body)?;
                cond = self.evaluate(&condition)?;
            }

            Ok(self.none.into_ref())
        } else {
            Err(Interpreter::runtime_error(
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
            let fo = SoxFunction::new(
                name.lexeme.to_string(),
                stmt_clone,
                self.environment.active.clone(),
                params.len() as i8,
                false
            );
            self.environment
                .define(name.lexeme.to_string(), fo.into_ref());
            Ok(self.none.into_ref())
        } else {
            Err(Interpreter::runtime_error(
                "Evaluation failed -  Calling a visit_function_stmt on non function node."
                    .to_string(),
            ))
        }
    }

    fn visit_return_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let mut return_value = self.none.into_ref();
        if let Stmt::Return { keyword: _, value } = stmt {
            if let Some(value) = value {
                return_value = self.evaluate(value)?;
            }
        }
        Err(Exception::Return(return_value).into_ref())
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

                if let Ok(SoxObject::Type(v)) = sc {
                    info!("Evaluated to a class");
                    Some(v)
                } else {
                    let re = Interpreter::runtime_error("Superclass must be a class.".to_string());
                    return Err(re);
                }
            } else {
                None
            };
            let none_val = self.none.clone().into_ref();
            // let active_env = self.active_env_mut();
            self.environment.define(name.lexeme.to_string(), none_val);
            let prev_env_ref = self.environment.active.clone();
            //let prev_env = self.active_env_ref.clone();
            // setup super keyword within namespace
            if sc.is_some() {
                self.environment.new_local_env();
                self.environment
                    .define("super", SoxObject::Type(sc.as_ref().unwrap().clone()))
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
                    let func = SoxFunction {
                        name: name.lexeme.to_string(),
                        declaration: Box::new(method.clone()),
                        environment_ref: self.environment.active.clone(),
                        is_initializer: name.lexeme == "init".to_string(),
                        arity: _params.len() as i8,
                    };
                    methods_map.insert(name.lexeme.clone().into(), func.into_ref());
                }
            }

            // set up class in environment
            let class_name = name.lexeme.to_string();
            let class = SoxType::new(
                class_name.to_string(),
                sc,
                Default::default(),
                Default::default(),
                methods_map,
            );
            self.environment.active = prev_env_ref;
            self.environment
                .find_and_assign(name.lexeme.to_string(), class.into_ref()).expect("TODO: panic message");
            // self.active_env_ref = prev_env;
            // let active_env = self.active_env_mut();
            // active_env.find_and_assign(name.lexeme.clone(), class.into_ref())?;
            //
            Ok(self.none.into_ref())
        } else {
            let err =
                Interpreter::runtime_error("Calling a visit_class_stmt on non class type.".into());
            return Err(err);
        };
        ret_val
    }
}

impl ExprVisitor for &mut Interpreter {
    type T = Result<SoxObject, SoxObject>;

    fn visit_assign_expr(&mut self, expr: &Expr) -> Self::T {
        let ret_val = if let Expr::Assign { name, value } = expr {
            let eval_val = self.evaluate(value)?;
            let dist = self._locals.get(&name);
            if dist.is_some() {
                let (dst, idx) = dist.unwrap();
                // info!("Distance found from resolution is {dst}");
                let key = (name.lexeme.to_string(), *dst, *idx);

                // let env = self.active_env_mut();
                // env.assign(&key, eval_val.clone())?;
                //
                self.environment.assign(&key, eval_val.clone())?;
            } else {
                // let env = self.active_env_mut();
                self.environment
                    .assign_in_global(name.lexeme.to_string(), eval_val.clone())?;
            };
            Ok(eval_val)
        } else {
            Err(Interpreter::runtime_error("Evaluation failed -  called visit_assign_expr to process non assignment statement.".to_string()))
        };
        ret_val
    }

    fn visit_literal_expr(&mut self, expr: &Expr) -> Self::T {
        let value = if let Expr::Literal { value } = expr {
            let obj = match value {
                Literal::String(s) => self.new_string(s.clone()),
                Literal::Integer(i) => self.new_int(i.clone()),
                Literal::Float(f) => self.new_float(f.0.clone()),
                Literal::Boolean(b) => self.new_bool(b.clone()),
                Literal::None => self.new_none(),
            };
            Ok(obj)
        } else {
            Err(Interpreter::runtime_error(
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
                    let exc = Err(Interpreter::runtime_error(
                        "Operands must be two numbers or two strings".into(),
                    ));
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxInt::from(v1.value - v2.value).into_ref())
                    } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                        if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                            Ok(SoxFloat::from(v1.value - v2.value).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_float(), right_val.as_int())
                        {
                            Ok(SoxFloat::from(v1.value - (v2.value as f64)).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_int(), right_val.as_float())
                        {
                            Ok(SoxFloat::from((v1.value as f64) - v2.value).into_ref())
                        } else {
                            exc
                        }
                    } else {
                        exc
                    };
                    value
                }
                TokenType::Rem => {
                    let exc = Err(Interpreter::runtime_error(
                        "Arguments to the remainder operator must both be numbers".into(),
                    ));
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxInt::from(v1.value % v2.value).into_ref())
                    } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                        if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                            Ok(SoxFloat::from(v1.value % v2.value).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_float(), right_val.as_int())
                        {
                            Ok(SoxFloat::from(v1.value % (v2.value as f64)).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_int(), right_val.as_float())
                        {
                            Ok(SoxFloat::from((v1.value as f64) % v2.value).into_ref())
                        } else {
                            exc
                        }
                    } else {
                        exc
                    };
                    value
                }
                TokenType::Plus => {
                    let exc = Err(Interpreter::runtime_error(
                        "Operands must be two numbers or two strings.".into(),
                    ));
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxInt::from(v1.value + v2.value).into_ref())
                    } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                        if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                            Ok(SoxFloat::from(v1.value + v2.value).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_float(), right_val.as_int())
                        {
                            Ok(SoxFloat::from(v1.value + (v2.value as f64)).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_int(), right_val.as_float())
                        {
                            Ok(SoxFloat::from((v1.value as f64) + v2.value).into_ref())
                        } else {
                            exc
                        }
                    } else if let (Some(v1), Some(v2)) =
                        (left_val.as_string(), right_val.as_string())
                    {
                        Ok(SoxString::from(v1.value.clone() + v2.value.as_str()).into_ref())
                    } else {
                        exc
                    };

                    value
                }
                TokenType::Star => {
                    let exc = Err(Interpreter::runtime_error(
                        "Arguments to the multiplication operator must both be numbers".into(),
                    ));
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxInt::from(v1.value * v2.value).into_ref())
                    } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                        if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                            Ok(SoxFloat::from(v1.value * v2.value).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_float(), right_val.as_int())
                        {
                            Ok(SoxFloat::from(v1.value * (v2.value as f64)).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_int(), right_val.as_float())
                        {
                            Ok(SoxFloat::from((v1.value as f64) * v2.value).into_ref())
                        } else {
                            exc
                        }
                    } else {
                        exc
                    };
                    value
                }
                TokenType::Slash => {
                    let exc = Err(Interpreter::runtime_error(
                        "Arguments to the division operator must both be numbers".into(),
                    ));
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxFloat::from((v1.value as f64) / (v2.value as f64)).into_ref())
                    } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                        if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                            Ok(SoxFloat::from(v1.value / v2.value).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_float(), right_val.as_int())
                        {
                            Ok(SoxFloat::from(v1.value / (v2.value as f64)).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_int(), right_val.as_float())
                        {
                            Ok(SoxFloat::from((v1.value as f64) / v2.value).into_ref())
                        } else {
                            exc
                        }
                    } else {
                        exc
                    };
                    value
                }
                TokenType::Less => {
                    let exc = Err(Interpreter::runtime_error(
                        "Arguments to the less than operator must both be numbers".into(),
                    ));
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxBool::from(v1.value < v2.value).into_ref())
                    } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                        if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                            Ok(SoxBool::from(v1.value < v2.value).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_float(), right_val.as_int())
                        {
                            Ok(SoxBool::from(v1.value < (v2.value as f64)).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_int(), right_val.as_float())
                        {
                            Ok(SoxBool::from((v1.value as f64) < v2.value).into_ref())
                        } else {
                            exc
                        }
                    } else {
                        exc
                    };
                    value
                }
                TokenType::Greater => {
                    let exc = Err(Interpreter::runtime_error(
                        "Arguments to the greater than operator must both be numbers".into(),
                    ));
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxBool::from(v1.value > v2.value).into_ref())
                    } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                        if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                            Ok(SoxBool::from(v1.value > v2.value).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_float(), right_val.as_int())
                        {
                            Ok(SoxBool::from(v1.value > (v2.value as f64)).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_int(), right_val.as_float())
                        {
                            Ok(SoxBool::from((v1.value as f64) > v2.value).into_ref())
                        } else {
                            exc
                        }
                    } else {
                        exc
                    };
                    value
                }

                TokenType::EqualEqual => {
                    let left_type = left_val.sox_type(self);
                    let eq = left_type.slots.methods.iter().find(|v| v.0 == "equals");
                    if let Some(entry) = eq {
                        let call_args = FuncArgs::new(vec![left_val.clone(), right_val.clone()]);
                        (entry.1.func)(self, call_args)
                    } else {
                        Ok(SoxBool::from(false).into_ref())
                    }
                   // let eq_slot_func = left_type.

                    // let value = if let (Some(v1), Some(v2)) =
                    //     (left_val.as_int(), right_val.as_int())
                    // {
                    //     Ok(SoxBool::from(v1.value == v2.value).into_ref())
                    // } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                    //     if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                    //         Ok(SoxBool::from(v1.value == v2.value).into_ref())
                    //     } else if let (Some(v1), Some(v2)) =
                    //         (left_val.as_float(), right_val.as_int())
                    //     {
                    //         Ok(SoxBool::from(v1.value == (v2.value as f64)).into_ref())
                    //     } else if let (Some(v1), Some(v2)) =
                    //         (left_val.as_int(), right_val.as_float())
                    //     {
                    //         Ok(SoxBool::from((v1.value as f64) == v2.value).into_ref())
                    //     } else {
                    //         Ok(SoxBool::from(false).into_ref())
                    //     }
                    // } else {
                    //     Ok(SoxBool::from(false).into_ref())
                    // };
                    // value

                }
                TokenType::BangEqual => {
                    // let exc = Err(Interpreter::runtime_error(
                    //     "Arguments to the not equals operator must both be numbers".into(),
                    // ));
                    // let value = if let (Some(v1), Some(v2)) =
                    //     (left_val.as_int(), right_val.as_int())
                    // {
                    //     Ok(SoxBool::from(v1.value != v2.value).into_ref())
                    // } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                    //     if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                    //         Ok(SoxBool::from(v1.value != v2.value).into_ref())
                    //     } else if let (Some(v1), Some(v2)) =
                    //         (left_val.as_float(), right_val.as_int())
                    //     {
                    //         Ok(SoxBool::from(v1.value != (v2.value as f64)).into_ref())
                    //     } else if let (Some(v1), Some(v2)) =
                    //         (left_val.as_int(), right_val.as_float())
                    //     {
                    //         Ok(SoxBool::from((v1.value as f64) != v2.value).into_ref())
                    //     } else {
                    //         exc
                    //     }
                    // } else {
                    //     exc
                    // };
                    let left_type = left_val.sox_type(self);
                    let eq = left_type.slots.methods.iter().find(|v| v.0 == "equals");
                    let value = if let Some(entry) = eq {
                        let call_args = FuncArgs::new(vec![left_val.clone(), right_val.clone()]);
                        (entry.1.func)(self, call_args)
                    } else {
                        Ok(SoxBool::from(false).into_ref())
                    };
                    Ok(SoxBool::from(!value?.try_into_rust_bool(self)).into_ref())
                }
                TokenType::LessEqual => {
                    let exc = Err(Interpreter::runtime_error(
                        "Arguments to the less than or equals operator must both be numbers".into(),
                    ));
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxBool::from(v1.value <= v2.value).into_ref())
                    } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                        if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                            Ok(SoxBool::from(v1.value <= v2.value).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_float(), right_val.as_int())
                        {
                            Ok(SoxBool::from(v1.value <= (v2.value as f64)).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_int(), right_val.as_float())
                        {
                            Ok(SoxBool::from((v1.value as f64) <= v2.value).into_ref())
                        } else {
                            exc
                        }
                    } else {
                        exc
                    };
                    value
                }
                TokenType::GreaterEqual => {
                    let exc = Err(Interpreter::runtime_error(
                        "Arguments to the greater than or equals operator must both be numbers"
                            .into(),
                    ));
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxBool::from(v1.value >= v2.value).into_ref())
                    } else if left_val.as_float().is_some() || right_val.as_float().is_some() {
                        if let (Some(v1), Some(v2)) = (left_val.as_float(), right_val.as_float()) {
                            Ok(SoxBool::from(v1.value >= v2.value).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_float(), right_val.as_int())
                        {
                            Ok(SoxBool::from(v1.value >= (v2.value as f64)).into_ref())
                        } else if let (Some(v1), Some(v2)) =
                            (left_val.as_int(), right_val.as_float())
                        {
                            Ok(SoxBool::from((v1.value as f64) >= v2.value).into_ref())
                        } else {
                            exc
                        }
                    } else {
                        exc
                    };
                    value
                }
                TokenType::Bang => {
                    let value = right_val.try_into_rust_bool(self);
                    Ok(SoxBool::from(value).into_ref())
                }
                _ => Err(Interpreter::runtime_error("Unsupported token type".into())),
            }
        } else {
            Err(Interpreter::runtime_error(
                "Evaluation failed - called visit_binary_expr on non binary expression".into(),
            ))
        };
        value
    }

    fn visit_grouping_expr(&mut self, expr: &Expr) -> Self::T {
        let value = if let Expr::Grouping { expr } = expr {
            Ok(self.evaluate(expr)?)
        } else {
            Err(Interpreter::runtime_error(
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
                    let value = if let Some(v) = right.as_float() {
                        let new_val = SoxFloat { value: -v.value };
                        Ok(new_val.into_ref())
                    } else if let Some(v) = right.as_int() {
                        let new_val = SoxInt { value: -v.value };
                        Ok(new_val.into_ref())
                    } else {
                        Err(Interpreter::runtime_error(
                            "The unary operator (-) can only be applied to a numeric value."
                                .to_string(),
                        ))
                    };
                    value
                }

                TokenType::Bang => {
                    let value = right.try_into_rust_bool(self);
                    Ok(SoxBool::from(!value).into_ref())
                }
                _ => Err(Interpreter::runtime_error("Unknown unary operator.".into())),
            }
        } else {
            let error = Interpreter::runtime_error(
                "Evaluation failed - called visit_unary_expr on a non unary expression".to_string(),
            );
            Err(error)
        };
        value
    }

    fn visit_logical_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Logical {
            left,
            operator,
            right,
        } = expr
        {
            let left = self.evaluate(left)?;
            if operator.token_type == TokenType::Or {
                if left.clone().try_into_rust_bool(self) {
                    return Ok(left);
                }
            } else {
                if !left.clone().try_into_rust_bool(self) {
                    return Ok(left);
                }
            }
            self.evaluate(&right)
        } else {
            Err(Interpreter::runtime_error(
                "Evaluation failed - called visit_logical_expr on non logical expression."
                    .to_string(),
            ))
        }
    }

    fn visit_variable_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Variable { name } = expr {
            self.lookup_variable(name)
        } else {
            Err(Interpreter::runtime_error(
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
            let callee_type = callee_.sox_type(self);
            let callee_type_name = callee_type.name.clone().unwrap();
            let ret_val = match callee_type.slots.call {
                Some(fo) => {
                    let val = (fo)(callee_, call_args, self);
                    val
                }
                _ => Err(Interpreter::runtime_error(
                    format!("{} object is not callable.", callee_type_name),
                )),
            };
            ret_val
        } else {
            Err(Interpreter::runtime_error(
                "Can only call functions and classes".into(),
            ))
        }
    }
    fn visit_get_expr(&mut self, expr: &Expr) -> Self::T {
        let ret_val = if let Expr::Get { name, object } = expr {
            let object = self.evaluate(object)?;
            if let SoxObject::TypeInstance(inst) = object {
                //info!("Instance of type {:?}", inst.class(self));

                SoxInstance::get(inst, name.clone(), self)
            } else {
                Err(Interpreter::runtime_error(
                    "Only class instances have attributes".into(),
                ))
            }
        } else {
            Err(Interpreter::runtime_error(
                "Calling visit_get_expr on none get expr".into(),
            ))
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
            if let Some(v) = object.as_class_instance() {
                let value = self.evaluate(value)?;

                v.set(name.clone(), value.clone());
                Ok(value)
            } else {
                Err(Interpreter::runtime_error(
                    "Only instances have fields".into(),
                ))
            }
        } else {
            Err(Interpreter::runtime_error(
                "Calling visit_set_expr on none set expr".into(),
            ))
        };
        ret_val
    }
    fn visit_this_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::This { keyword } = expr {
            let value = self.lookup_variable(keyword);
            value
        } else {
            Err(Interpreter::runtime_error(
                "Calling visit_this_expr on none this expr".into(),
            ))
        }
    }
    fn visit_super_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Super { keyword, method } = expr {
            let (dist_to_ns, binding_idx) = self._locals.get(&keyword).unwrap();
            let this_token = Token::new(TokenType::This, "this".to_string(), Literal::None, 0);
            let (dist_to_ns2, binding_idx2) = self._locals.get(&this_token).unwrap();

            let key = ("super".to_string(), *dist_to_ns, *binding_idx);
            let key2 = ("this".to_string(), *dist_to_ns2, *binding_idx2);

            //let env = self.active_env_mut();
            let super_type = self.environment.get(key)?;
            let instance = self.environment.get(key2)?;

            let method = if let SoxObject::Type(v) = super_type {
                let c = v;
                let method_name = method.lexeme.clone();
                let method = c.find_method(method_name.as_str());
                let t = if let Some(m) = method {
                    if let Some(func) = m.as_func() {
                        let bound_method = func.bind(instance, self)?;
                        Ok(bound_method)
                    } else {
                        Err(Interpreter::runtime_error(format!(
                            "Undefined property {}",
                            method_name
                        )))
                    }
                } else {
                    Err(Interpreter::runtime_error(format!(
                        "Undefined property {}",
                        method_name
                    )))
                };
                t
            } else {
                Err(Interpreter::runtime_error(
                    "Unable to resolve instance - this".into(),
                ))
            };
            method
        } else {
            Err(Interpreter::runtime_error(
                "Calling visit_super_expr on none super expr".into(),
            ))
        }
    }
}

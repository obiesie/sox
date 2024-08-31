use std::collections::HashMap;

use log::{debug, info};
use slotmap::{DefaultKey, SlotMap};

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
use crate::environment::{Env, Namespace};
use crate::expr::Expr;
use crate::expr::ExprVisitor;
use crate::stmt::{Stmt, StmtVisitor};
use crate::token::{Literal, Token};
use crate::token_type::TokenType;

pub struct Interpreter {
    pub envs: SlotMap<DefaultKey, Env>,
    pub active_env_ref: DefaultKey,
    pub global_env_ref: DefaultKey,
    pub types: TypeLibrary,
    pub none: SoxRef<SoxNone>,
    pub locals: HashMap<(String, usize), (usize, usize)>,
}

impl Interpreter {
    pub fn new() -> Self {
        let environment = Env::default();
        let mut envs = SlotMap::new();
        let active_env_ref = envs.insert(environment);
        let types = TypeLibrary::init();
        let none = SoxRef::new(SoxNone {});
        let interpreter = Interpreter {
            envs,
            active_env_ref,
            global_env_ref: active_env_ref.clone(),
            types,
            none,
            locals: Default::default(),
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

    fn global_env_mut(&mut self) -> &mut Env {
        self.envs.get_mut(self.global_env_ref).unwrap()
    }

    fn active_env_mut(&mut self) -> &mut Env {
        self.envs.get_mut(self.active_env_ref).unwrap()
    }

    fn active_env(&self) -> &Env {
        self.envs.get(self.active_env_ref).unwrap()
    }

    pub fn referenced_env(&mut self, key: DefaultKey) -> &mut Env {
        self.envs.get_mut(key).unwrap()
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>) {
        for stmt in statements {
            info!("Executing statement -- {:?}", stmt);
            self.execute(stmt).expect("Runtime error");
        }
    }

    fn evaluate(&mut self, expr: &Expr) -> SoxResult {
        expr.accept(self)
    }

    fn execute(&mut self, stmt: &Stmt) -> SoxResult<()> {
        stmt.accept(self)
    }

    pub fn execute_block(
        &mut self,
        statements: Vec<&Stmt>,
        namespace: Option<Namespace>,
    ) -> SoxResult<()> {
        let active_env = self.active_env_mut();
        if let Some(ns) = namespace {
            active_env.push(ns)?;
            info!("The active env is {:?}", active_env);
        } else {
            active_env.new_namespace()?;
        }
        for statement in statements {
            let res = self.execute(statement);
            if let Err(v) = res {
                let active_env = self.active_env_mut();

                active_env.pop()?;
                return Err(v);
            }
        }
        let active_env = self.active_env_mut();
        active_env.pop()?;
        Ok(())
    }

    fn lookup_variable(&mut self, name: &Token, _expr: &Expr) -> SoxResult {
        if let Some(dist) = self.locals.get(&(name.lexeme.to_string(), name.line)) {
            let (dst, binding_idx) = dist;
            let active_env = self.envs.get_mut(self.active_env_ref).unwrap();
            let key = (name.lexeme.to_string(), *dst, *binding_idx);
            active_env.get(&key)
        } else {
            let global_env = self.global_env_mut();
            let val = global_env.find_and_get(name.lexeme.to_string());
            val
        }
    }

    pub fn runtime_error(msg: String) -> SoxObject {
        let error = Exception::Err(RuntimeError { msg });
        error.into_ref()
    }
}

impl StmtVisitor for &mut Interpreter {
    type T = SoxResult<()>;

    fn visit_expression_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let mut return_value = Ok(());
        if let Stmt::Expression(expr) = stmt {
            let value = self.evaluate(expr);
            return_value = match value {
                Ok(_) => Ok(()),
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
                    println!(">> {:?}", v);
                    Ok(())
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
            if let Some(initializer_stmt) = initializer {
                value = self.evaluate(initializer_stmt)?;
            }
            let active_env = self.active_env_mut();
            let name_ident = name.lexeme.to_string();
            active_env.define(name_ident, value)
        } else {
            return Err(Interpreter::runtime_error(
                "Evaluation failed - visiting a non declaration statement with visit_decl_stmt."
                    .to_string(),
            ));
        };
        Ok(())
    }

    fn visit_block_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Block(statements) = stmt {
            let stmts = statements.iter().map(|v| v).collect::<Vec<&Stmt>>();

            debug!("statements are {:?}", stmts);
            self.execute_block(stmts, None)?;

            Ok(())
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
        Ok(())
    }

    fn visit_while_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::While { condition, body } = stmt {
            let mut cond = self.evaluate(condition)?;
            while cond.try_into_rust_bool(self) {
                self.execute(body)?;
                cond = self.evaluate(&condition)?;
            }

            Ok(())
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
            params: _params,
            body: _body,
        } = stmt
        {
            let func_env = Env::default();
            let env_id = self.envs.insert(func_env);

            let stmt_clone = stmt.clone();
            let fo = SoxFunction::new(stmt_clone, env_id);
            let ns = {
                let active_env = self.active_env_mut();
                active_env.define(name.lexeme.clone(), fo.into_ref());
                active_env.namespaces.clone()
            };
            let func_env = self.envs.get_mut(env_id).unwrap();
            func_env.namespaces = ns;

            Ok(())
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
            return_value = self.evaluate(value)?;
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
            let active_env = self.active_env_mut();
            active_env.define(name.lexeme.to_string(), none_val);

            let prev_env = self.active_env_ref.clone();
            // setup super keyword within namespace
            if sc.is_some() {
                let env_ref = {
                    let active_env = self.active_env();
                    let mut env_copy = active_env.clone();
                    let namespace = Namespace::default();
                    env_copy.push(namespace)?;
                    let env_ref = self.envs.insert(env_copy);

                    env_ref
                };
                self.active_env_ref = env_ref;
                let env = self.referenced_env(env_ref);
                env.define("super", SoxObject::Type(sc.as_ref().unwrap().clone()))
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
                        declaration: Box::new(method.clone()),
                        environment_ref: self.active_env_ref.clone(),
                        is_initializer: name.lexeme == "init".to_string(),
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
            let (dist_to_binding, binding_idx) = self
                .locals
                .get(&(name.lexeme.to_string(), name.line))
                .unwrap();
            let key = (name.lexeme.to_string(), *dist_to_binding, *binding_idx);

            self.active_env_ref = prev_env;
            let active_env = self.active_env_mut();
            active_env.assign(&key, class.into_ref())?;

            Ok(())
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
            let dist = self.locals.get(&(name.lexeme.to_string(), name.line));
            if dist.is_some() {
                let (dst, idx) = dist.unwrap();
                // info!("Distance found from resolution is {dst}");
                let key = (name.lexeme.to_string(), *dst, *idx);

                let env = self.active_env_mut();
                env.assign(&key, eval_val.clone())?;
            } else {
                let global_env = self.global_env_mut();
                global_env.find_and_assign(name.lexeme.to_string(), eval_val.clone())?;
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
                Literal::Float(f) => self.new_float(f.clone()),
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
                    let value =
                        if let (Some(v1), Some(v2)) = (left_val.as_int(), right_val.as_int()) {
                            Ok(SoxInt::from(v1.value - v2.value).into_ref())
                        } else {
                            Err(Interpreter::runtime_error(
                                "Arguments to the minus operator must both be numbers".into(),
                            ))
                        };
                    value
                }
                TokenType::Rem => {
                    let value =
                        if let (Some(v1), Some(v2)) = (left_val.as_int(), right_val.as_int()) {
                            Ok(SoxInt::from(v1.value % v2.value).into_ref())
                        } else {
                            Err(Interpreter::runtime_error(
                                "Arguments to the remainder operator must both be numbers".into(),
                            ))
                        };
                    value
                }
                TokenType::Plus => {
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxInt::from(v1.value + v2.value).into_ref())
                    } else if let (Some(v1), Some(v2)) =
                        (left_val.as_string(), right_val.as_string())
                    {
                        Ok(SoxString::from(v1.value.clone() + v2.value.as_str()).into_ref())
                    } else {
                        Err(Interpreter::runtime_error(
                            "Arguments to the plus operator must both be strings or numbers".into(),
                        ))
                    };

                    value
                }
                TokenType::Star => {
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxInt::from(v1.value * v2.value).into_ref())
                    } else {
                        Err(Interpreter::runtime_error(
                            "Arguments to the multiplication operator must both be numbers".into(),
                        ))
                    };
                    value
                }
                TokenType::Slash => {
                    let value =
                        if let (Some(v1), Some(v2)) = (left_val.as_int(), right_val.as_int()) {
                            Ok(SoxFloat::from((v1.value as f64) / (v2.value as f64)).into_ref())
                        } else {
                            Err(Interpreter::runtime_error(
                                "Arguments to the division operator must both be numbers".into(),
                            ))
                        };
                    value
                }
                TokenType::Less => {
                    let value =
                        if let (Some(v1), Some(v2)) = (left_val.as_int(), right_val.as_int()) {
                            Ok(SoxBool::from(v1.value < v2.value).into_ref())
                        } else {
                            Err(Interpreter::runtime_error(
                                "Arguments to the less than operator must both be numbers".into(),
                            ))
                        };
                    value
                }
                TokenType::Greater => {
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
                        Ok(SoxBool::from(v1.value > v2.value).into_ref())
                    } else {
                        Err(Interpreter::runtime_error(
                            "Arguments to the greater than operator must both be numbers".into(),
                        ))
                    };
                    value
                }
                TokenType::EqualEqual => {
                    let value =
                        if let (Some(v1), Some(v2)) = (left_val.as_int(), right_val.as_int()) {
                            Ok(SoxBool::from(v1.value == v2.value).into_ref())
                        } else {
                            Err(Interpreter::runtime_error(
                                "Arguments to the equals operator must both be numbers".into(),
                            ))
                        };
                    value
                }
                TokenType::BangEqual => {
                    let value =
                        if let (Some(v1), Some(v2)) = (left_val.as_int(), right_val.as_int()) {
                            Ok(SoxBool::from(v1.value != v2.value).into_ref())
                        } else {
                            Err(Interpreter::runtime_error(
                                "Arguments to the not equals operator must both be numbers".into(),
                            ))
                        };
                    value
                }
                TokenType::LessEqual => {
                    let value =
                        if let (Some(v1), Some(v2)) = (left_val.as_int(), right_val.as_int()) {
                            Ok(SoxBool::from(v1.value <= v2.value).into_ref())
                        } else {
                            Err(Interpreter::runtime_error(
                                "Arguments to the less than or equals operator must both be numbers"
                                    .into(),
                            ))
                        };
                    value
                }
                TokenType::GreaterEqual => {
                    let value =
                        if let (Some(v1), Some(v2)) = (left_val.as_int(), right_val.as_int()) {
                            Ok(SoxBool::from(v1.value >= v2.value).into_ref())
                        } else {
                            Err(Interpreter::runtime_error(
                            "Arguments to the greater than or equals operator must both be numbers"
                                .into(),
                        ))
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
                    Ok(SoxBool::from(value).into_ref())
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
            self.lookup_variable(name, expr)
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
            let ret_val = match callee_type.slots.call {
                Some(fo) => {
                    let val = (fo)(callee_, call_args, self);
                    val
                }
                _ => Err(Interpreter::runtime_error(
                    "Callee evaluated to an object that is not callable.".into(),
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
                info!("Instance of type {:?}", inst.class(self));

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
            let value = self.lookup_variable(keyword, expr);
            value
        } else {
            Err(Interpreter::runtime_error(
                "Calling visit_this_expr on none this expr".into(),
            ))
        }
    }
    fn visit_super_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Super { keyword, method } = expr {
            let (dist_to_ns, binding_idx) = self
                .locals
                .get(&("super".to_string(), keyword.line))
                .unwrap();
            let (dist_to_ns2, binding_idx2) = self
                .locals
                .get(&("this".to_string(), keyword.line))
                .unwrap();

            let key = ("super".to_string(), *dist_to_ns, *binding_idx);
            let key2 = ("this".to_string(), *dist_to_ns2, *binding_idx2);

            let env = self.active_env_mut();
            let super_type = env.get(&key)?;
            let instance = env.get(&key2)?;

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

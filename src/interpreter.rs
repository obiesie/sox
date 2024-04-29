use log::{debug, info};
use slotmap::{DefaultKey, SlotMap};

use crate::builtins::bool_::SoxBool;
use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::float::SoxFloat;
use crate::builtins::function::SoxFunction;
use crate::builtins::int::SoxInt;
use crate::builtins::method::FuncArgs;
use crate::builtins::none::SoxNone;
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
    pub types: TypeLibrary,
}

impl Interpreter {
    pub fn new() -> Self {
        let environment = Env::default();
        let mut envs = SlotMap::new();
        let active_env_ref = envs.insert(environment);
        let types = TypeLibrary::init();
        let interpreter = Interpreter {
            envs,
            active_env_ref,
            types,
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

    fn active_env_mut(&mut self) -> &mut Env {
        return self.envs.get_mut(self.active_env_ref).unwrap();
    }

    fn active_env(&self) -> &Env {
        return self.envs.get(self.active_env_ref).unwrap();
    }

    pub fn referenced_env(&mut self, key: DefaultKey) -> &mut Env {
        return self.envs.get_mut(key).unwrap();
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>) {
        for stmt in statements {
            self.execute(stmt).expect("Runtime error");
        }
    }

    fn evaluate(&mut self, expr: &Expr) -> SoxResult {
        return expr.accept(self);
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
        let active_env = self.active_env_mut();
        let val = active_env.get(name.lexeme.as_str());
        return val;
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

            return Ok(());
        } else {
            return Err(Interpreter::runtime_error(
                "Evaluation failed - visited non block statement with visit_block_stmt."
                    .to_string(),
            ));
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
            return Err(Interpreter::runtime_error(
                "Evaluation failed -  visited non while statement with visit_while_stmt."
                    .to_string(),
            ));
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
            return Err(Interpreter::runtime_error(
                "Evaluation failed -  Calling a visit_function_stmt on non function node."
                    .to_string(),
            ));
        }
    }

    fn visit_return_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let mut return_value = SoxObject::None;
        if let Stmt::Return { keyword, value } = stmt {
            return_value = self.evaluate(value)?;
        }
        Err(Exception::Return(return_value).into_ref())
    }

    fn visit_class_stmt(&mut self, _stmt: &Stmt) -> Self::T {
        todo!()
    }
}

impl ExprVisitor for &mut Interpreter {
    type T = Result<SoxObject, SoxObject>;

    fn visit_assign_expr(&mut self, expr: &Expr) -> Self::T {
        let ret_val = if let Expr::Assign { name, value } = expr {
            let eval_val = self.evaluate(value)?;
            let env = self.active_env_mut();
            env.assign(name.lexeme.as_str(), eval_val.clone())?;
            // TODO should returned value be what is looked up?
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
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
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
                    let value = if let (Some(v1), Some(v2)) =
                        (left_val.as_int(), right_val.as_int())
                    {
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
        return value;
    }

    fn visit_unary_expr(&mut self, expr: &Expr) -> Self::T {
        let value = if let Expr::Unary { operator, right } = expr {
            let right = self.evaluate(right)?;
            match operator.token_type {
                TokenType::Minus => {
                    let value = if let Some(mut v) = right.as_float() {
                        let new_val = SoxFloat { value: -v.value };
                        Ok(new_val.into_ref())
                    } else if let Some(mut v) = right.as_int() {
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
        return value;
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
            return self.evaluate(&right);
        } else {
            Err(Interpreter::runtime_error(
                "Evaluation failed - called visit_logical_expr on non logical expression."
                    .to_string(),
            ))
        }
    }

    fn visit_variable_expr(&mut self, expr: &Expr) -> Self::T {
        return if let Expr::Variable { name } = expr {
            self.lookup_variable(name, expr)
        } else {
            Err(Interpreter::runtime_error(
                "Evaluation failed - called visit_variable_expr on non variable expr.".into(),
            ))
        };
    }

    fn visit_call_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Call {
            callee,
            paren,
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
    fn visit_get_expr(&mut self, _expr: &Expr) -> Self::T {
        todo!()
    }
    fn visit_set_expr(&mut self, _expr: &Expr) -> Self::T {
        todo!()
    }
    fn visit_this_expr(&mut self, _expr: &Expr) -> Self::T {
        todo!()
    }
    fn visit_super_expr(&mut self, _expr: &Expr) -> Self::T {
        todo!()
    }
}

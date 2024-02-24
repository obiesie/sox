use std::ops::Deref;

use log::{debug, info};
use slotmap::{DefaultKey, SlotMap};

use crate::core::SoxObject;
use crate::environment::Env;
use crate::exceptions::{Exception, RuntimeError};
use crate::expr::Expr;
use crate::expr::ExprVisitor;
use crate::payload;
use crate::stmt::{Stmt, StmtVisitor};
use crate::token::Token;
use crate::token_type::TokenType;

#[derive(Debug, Default)]
pub struct Interpreter {
    pub envs: SlotMap<DefaultKey, Env>,
    pub active_env_ref: DefaultKey,
}

impl Interpreter {
    pub fn new() -> Self {
        let environment = Env::default();
        let mut envs = SlotMap::new();
        let active_env_ref = envs.insert(environment);
        Interpreter {
            envs,
            active_env_ref,
        }
    }

    fn active_env(&mut self) -> &mut Env {
        return self.envs.get_mut(self.active_env_ref).unwrap();
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>) {
        for stmt in statements {
            self.execute(stmt).expect("Runtime error");
        }
    }

    fn evaluate(&mut self, expr: &Expr) -> Result<SoxObject, RuntimeError> {
        return expr.accept(self);
    }

    fn execute(&mut self, stmt: &Stmt) -> Result<(), Exception> {
        stmt.accept(self)
    }

    pub fn execute_block(&mut self, statements: Vec<&Stmt>) -> Result<(), Exception> {

        let active_env = self.active_env();
        active_env.new_namespace()?;


        for statement in statements {
            let res = self.execute(statement);
            if let Err(v) = res {
                let active_env = self.active_env();

                active_env.pop()?;
                return Err(v);
            }
        }
        let active_env = self.active_env();
        active_env.pop()?;
        Ok(())
    }

    fn is_truthy(&self, obj: &SoxObject) -> bool {
        let truth_value = match obj {
            SoxObject::Boolean(v) => v.clone(),
            _ => true,
        };
        truth_value
    }

    fn lookup_variable(&mut self, name: &Token, _expr: &Expr) -> Result<SoxObject, RuntimeError> {
        let active_env = self.active_env();
        let val = active_env.get(name.lexeme.as_str());
        return val;
    }
}

impl StmtVisitor for &mut Interpreter {
    type T = Result<(), Exception>;

    fn visit_expression_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let mut return_value = Ok(());
        if let Stmt::Expression(expr) = stmt {
            let value = self.evaluate(expr);
            return_value = match value {
                Ok(_) => {
                    Ok(())
                }
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
            Err(RuntimeError {
                msg: "Evaluation failed - visited non print statement with visit_print_stmt.".to_string(),
            }
            .into())
        };
        return_value
    }

    fn visit_decl_stmt(&mut self, stmt: &Stmt) -> Self::T {
        let mut value = SoxObject::None;
        if let Stmt::Var { name, initializer } = stmt {
            if let Some(initializer_stmt) = initializer {
                value = self.evaluate(initializer_stmt)?;
            }
            let active_env = self.active_env();
            let name_ident = name.lexeme.to_string();
            active_env.define(name_ident, value)
        } else {
            return Err(RuntimeError {
                msg: "Evaluation failed - visiting a non declaration statement with visit_decl_stmt".to_string(),
            }
            .into());
        }
        Ok(())
    }

    fn visit_block_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Block(statements) = stmt {
            let stmts = statements.iter().map(|v| v).collect::<Vec<&Stmt>>();

            debug!("statements are {:?}", stmts);
            self.execute_block(stmts)?;

            return Ok(());
        } else {
            return Err(RuntimeError {
                msg: "Evaluation failed - visited non block statement with visit_block_stmt".to_string(),
            }
            .into());
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
            if self.is_truthy(&cond_val) {
                self.execute(then_branch)?;
            } else if let Some(else_branch_stmt) = else_branch.as_ref() {
                self.execute(else_branch_stmt)?;
            }
        } else {
            return Err(RuntimeError {
                msg: "Evaluation failed - visited non if statement with visit_if_stmt".into(),
            }
            .into());
        }
        Ok(())
    }

    fn visit_while_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::While { condition, body } = stmt {
            let mut cond = self.evaluate(condition)?;
            while self.is_truthy(&cond) {
                self.execute(body)?;
                cond = self.evaluate(&condition)?;
            }

            Ok(())
        } else {
            Err(RuntimeError {
                msg: "Evaluation failed - visited non while statement with visit_while_stmt."
                    .into(),
            }
            .into())
        }
    }

    fn visit_function_stmt(&mut self, _stmt: &Stmt) -> Self::T {
        todo!()
    }

    fn visit_return_stmt(&mut self, stmt: &Stmt) -> Self::T {
      todo!()
    }

    fn visit_class_stmt(&mut self, _stmt: &Stmt) -> Self::T {
        todo!()
    }
}

impl ExprVisitor for &mut Interpreter {
    type T = Result<SoxObject, RuntimeError>;

    fn visit_assign_expr(&mut self, expr: &Expr) -> Self::T {
        let ret_val = if let Expr::Assign { name, value } = expr {
            let eval_val = self.evaluate(value)?;
            let env = self.active_env();
            env.assign(name.lexeme.as_str(), eval_val.clone())?;
            // TODO should returned value be what is looked up?
            Ok(eval_val)
        } else {
            Err(RuntimeError {
                msg: "Evaluation failed - called visit_assign_expr to process non assignment statement".into(),
            })
        };
        ret_val
    }

    fn visit_literal_expr(&mut self, expr: &Expr) -> Self::T {
        let value = if let Expr::Literal { value } = expr {
            let obj = SoxObject::from(value);
            Ok(obj)
        } else {
            Err(RuntimeError {
                msg: "Evaluation failed - called visit_literal_expr on a non literal expression"
                    .into(),
            })
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
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Int(v1 - v2))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the minus operator must both be numbers".into(),
                        })
                    };
                    value
                }
                TokenType::Rem => {
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Int(v1 % v2))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the min operator must both be numbers".into(),
                        })
                    };
                    value
                }
                TokenType::Plus => {
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Int(v1 + v2))
                    } else if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::String),
                        payload!(right_val, SoxObject::String),
                    ) {
                        Ok(SoxObject::String(v1 + v2.as_str()))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the plus operator must both be strings or numbers"
                                .into(),
                        })
                    };

                    value
                }
                TokenType::Star => {
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Int(v1 * v2))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the min operator must both be numbers".into(),
                        })
                    };
                    value
                }
                TokenType::Slash => {
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Float((v1 as f64) / (v2 as f64)))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the min operator must both be numbers".into(),
                        })
                    };
                    value
                }
                TokenType::Less => {
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Boolean(v1 < v2))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the min operator must both be numbers".into(),
                        })
                    };
                    value
                }
                TokenType::Greater => {
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Boolean(v1 > v2))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the min operator must both be numbers".into(),
                        })
                    };
                    value
                }
                TokenType::EqualEqual => {
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Boolean(v1 == v2))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the min operator must both be numbers".into(),
                        })
                    };
                    value
                }
                TokenType::BangEqual => {
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Boolean(v1 != v2))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the min operator must both be numbers".into(),
                        })
                    };
                    value
                }
                TokenType::LessEqual => {
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Boolean(v1 <= v2))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the min operator must both be numbers".into(),
                        })
                    };
                    value
                }
                TokenType::GreaterEqual => {
                    let value = if let (Some(v1), Some(v2)) = (
                        payload!(left_val, SoxObject::Int),
                        payload!(right_val, SoxObject::Int),
                    ) {
                        Ok(SoxObject::Boolean(v1 >= v2))
                    } else {
                        Err(RuntimeError {
                            msg: "Arguments to the min operator must both be numbers".into(),
                        })
                    };
                    value
                }
                TokenType::Bang => {
                    let value = self.is_truthy(&right_val);
                    Ok(SoxObject::Boolean(value))
                }
                _ => Err(RuntimeError {
                    msg: "Unsupported token type".into(),
                }),
            }
        } else {
            Err(RuntimeError {
                msg: "Evaluation failed - called visit_binary_expr on non binary expression".into(),
            })
        };
        value
    }

    fn visit_grouping_expr(&mut self, expr: &Expr) -> Self::T {
        let value = if let Expr::Grouping { expr } = expr {
            Ok(self.evaluate(expr)?)
        } else {
            Err(RuntimeError {
                msg: "Evaluation failed - called visit_grouping_expr on on group expr.".into(),
            })
        };
        return value;
    }

    fn visit_unary_expr(&mut self, expr: &Expr) -> Self::T {
        let value = if let Expr::Unary { operator, right } = expr {
            let right = self.evaluate(right)?;
            match operator.token_type {
                TokenType::Minus => {
                    let value = if let SoxObject::Float(v) = right {
                        Ok(SoxObject::Float(-v))
                    } else if let SoxObject::Int(v) = right {
                        Ok(SoxObject::Int(-v))
                    } else {
                        Err(RuntimeError {
                            msg: "The unary operator (-) can only be applied to a numeric value."
                                .into(),
                        })
                    };
                    value
                }

                TokenType::Bang => {
                    let value = self.is_truthy(&right);
                    Ok(SoxObject::Boolean(value))
                }
                _ => Err(RuntimeError {
                    msg: "Unknown unary operator.".into(),
                }),
            }
        } else {
            Err(RuntimeError {
                msg: "Evaluation failed - called visit_unary_expr on non unary expression".into(),
            })
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
                if self.is_truthy(&left) {
                    return Ok(left);
                }
            } else {
                if !(self.is_truthy(&left)) {
                    return Ok(left);
                }
            }
            return self.evaluate(&right);
        } else {
            Err(RuntimeError {
                msg: "Evaluation failed - called visit_logical_expr on non logical expression."
                    .into(),
            })
        }
    }

    fn visit_variable_expr(&mut self, expr: &Expr) -> Self::T {
        return if let Expr::Variable { name } = expr {
            self.lookup_variable(name, expr)
        } else {
            Err(RuntimeError {
                msg: "Evaluation failed - called visit_variable_expr on non variable expr.".into(),
            })
        };
    }

    fn visit_call_expr(&mut self, _expr: &Expr) -> Self::T {
        todo!()
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

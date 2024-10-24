use std::collections::HashMap;

use log::info;

use crate::expr::{Expr, ExprVisitor};
use crate::stmt::{Stmt, StmtVisitor};
use crate::token::{Literal, Token};
use crate::token_type::TokenType;

#[derive(Clone, Debug)]
pub enum ResolverError {
    NoScope,
    DuplicateVariable(String),
    NotFound(String),
    SyntaxError(String),
}

pub struct Resolver {
    scopes: Vec<Vec<(Token, bool)>>,
    current_function: FunctionType,
    current_class: ClassType,
    resolved_data: HashMap<(String, usize), (usize, usize)>,
    _resolved_data: HashMap<Token, (usize, usize)>,
}
#[derive(Clone, Debug, Eq, PartialEq, Copy)]

pub enum FunctionType {
    None,
    Function,
    Method,
    Initializer,
}

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum ClassType {
    None,
    Class,
    SubClass,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            scopes: vec![],
            current_function: FunctionType::None,
            current_class: ClassType::None,
            resolved_data: Default::default(),
            _resolved_data: Default::default(),
        }
    }

    pub fn begin_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    pub fn resolve(
        &mut self,
        statements: &Vec<Stmt>,
    ) -> Result<HashMap<Token, (usize, usize)>, ResolverError> {
        for stmt in statements {
            self.resolve_stmt(stmt.clone())?;
        }
        Ok(self._resolved_data.clone())
    }

    pub fn resolve_local(&mut self, expr: Expr, name: Token) -> Result<(), ResolverError> {
        for (dist_index, scope) in self.scopes.iter_mut().rev().enumerate() {
            let mut found = false;
            for idx in 0..scope.len() {
                let val = scope.get_mut(idx);
                if val.as_ref().unwrap().0.lexeme == name.lexeme.as_str() {
                    self._resolved_data.insert(name.clone(), (dist_index, idx));
                    found = true;
                }
            }
            if found {
                break;
            }
        }
        Ok(())
    }

    pub fn resolve_stmt(&mut self, stmt: Stmt) -> Result<(), ResolverError> {
        stmt.accept(self)
    }

    pub fn resolve_expr(&mut self, expr: &Expr) -> Result<(), ResolverError> {
        expr.accept(self)
    }

    pub fn end_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare(&mut self, name: Token) -> Result<(), ResolverError> {
        if self.scopes.is_empty() {
            return Ok(());
        }
        let scope = self.scopes.last_mut().unwrap(); // Handle potential None case if needed
        scope.push((name, false));
        Ok(())
    }

    pub fn define(&mut self, name: Token) -> Result<(), ResolverError> {
        if let Some(scope) = self.scopes.last_mut() {
            if let Some(entry) = scope
                .iter_mut()
                .find(|e| e.0.lexeme == name.lexeme.as_str() && e.0.line == name.line)
            {
                entry.1 = true;
            }
        }
        Ok(())
    }

    pub fn resolve_function(
        &mut self,
        stmt: Stmt,
        func_type: FunctionType,
    ) -> Result<(), ResolverError> {
        if let Stmt::Function { name, params, body } = stmt {
            let enclosing_function = self.current_function.clone();
            self.current_function = func_type;
            self.begin_scope();
            for param in params.iter() {
                self.declare(param.clone())?;
                self.define(param.clone())?;
            }
            self.resolve(&body)?;
            self.end_scope();

            self.current_function = enclosing_function;
        }
        Ok(())
    }
}

impl StmtVisitor for &mut Resolver {
    type T = Result<(), ResolverError>;

    fn visit_expression_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Expression(expr) = stmt {
            self.resolve_expr(expr)?;
        }
        Ok(())
    }

    fn visit_print_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Print(expr) = stmt {
            self.resolve_expr(expr)?;
        }
        Ok(())
    }

    fn visit_decl_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Var { name, initializer } = stmt {
            self.declare(name.clone())?;
            if let Some(init_val) = initializer {
                self.resolve_expr(init_val)?;
            }
            self.define(name.clone())?;
        }
        Ok(())
    }

    fn visit_block_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Block(stmts) = stmt {
            self.begin_scope();
            self.resolve(stmts)?;
            self.end_scope();
        }
        Ok(())
    }

    fn visit_if_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::If {
            condition,
            then_branch,
            else_branch,
        } = stmt
        {
            self.resolve_expr(condition)?;
            self.resolve_stmt(then_branch.as_ref().clone())?;
            if else_branch.is_some() {
                self.resolve_stmt(else_branch.as_ref().clone().unwrap())?;
            }
        }
        Ok(())
    }

    fn visit_while_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::While { condition, body } = stmt {
            self.resolve_expr(condition)?;
            self.resolve_stmt(body.as_ref().clone())?;
        }
        Ok(())
    }

    fn visit_function_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Function { name, params, body } = stmt {
            self.declare(name.clone())?;
            self.define(name.clone())?;
            self.resolve_function(stmt.clone(), FunctionType::Function)?;
        };
        Ok(())
    }

    fn visit_return_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if self.current_function == FunctionType::None {
            return Err(ResolverError::SyntaxError(
                "Return not allowed at top-level code.".into(),
            ));
        }
        if let Stmt::Return { keyword, value } = stmt {
            if value.is_some() {
                
                if self.current_function == FunctionType::Initializer  {
                    
                    return Err(ResolverError::SyntaxError(
                        "Cannot return value from initializer.".into(),
                    ));
                }
                self.resolve_expr(&value.clone().unwrap())?;
            }
        }
        Ok(())
    }

    fn visit_class_stmt(&mut self, stmt: &Stmt) -> Self::T {
        if let Stmt::Class {
            name,
            methods,
            superclass,
        } = stmt
        {
            let enclosing_class = self.current_class;
            self.current_class = ClassType::Class;

            self.declare(name.clone())?;
            self.define(name.clone())?;

            let class_name = name.clone();
            if let Some(sc) = superclass {
                self.current_class = ClassType::SubClass;
                if let Expr::Variable { name } = sc {
                    if name.lexeme == class_name.lexeme {
                        return Err(ResolverError::SyntaxError(
                            format!("Error at '{}': A class cannot inherit from itself.", name.lexeme),
                            
                        ));
                    }
                }
                self.resolve_expr(sc)?;

                self.begin_scope();
                let super_token =
                    Token::new(TokenType::Super, "super".to_string(), Literal::None, 0);
                self.scopes.last_mut().unwrap().push((super_token, true));
            }

            self.begin_scope();
            let this_token = Token::new(TokenType::This, "this".to_string(), Literal::None, 0);

            self.scopes.last_mut().unwrap().push((this_token, true));
            for method in methods.iter() {
                let dec = if let Stmt::Function { name, params, body } = method {
                    if name.lexeme == "init" {
                        FunctionType::Initializer
                    } else {
                        FunctionType::Method
                    }
                } else {
                    FunctionType::Method
                };
                self.resolve_function(method.clone(), dec)?;
            }
            self.end_scope();
            self.current_class = enclosing_class;
            if superclass.is_some() {
                self.end_scope();
            }
        }
        Ok(())
    }
}
impl ExprVisitor for &mut Resolver {
    type T = Result<(), ResolverError>;

    fn visit_assign_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Assign { name, value } = expr {
            self.resolve_expr(value)?;
            self.resolve_local(expr.clone(), name.clone())?;
        }
        Ok(())
    }

    fn visit_literal_expr(&mut self, expr: &Expr) -> Self::T {
        Ok(())
    }

    fn visit_binary_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Binary {
            left,
            right,
            operator,
        } = expr
        {
            self.resolve_expr(left.as_ref())?;
            self.resolve_expr(right.as_ref())?;
        }
        Ok(())
    }

    fn visit_grouping_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Grouping { expr } = expr {
            self.resolve_expr(expr.as_ref())?;
        }
        Ok(())
    }

    fn visit_unary_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Unary { right, operator } = expr {
            self.resolve_expr(right.as_ref())?;
        }
        Ok(())
    }

    fn visit_logical_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Logical {
            left,
            right,
            operator,
        } = expr
        {
            self.resolve_expr(left.as_ref())?;
            self.resolve_expr(right.as_ref())?;
        }
        Ok(())
    }

    fn visit_variable_expr(&mut self, expr: &Expr) -> Self::T {
        let mut ret_val = Ok(());
        if let Expr::Variable { name } = expr {
            if !self.scopes.is_empty()
                && self.scopes.last().is_some()
                && self
                    .scopes
                    .last()
                    .unwrap()
                    .iter()
                    .find(|v| v.0.lexeme == name.lexeme.as_str())
                    .is_some()
                && self
                    .scopes
                    .last()
                    .unwrap()
                    .iter()
                    .find(|v| v.0.lexeme == name.lexeme.as_str())
                    .unwrap()
                    .1
                    == false
            {
                ret_val = Err(ResolverError::SyntaxError(format!(
                    "Can't read local variable[{:?}] in its own initializer",
                    name.lexeme
                )))
            }
            self.resolve_local(expr.clone(), name.clone())?;
        }

        ret_val
    }

    fn visit_call_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Call {
            callee,
            paren,
            arguments,
        } = expr
        {
            self.resolve_expr(callee.as_ref())?;
            for arg in arguments {
                self.resolve_expr(arg)?;
            }
        };
        Ok(())
    }

    fn visit_get_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Get { name, object } = expr {
            self.resolve_expr(object)?;
        }
        Ok(())
    }

    fn visit_set_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Set {
            name,
            object,
            value,
        } = expr
        {
            self.resolve_expr(value)?;
            self.resolve_expr(object)?;
        };
        Ok(())
    }

    fn visit_this_expr(&mut self, expr: &Expr) -> Self::T {
        let res = if let Expr::This { keyword } = expr {
            if self.current_class == ClassType::None {
                Err(ResolverError::SyntaxError(
                    "Can't use 'this' outside of a class".into(),
                ))
            } else {
                self.resolve_local(expr.clone(), keyword.clone())?;
                Ok(())
            }
        } else {
            Err(ResolverError::SyntaxError(
                "Can't use func visit_this_expr on none this expression".into(),
            ))
        };
        res
    }

    fn visit_super_expr(&mut self, expr: &Expr) -> Self::T {
        if let Expr::Super { keyword, method } = expr {
            let res = if self.current_class == ClassType::None {
                Err(ResolverError::SyntaxError(
                    "Can't use 'super' outside of a class".into(),
                ))
            } else if self.current_class != ClassType::SubClass {
                Err(ResolverError::SyntaxError(
                    "Can't use 'super' in a class with no superclass".into(),
                ))
            } else {
                self.resolve_local(expr.clone(), keyword.clone())?;
                Ok(())
            };
            res
        } else {
            Err(ResolverError::SyntaxError(
                "Can't use func visit_super_expr on none super expression".into(),
            ))
        }
    }
}

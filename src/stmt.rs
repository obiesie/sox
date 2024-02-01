use crate::expr::Expr;
use crate::token::Token;

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Expression(Expr),
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Box<Option<Stmt>>,
    },
    Print(Expr),
    Return {
        keyword: Token,
        value: Expr,
    },
    Var {
        name: Token,
        initializer: Option<Expr>,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
    },
    Block(Vec<Stmt>),
    Function {
        name: Token,
        params: Vec<Token>,
        body: Vec<Stmt>,
    },
    Class {
        name: Token,
        superclass: Option<Expr>,
        methods: Vec<Stmt>,
    },
}

impl Stmt {
    pub(crate) fn accept<T: Visitor>(&self, mut visitor: T) -> T::T {
        match self {
            Stmt::Expression(_v) => visitor.visit_expression_stmt(self),
            Stmt::Print(_) => visitor.visit_print_stmt(self),
            Stmt::Var {
                name: _,
                initializer: _,
            } => visitor.visit_decl_stmt(self),
            Stmt::Block(_v) => visitor.visit_block_stmt(self),
            Stmt::If { .. } => visitor.visit_if_stmt(self),
            Stmt::While { .. } => visitor.visit_while_stmt(self),
            Stmt::Function { .. } => visitor.visit_function_stmt(self),
            Stmt::Return { .. } => visitor.visit_return_stmt(self),
            Stmt::Class { .. } => visitor.visit_class_stmt(self),
        }
    }
}

pub trait Visitor {
    type T;

    fn visit_expression_stmt(&mut self, stmt: &Stmt) -> Self::T;
    fn visit_print_stmt(&mut self, stmt: &Stmt) -> Self::T;
    fn visit_decl_stmt(&mut self, stmt: &Stmt) -> Self::T;
    fn visit_block_stmt(&mut self, stmt: &Stmt) -> Self::T;
    fn visit_if_stmt(&mut self, stmt: &Stmt) -> Self::T;
    fn visit_while_stmt(&mut self, stmt: &Stmt) -> Self::T;
    fn visit_function_stmt(&mut self, stmt: &Stmt) -> Self::T;
    //
    fn visit_return_stmt(&mut self, stmt: &Stmt) -> Self::T;
    fn visit_class_stmt(&mut self, stmt: &Stmt) -> Self::T;
}

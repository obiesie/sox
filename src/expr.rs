// use crate::objects::Object;
use crate::token::{Literal, Token};

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Assign {
        name: Token,
        value: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        paren: Token,
        arguments: Vec<Expr>,
    },
    Get {
        object: Box<Expr>,
        name: Token,
    },
    Grouping {
        expr: Box<Expr>,
    },
    Literal {
        value: Literal,
    },
    Variable {
        name: Token,
    },
    Logical {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Set {
        object: Box<Expr>,
        name: Token,
        value: Box<Expr>,
    },
    Super {
        keyword: Token,
        method: Token,
    },
    This {
        keyword: Token,
    },
    Unary {
        operator: Token,
        right: Box<Expr>,
    },
}

impl Expr {
    pub(crate) fn accept<T: ExprVisitor>(&self, mut visitor: T) -> T::T {
        match self {
            Expr::Assign { .. } => visitor.visit_assign_expr(&self),
            Expr::Binary { .. } => visitor.visit_binary_expr(&self),
            Expr::Grouping { .. } => visitor.visit_grouping_expr(&self),
            Expr::Literal { .. } => visitor.visit_literal_expr(&self),
            Expr::Unary { .. } => visitor.visit_unary_expr(&self),
            Expr::Variable { .. } => visitor.visit_variable_expr(&self),
            Expr::Logical { .. } => visitor.visit_logical_expr(&self),
            Expr::Call { .. } => visitor.visit_call_expr(&self),
            Expr::Get { .. } => visitor.visit_get_expr(&self),
            Expr::Set { .. } => visitor.visit_set_expr(&self),
            Expr::This { .. } => visitor.visit_this_expr(&self),
            Expr::Super { .. } => visitor.visit_super_expr(self),
        }
    }
}

pub trait ExprVisitor {
    type T;

    fn visit_assign_expr(&mut self, expr: &Expr) -> Self::T;
    fn visit_literal_expr(&mut self, expr: &Expr) -> Self::T;

    fn visit_binary_expr(&mut self, expr: &Expr) -> Self::T;
    fn visit_grouping_expr(&mut self, expr: &Expr) -> Self::T;
    fn visit_unary_expr(&mut self, expr: &Expr) -> Self::T;
    fn visit_logical_expr(&mut self, expr: &Expr) -> Self::T;
    fn visit_variable_expr(&mut self, expr: &Expr) -> Self::T;
    fn visit_call_expr(&mut self, expr: &Expr) -> Self::T;
    fn visit_get_expr(&mut self, expr: &Expr) -> Self::T;
    fn visit_set_expr(&mut self, expr: &Expr) -> Self::T;
    fn visit_this_expr(&mut self, expr: &Expr) -> Self::T;
    fn visit_super_expr(&mut self, expr: &Expr) -> Self::T;
}

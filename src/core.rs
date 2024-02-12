use std::fmt::Debug;

use crate::token::Literal;

#[macro_export]
macro_rules! payload {
    ($e:expr, $p:path) => {
        match $e {
            $p(v) => Some(v),
            _ => None
        }
    };
}

#[derive(Clone, Debug)]
pub enum SoxObject {
    Int(i64),
    String(String),
    Float(f64),
    Boolean(bool),
    None
}

impl From<&Literal> for SoxObject{
    fn from(value: &Literal) -> Self {
        match value {
            Literal::String(s) => {SoxObject::String(s.to_string())}
            Literal::Integer(i) => {SoxObject::Int(*i)}
            Literal::Float(f) => {SoxObject::Float(*f)}
            Literal::Boolean(b) => {SoxObject::Boolean(*b)}
            Literal::None => {SoxObject::None}
        }
    }
}

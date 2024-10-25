use crate::token_type::TokenType;
// use rand::Rng;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Literal {
    String(String),
    Integer(i64),
    Float(Float),
    Boolean(bool),
    None,
}

#[derive(Clone, Debug)]
pub struct Float(pub(crate) f64);

impl Eq for Float {}
impl PartialEq for Float {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits() || (self.0.is_nan() && other.0.is_nan())
    }
}

impl Hash for Float {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.0.is_nan() {
            state.write_u64(f64::NAN.to_bits());
        } else {
            state.write_u64(self.0.to_bits());
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub literal: Literal,
    pub line: usize,
    //pub id: u32,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: String, literal: Literal, line: usize) -> Self {
        // let mut rng = rand::thread_rng();
        Self {
            token_type,
            lexeme,
            literal,
            line,
            //id: rng.gen(),
        }
    }
}

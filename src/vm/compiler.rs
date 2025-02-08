use crate::builtins::int::SoxInt;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::object::core::{SoxObjectRef, SoxRef};
use crate::parser::SyntaxError;
use crate::token::Token;
use crate::token_type::TokenType;
use crate::vm::chunk::OpCode::{OpConstant, OpReturn};
use crate::vm::chunk::{Chunk, OpCode};
use std::borrow::Borrow;
use std::str::FromStr;

#[repr(u8)]
#[derive(Clone, Copy, PartialOrd, PartialEq, Debug, Hash, Eq)]
pub enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl Default for Precedence{
    fn default() -> Self {
        Precedence::None
    }
}

impl TryFrom<u8> for Precedence {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Assignment),
            2 => Ok(Self::Or),
            3 => Ok(Self::And),
            4 => Ok(Self::Equality),
            5 => Ok(Self::Comparison),
            6 => Ok(Self::Term),
            7 => Ok(Self::Factor),
            8 => Ok(Self::Unary),
            9 => Ok(Self::Call),
            10 => Ok(Self::Primary),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct ParseRule{
    pub infix_fn: Option<fn(&mut Compiler, &Interpreter) -> ()>,
    pub prefix_fn: Option<fn(&mut Compiler, &Interpreter) -> ()>,
    pub precedence: Precedence

}

impl ParseRule {
    pub fn new(infix_fn: Option<fn(&mut Compiler, &Interpreter) -> ()>,
               prefix_fn: Option<fn(&mut Compiler, &Interpreter) -> ()>,
               precedence: Precedence) -> Self{
        Self{
            infix_fn,
            prefix_fn,
            precedence
        }
    }
}



const PARSE_RULES: [ParseRule; 45] = {
    let mut data = [ParseRule::default(); 45];
    data[TokenType::LeftParen as usize] = ParseRule::new(
        Some(Compiler::grouping as fn(&mut Compiler, &Interpreter) -> ()),
        None,
        Precedence::None,
    );
    data[TokenType::RightParen as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::LeftBrace as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::RightBrace as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::LeftSqb as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::RightSqb as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Colon as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Comma as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Semi as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Minus as usize] = ParseRule::new(
        Some(Compiler::grouping as fn(&mut Compiler, &Interpreter) -> ()),
        Some(Compiler::binary as fn(&mut Compiler, &Interpreter) -> ()),
        Precedence::Term,
    );
    data[TokenType::Plus as usize] = ParseRule::new(
        None,
        Some(Compiler::binary as fn(&mut Compiler, &Interpreter) -> ()),
        Precedence::Term,
    );
    data[TokenType::Star as usize] = ParseRule::new(
        None,
        Some(Compiler::binary as fn(&mut Compiler, &Interpreter) -> ()),
        Precedence::Factor,
    );
    data[TokenType::Slash as usize] = ParseRule::new(
        None,
        Some(Compiler::binary as fn(&mut Compiler, &Interpreter) -> ()),
        Precedence::Factor,
    );
    data[TokenType::Dot as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Rem as usize] = ParseRule::new(None, None, Precedence::None);

    data[TokenType::Bang as usize] = ParseRule::new(
        Some(Compiler::unary as fn(&mut Compiler, &Interpreter) -> ()),
        None,
        Precedence::None,
    );
    data[TokenType::BangEqual as usize] = ParseRule::new(
        None,
        Some(Compiler::binary as fn(&mut Compiler, &Interpreter) -> ()),
        Precedence::Equality,
    );
    data[TokenType::Equal as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::EqualEqual as usize] = ParseRule::new(
        None,
        Some(Compiler::binary as fn(&mut Compiler, &Interpreter) -> ()),
        Precedence::Equality,
    );
    data[TokenType::Greater as usize] = ParseRule::new(
        None,
        Some(Compiler::binary as fn(&mut Compiler, &Interpreter) -> ()),
        Precedence::Comparison,
    );
    data[TokenType::GreaterEqual as usize] = ParseRule::new(
        None,
        Some(Compiler::binary as fn(&mut Compiler, &Interpreter) -> ()),
        Precedence::Comparison,
    );
    data[TokenType::Less as usize] = ParseRule::new(
        None,
        Some(Compiler::binary as fn(&mut Compiler, &Interpreter) -> ()),
        Precedence::Comparison,
    );
    data[TokenType::LessEqual as usize] = ParseRule::new(
        None,
        Some(Compiler::binary as fn(&mut Compiler, &Interpreter) -> ()),
        Precedence::Comparison,
    );

    data[TokenType::Identifier as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Number as usize] = ParseRule::new(
        Some(Compiler::number as fn(&mut Compiler, &Interpreter) -> ()),
        None,
        Precedence::None,
    );
    data[TokenType::SoxString as usize] = ParseRule::new(None, None, Precedence::None);

    data[TokenType::And as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Class as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Else as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::False as usize] = ParseRule::new(
        Some(Compiler::literal as fn(&mut Compiler, &Interpreter) -> ()),
        None,
        Precedence::None,
    );

    data[TokenType::For as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::If as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Or as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Return as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Super as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::True as usize] = ParseRule::new(
        Some(Compiler::literal as fn(&mut Compiler, &Interpreter) -> ()),
        None,
        Precedence::None,
    );
    data[TokenType::While as usize] = ParseRule::new(None, None, Precedence::None);

    data[TokenType::Def as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::This as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Let as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Print as usize] = ParseRule::new(None, None, Precedence::None);

    data[TokenType::None as usize] = ParseRule::new(
        Some(Compiler::literal as fn(&mut Compiler, &Interpreter) -> ()),
        None,
        Precedence::None,
    );
    data[TokenType::Error as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::EOF as usize] = ParseRule::new(None, None, Precedence::None);

    data
};

pub struct Compiler {
    chunk: Option<Chunk>,
    previous: Option<Token>,
    current: Option<Token>,
    lexer: Option<Lexer>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: None,
            previous: None,
            current: None,
            lexer: None,
        }
    }

    pub fn compile(&mut self, source: &'static str, chunk: Chunk, i: &Interpreter) -> Result<Chunk, ()> {
        let lexer = Lexer::new(source);
        self.chunk = Some(chunk);
        self.lexer = Some(lexer);
        self.advance();
        self.expression(i);
        let end = self.consume(TokenType::EOF, "Expect end of expression.");
        Ok(self.chunk.take().unwrap())
    }

    pub fn end(&mut self) {
        self.emit_return()
    }

    pub fn emit_constant(&mut self, value: SoxObjectRef) {
        let const_idx = self
            .make_constant(value.clone())
            .expect("Too many constants in one chunk");
        self.emit_byte((OpConstant, Some(const_idx as u8)))
    }

    pub fn make_constant(&mut self, value: SoxObjectRef) -> Result<usize, ()> {
        let idx = self.chunk.as_mut().unwrap().add_constant(value);
        if idx > i8::MAX as usize {
            Err(())
        } else {
            Ok(idx)
        }
    }

    pub fn emit_return(&mut self) {
        self.emit_byte((OpReturn, None));
    }

    pub fn emit_byte(&mut self, data: (OpCode, Option<u8>)) {
        let opcode = data.0;
        let operand = data.1;
        self.chunk.as_mut().unwrap()
            .write_chunk(opcode as u8, self.previous.as_ref().unwrap().line);
        if let Some(operand) = operand {
            self.chunk.as_mut().unwrap()
                .write_chunk(operand, self.previous.as_ref().unwrap().line);
        }
    }

    pub fn expression(&mut self, i: &Interpreter) {
        self.parse_with_precedence(Precedence::Assignment, i);
    }

    pub fn grouping(&mut self, i: &Interpreter) {
        self.expression(i);
        self.consume(TokenType::RightParen, "Expect ')' after expression.")
            .expect("TODO: panic message");
    }

    pub fn unary(&mut self, i: &Interpreter) {
        let operator_type = self.previous.as_ref().unwrap().token_type;
        self.parse_with_precedence(Precedence::Unary, i).expect("TODO: panic message");

        match operator_type {
            TokenType::Bang => {
                self.emit_byte((OpCode::OpNot, None));
                return;
            }
            TokenType::Minus => {
                self.emit_byte((OpCode::OpNegate, None));
                return;
            }
           
            _ => {
                return;
            }
        }
    }

    pub fn binary(&mut self, i: &Interpreter) {
        if let Some(token) = self.previous.as_ref() {
            let operator_type = token.token_type;
            let rule = self.get_rule(operator_type);
            let new_precedence = Precedence::try_from(rule.2 as u8 + 1).unwrap();
            self.parse_with_precedence(new_precedence, i);
            match operator_type {
                TokenType::Plus => {
                    self.emit_byte((OpCode::OpAdd, None));
                }
                TokenType::Minus => {
                    self.emit_byte((OpCode::OpSubtract, None));
                }
                TokenType::Star => {
                    self.emit_byte((OpCode::OpMultiply, None));
                }
                TokenType::Slash => {
                    self.emit_byte((OpCode::OpDivide, None));
                }
                _ => {
                    return;
                }
            } 
        }
    }

    pub fn literal(&mut self, i: &Interpreter) {
        let value = self.previous.as_ref().unwrap();
        match value.token_type {
            TokenType::False => {
                self.emit_byte((OpCode::OpFalse, None));
            }
            TokenType::True => self.emit_byte((OpCode::OpTrue, None)),
            TokenType::None => self.emit_byte((OpCode::OpNone, None)),
            _ => {}
        }
    }
    pub fn get_rule(&self, token_type: TokenType) -> &ParseRule {
        PARSE_RULES[token_type as usize].borrow()
    }

    pub fn parse_with_precedence(&mut self, precedence: Precedence, i: &Interpreter) -> Result<(), ()>{
        self.advance();
        let previous_token_type = self.previous.as_ref().map(|token| token.token_type);
        if let Some(previous_token_type) = previous_token_type {
            let prefix_rule = self.get_rule(previous_token_type).0;
            if let Some(prefix_rule_fn) = prefix_rule {
                prefix_rule_fn(self, i);
            } else {
                return Err(());
            }
        } else{
            return Err(());
        }
        while self.current.is_some() && precedence <= self.get_rule(self.current.as_ref().unwrap().token_type).2 {
            self.advance();
            let infix_rule = self.get_rule(self.previous.as_ref().unwrap().token_type).1;
            // TODO Handle error
            if let Some(infix_rule_fn) = infix_rule {
                infix_rule_fn(self, i)
            }
        }
        Ok(())
    }

    pub fn advance(&mut self) {
        self.previous = self.current.take();
        let token = self.lexer.as_mut().and_then(|lexer| lexer.next());
        if token.is_some(){
            self.current = token;
        }
    }

    pub fn consume(&mut self, expected_type: TokenType, message: &str) -> Result<(), SyntaxError> {
        if self.current.is_some() && self.current.as_ref().unwrap().token_type == expected_type {
            self.advance();
        }
        Err(SyntaxError {
            msg: format!(
                "Error after '{}': {}",
                self.previous.as_ref().unwrap().lexeme,
                message
            ),
            line: self.previous.as_ref().unwrap().line,
        })
    }

    pub fn number(&mut self, i: &Interpreter) {
        let val = i64::from_str(self.previous.as_ref().unwrap().lexeme).unwrap();
        let obj_payload = SoxInt { value: val };
        let obj = SoxRef::new_ref(obj_payload, i.types.int_type.to_owned());
        self.emit_constant(SoxObjectRef::from(obj));
    }
}

use crate::builtins::int::SoxInt;
use crate::builtins::string::SoxString;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::object::core::{SoxObjectRef, SoxRef};
use crate::parser::{SyntaxError, TO_IGNORE};
use crate::token::Token;
use crate::token_type::TokenType;
use crate::vm::chunk::OpCode::{OpConstant, OpGetGlobal, OpGetLocal, OpJump, OpJumpIfFalse, OpNone, OpPop, OpReturn, OpSetGlobal, OpSetLocal};
use crate::vm::chunk::{Chunk, OpCode};
use std::borrow::Borrow;
use std::iter::Peekable;
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

impl Default for Precedence {
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
pub struct ParseRule {
    pub infix_fn: Option<fn(&mut Compiler, &Interpreter) -> ()>,
    pub prefix_fn: Option<fn(&mut Compiler, &Interpreter) -> ()>,
    pub precedence: Precedence,
}

impl ParseRule {
    pub const fn const_default() -> Self {
        Self {
            infix_fn: None,
            prefix_fn: None,
            precedence: Precedence::None,
        }
    }

    pub const fn new(
        prefix_fn: Option<fn(&mut Compiler, &Interpreter) -> ()>,
        infix_fn: Option<fn(&mut Compiler, &Interpreter) -> ()>,
        precedence: Precedence,
    ) -> Self {
        Self {
            infix_fn,
            prefix_fn,
            precedence,
        }
    }
}

const PARSE_RULES: [ParseRule; 45] = {
    let mut data = [ParseRule::const_default(); 45];
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
        Some(Compiler::unary as fn(&mut Compiler, &Interpreter) -> ()),
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

    data[TokenType::Identifier as usize] = ParseRule::new(
        Some(Compiler::variable as fn(&mut Compiler, &Interpreter)),
        None,
        Precedence::None,
    );
    data[TokenType::Number as usize] = ParseRule::new(
        Some(Compiler::number as fn(&mut Compiler, &Interpreter) -> ()),
        None,
        Precedence::None,
    );
    data[TokenType::SoxString as usize] = ParseRule::new(
        Some(Compiler::string as fn(&mut Compiler, &Interpreter) -> ()),
        None,
        Precedence::None,
    );

    data[TokenType::And as usize] = ParseRule::new(None, Some(Compiler::and_ as fn(&mut Compiler, &Interpreter)), Precedence::And);
    data[TokenType::Class as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Else as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::False as usize] = ParseRule::new(
        Some(Compiler::literal as fn(&mut Compiler, &Interpreter) -> ()),
        None,
        Precedence::None,
    );

    data[TokenType::For as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::If as usize] = ParseRule::new(None, None, Precedence::None);
    data[TokenType::Or as usize] = ParseRule::new(None, Some(Compiler::or_ as fn(&mut Compiler, &Interpreter)), Precedence::Or);
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

#[derive(Clone, Debug)]
pub struct Local {
    name: String,
    depth: Option<usize>,
}

pub struct Compiler {
    chunk: Option<Chunk>,
    previous: Option<Token>,
    current: Option<Token>,
    tokens: Option<Peekable<Lexer>>,
    can_assign: bool,
    locals: Vec<Local>,
    scope_depth: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: None,
            previous: None,
            current: None,
            tokens: None,
            can_assign: false,
            scope_depth: 0,
            locals: Vec::new(),
        }
    }

    pub fn compile(
        &mut self,
        source: &'static str,
        chunk: Chunk,
        i: &Interpreter,
    ) -> Result<Chunk, ()> {
        let lexer = Lexer::new(source);
        self.chunk = Some(chunk);
        self.tokens = Some(lexer.peekable());
        self.advance();
        while !self.match_token(vec![TokenType::EOF]) && self.current.is_some() {
            self.declaration(i);
        }
        Ok(self.chunk.take().unwrap())
    }

    fn match_token(&mut self, token_types: Vec<TokenType>) -> bool {
        for token_type in token_types {
            if self.check(token_type) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&mut self, token_type: TokenType) -> bool {
        if !(self.current.is_some() && self.current.as_ref().unwrap().token_type == token_type) {
            return false;
        }
        return true;
    }

    pub fn declaration(&mut self, i: &Interpreter) {
        if self.match_token(vec![TokenType::Let]) {
            self.let_declaration(i);
        } else {
            self.statement(i);
        }
    }

    pub fn let_declaration(&mut self, i: &Interpreter) {
        let global = self.parse_variable(i, "Expect variable name.");
        if self.match_token(vec![TokenType::Equal]) {
            self.expression(i);
        } else {
            self.emit_instruction_bytes((OpNone, None))
        }
        self.consume(TokenType::Semi, "Expect ';' after variable declaration.").expect("");
        self.define_variable(global, i);

    }

    pub fn define_variable(&mut self, global: Option<usize>, i: &Interpreter) {
        if self.scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.emit_instruction_bytes((OpCode::OpDefineGlobal, Some(global.unwrap() as u8)));
    }

    pub fn mark_initialized(&mut self) {
        self.locals.last_mut().unwrap().depth = Some(self.scope_depth);
    }
    pub fn parse_variable(&mut self, i: &Interpreter, message: &str) -> Option<usize> {
        self.consume(TokenType::Identifier, message).expect("TODO: panic message");

        if self.scope_depth > 0 {
            self.declare_variable(i);
            return None;
        }
        let global =
            self.identifier_constant(self.previous.as_ref().unwrap().lexeme.to_string(), i);
        return Some(global);
    }

    pub fn declare_variable(&mut self, i: &Interpreter) {
        if self.scope_depth == 0 {
            return;
        }
        let name = self.previous.as_ref().unwrap().lexeme.to_string();

        for local in self.locals.iter().rev() {
            if local.depth.is_some() && (local.depth.unwrap() < self.scope_depth) {
                break;
            }
            if local.name == name {
                panic!("Variable with this name already declared in this scope.");
            }
        }
        self.add_local(name)
    }

    pub fn add_local(&mut self, name: String) {
        let local = Local { name, depth: None };
        self.locals.push(local);
    }

    pub fn identifier_constant(&mut self, name: String, i: &Interpreter) -> usize {
        let constant_value = SoxObjectRef::from(i.new_string(name.parse().unwrap()));
        let constant = self.make_constant(constant_value).unwrap();
        constant
    }
    pub fn statement(&mut self, i: &Interpreter) {
        if let Some(token) = self.current.as_ref() {
            if self.match_token(vec![TokenType::Print]) {
                self.print_statement(i);
            } else if self.match_token(vec![TokenType::For]){
                self.for_statement(i)
            } else if self.match_token(vec![TokenType::If]){
               self.if_statement(i);  
            } else if self.match_token(vec![TokenType::While]) {
              self.while_statement(i);  
            } else if self.match_token(vec![TokenType::LeftBrace]) {
                self.begin_scope(i);
                self.block(i);
                self.end_scope(i);
            }  else {
                self.expression_statement(i);
            }
        }
    }
    
    pub fn and_(&mut self, i: &Interpreter) {
        let end_jump = self.emit_jump(OpJumpIfFalse);
        self.emit_instruction_bytes((OpPop, None));
        
        self.parse_with_precedence(Precedence::And, i).expect("TODO: panic message");
        self.patch_jump(end_jump);
    }


    pub fn or_(&mut self, i: &Interpreter) {
        let else_jump = self.emit_jump(OpJumpIfFalse);
        let end_jump = self.emit_jump(OpJump);
        
        self.patch_jump(else_jump);
        self.emit_instruction_bytes((OpPop, None));

        self.parse_with_precedence(Precedence::Or, i).expect("TODO: panic message");
        self.patch_jump(end_jump);

    }
    
    pub fn for_statement(&mut self, i: &Interpreter){
        self.begin_scope(i);
        
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'");
        if self.match_token(vec![TokenType::Semi]){
            
        } else if self.match_token(vec![TokenType::Let]){
            self.let_declaration(i);
        } else{
            self.expression_statement(i);
        }
        let mut loop_start = self.chunk.as_ref().unwrap().code.len();

        let exit_jump = None;
        if !self.match_token(vec![TokenType::Semi]){
            self.expression(i);
            self.consume(TokenType::Semi, "Expect ';' after variable declaration.").expect("");

            let exit_jump = Some(self.emit_jump(OpJumpIfFalse));
            self.emit_instruction_bytes((OpPop, None));
        }
        if !self.match_token(vec![TokenType::RightParen]) {
            let body_jump = self.emit_jump(OpJump);
            let increment_start = self.chunk.as_ref().unwrap().code.len();
            self.expression(i);
            self.emit_instruction_bytes((OpPop, None));
            self.consume(TokenType::RightParen, "Expect ')' after for clauses").expect("");
            
            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }
        
        self.statement(i);
        self.emit_loop(loop_start);
        if exit_jump.is_some() {
            self.patch_jump(exit_jump.unwrap());
            self.emit_instruction_bytes((OpPop, None));
        }
        self.end_scope(i)
    }
    
    pub fn while_statement(&mut self, i: &Interpreter){
        let loop_start = self.chunk.as_ref().unwrap().code.len();
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression(i);
        self.consume(TokenType::RightParen, "Expect ')' after 'while'.");
        
        let exit_jump = self.emit_jump(OpJumpIfFalse);
        self.emit_instruction_bytes((OpPop, None));
        self.statement(i);
        
        self.emit_loop(loop_start);
        
        self.patch_jump(exit_jump);
        self.emit_instruction_bytes((OpPop, None));
    }
    
    pub fn if_statement(&mut self, i: &Interpreter){
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression(i);
        self.consume(TokenType::RightParen, "Expect '(' after 'if'.");
        let then_jump = self.emit_jump(OpJumpIfFalse);
       
        self.emit_instruction_bytes((OpPop, None));
        self.statement(i);
        let jump = self.emit_jump(OpJump);
        self.patch_jump(then_jump);

        self.emit_instruction_bytes((OpPop, None));
        if self.match_token(vec![TokenType::Else]){
            self.statement(i);
        }
        
        self.patch_jump(jump);
    }

    fn emit_loop(&mut self, loop_start: usize){
        self.emit_instruction_bytes((OpCode::OpLoop, None));
        
        let offset = self.chunk.as_ref().unwrap().code.len() - loop_start + 2;
        if offset as i16 > i16::MAX {
            panic!("loop body too large.");
        }
        
        self.emit_bytes(((offset >> 8) & 0xff) as u8);
        self.emit_bytes((offset & 0xff) as u8);
    }
    fn emit_jump(&mut self, op_code: OpCode) -> usize{
        self.emit_instruction_bytes((op_code, None));
        self.emit_instruction_bytes((OpNone, None));
        self.emit_instruction_bytes((OpNone, None));
        return self.chunk.as_ref().unwrap().code.len() - 2
    }
    
    fn patch_jump(&mut self, offset: usize){
        let code_len = self.chunk.as_ref().unwrap().code.len();
        let jump = code_len - offset - 2;
        
        if jump as i16 > i16::MAX {
            panic!("Too much code to jump over!")
        }
        self.chunk.as_mut().unwrap().code[offset] = ((jump >> 8) & 0xff) as u8;
        self.chunk.as_mut().unwrap().code[offset + 1] = (jump & 0xff) as u8;

    }
    
    pub fn begin_scope(&mut self, i: &Interpreter) {
        self.scope_depth += 1;
    }

    pub fn end_scope(&mut self, interpreter: &Interpreter) {
        self.scope_depth -= 1;
        let mut local_count = self.locals.len();
        while local_count > 0
            && self.locals[local_count-1].depth.unwrap() > self.scope_depth
        {
            self.emit_instruction_bytes((OpCode::OpPop, None));
            local_count -= 1;
        }
    }

    pub fn block(&mut self, i: &Interpreter) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.declaration(i);
        }
        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    pub fn print_statement(&mut self, i: &Interpreter) {
        self.expression(i);
        self.consume(TokenType::Semi, "Expect ';' after value.");
        self.emit_instruction_bytes((OpCode::OpPrint, None));
    }

    pub fn expression_statement(&mut self, i: &Interpreter) {
        self.expression(i);
        self.consume(TokenType::Semi, "Expect ';' after expression.");
        self.emit_instruction_bytes((OpCode::OpPop, None));
    }
    pub fn end(&mut self) {
        self.emit_return()
    }

    pub fn emit_constant(&mut self, value: SoxObjectRef) {
        let const_idx = self
            .make_constant(value.clone())
            .expect("Too many constants in one chunk");
        self.emit_instruction_bytes((OpConstant, Some(const_idx as u8)))
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
        self.emit_instruction_bytes((OpReturn, None));
    }

    pub fn emit_instruction_bytes(&mut self, data: (OpCode, Option<u8>)) {
        let opcode = data.0;
        let operand = data.1;
        self.emit_bytes(opcode as u8);
       
        if let Some(operand) = operand {
            self.emit_bytes(operand);
        }
    }
    
    fn emit_bytes(&mut self, data: u8){
        self.chunk.as_mut().unwrap().write_chunk(data, self.previous.as_ref().unwrap().line);
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
        self.parse_with_precedence(Precedence::Unary, i)
            .expect("TODO: panic message");

        match operator_type {
            TokenType::Bang => {
                self.emit_instruction_bytes((OpCode::OpNot, None));
                return;
            }
            TokenType::Minus => {
                self.emit_instruction_bytes((OpCode::OpNegate, None));
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
            let new_precedence = Precedence::try_from(rule.precedence as u8 + 1).unwrap();
            self.parse_with_precedence(new_precedence, i);
            match operator_type {
                TokenType::Plus => {
                    self.emit_instruction_bytes((OpCode::OpAdd, None));
                }
                TokenType::Minus => {
                    self.emit_instruction_bytes((OpCode::OpSubtract, None));
                }
                TokenType::Star => {
                    self.emit_instruction_bytes((OpCode::OpMultiply, None));
                }
                TokenType::Slash => {
                    self.emit_instruction_bytes((OpCode::OpDivide, None));
                }
                TokenType::Less => {
                    self.emit_instruction_bytes((OpCode::OpLess, None));
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
                self.emit_instruction_bytes((OpCode::OpFalse, None));
            }
            TokenType::True => self.emit_instruction_bytes((OpCode::OpTrue, None)),
            TokenType::None => self.emit_instruction_bytes((OpCode::OpNone, None)),

            _ => {}
        }
    }
    pub fn get_rule(&self, token_type: TokenType) -> &ParseRule {
        PARSE_RULES[token_type as usize].borrow()
    }

    pub fn parse_with_precedence(
        &mut self,
        precedence: Precedence,
        i: &Interpreter,
    ) -> Result<(), ()> {
        self.advance();
        let previous_token_type = self.previous.as_ref().map(|token| token.token_type);
        if let Some(previous_token_type) = previous_token_type {
            let prefix_rule = self.get_rule(previous_token_type).prefix_fn;
            if let Some(prefix_rule_fn) = prefix_rule {
                self.can_assign = precedence <= Precedence::Assignment;
                prefix_rule_fn(self, i);
            } else {
                return Err(());
            }
        } else {
            return Err(());
        }
        while self.current.is_some()
            && precedence
                <= self
                    .get_rule(self.current.as_ref().unwrap().token_type)
                    .precedence
        {
            self.advance();
            let infix_rule = self
                .get_rule(self.previous.as_ref().unwrap().token_type)
                .infix_fn;
            if let Some(infix_rule_fn) = infix_rule {
                infix_rule_fn(self, i)
            }
        }
        if self.can_assign && self.match_token(vec![TokenType::Equal]) {
            panic!("Invalid assignment target.");
        }
        Ok(())
    }

    pub fn advance(&mut self) {
        self.previous = self.current.take();
        while let Some(_) = self
            .tokens
            .as_mut()
            .unwrap()
            .next_if(|token| TO_IGNORE.contains(&token.token_type))
        {}
        let token = self.tokens.as_mut().and_then(|lexer| lexer.next());
            self.current = token;
    }

    pub fn consume(&mut self, expected_type: TokenType, message: &str) -> Result<(), SyntaxError> {
        if self.current.is_some() && self.current.as_ref().unwrap().token_type == expected_type {
            self.advance();
            return Ok(());
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

    pub fn string(&mut self, i: &Interpreter) {
        let obj_payload = SoxString {
            value: self.previous.as_ref().unwrap().lexeme.to_string(),
        };
        let obj = SoxRef::new_ref(obj_payload, i.types.str_type.to_owned());
        self.emit_constant(SoxObjectRef::from(obj));
    }

    pub fn variable(&mut self, i: &Interpreter) {
        self.named_variable(self.previous.as_ref().unwrap().lexeme.to_string(), i);
    }

    pub fn named_variable(&mut self, name: String, i: &Interpreter) {
        let get_op;
        let set_op;
        let mut arg = self.resolve_local(name.clone(), i);
        if let Some(arg) = arg {
            get_op = OpGetLocal;
            set_op = OpSetLocal;
        } else {
            arg = Some(self.identifier_constant(name, i) as u8);
            get_op = OpGetGlobal;
            set_op = OpSetGlobal;
        }
        if self.match_token(vec![TokenType::Equal]) && self.can_assign {
            self.expression(i);
            self.emit_instruction_bytes((set_op, arg))
        } else {
            self.emit_instruction_bytes((get_op, arg))
        }
    }

    pub fn resolve_local(&mut self, name: String, i: &Interpreter) -> Option<u8> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                if local.depth.is_none() {
                    panic!("Can't read local variable in its own initializer.");
                }
                return Some(i as u8);
            }
        }
        None
    }
}

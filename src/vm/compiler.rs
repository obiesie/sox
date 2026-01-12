use crate::builtins::chunk::OpCode::{
    OpConstant, OpGetGlobal, OpGetLocal, OpGetUpvalue, OpJump, OpJumpIfFalse, OpNone, OpPop,
    OpReturn, OpSetGlobal, OpSetLocal, OpSetUpvalue,
};
use crate::builtins::chunk::{Chunk, OpCode};
use crate::builtins::function::SoxFunction;
use crate::builtins::int::SoxInt;
use crate::builtins::module::SoxModule;
use crate::builtins::string::SoxString;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::object::core::{SoxObjectRef, SoxRef};
use crate::parser::{SyntaxError, TO_IGNORE};
use crate::token::Token;
use crate::token_type::TokenType;
use log::info;
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

/// Compile-time error type that unifies syntax errors and semantic errors.
#[derive(Debug)]
pub enum CompileError {
    Syntax(SyntaxError),
    TooManyParameters { line: usize, token: String },
    TooManyArguments { line: usize, token: String },
    TooManyUpvalues { line: usize },
    TooManyConstants { line: usize },
    JumpTooLarge { line: usize },
    LoopBodyTooLarge { line: usize },
    InvalidAssignmentTarget { line: usize },
    VariableAlreadyDeclared { name: String, line: usize },
    VariableInOwnInitializer { name: String, line: usize },
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Syntax(e) => write!(f, "[line {}] {}", e.line, e.msg),
            CompileError::TooManyParameters { line, token } => write!(
                f,
                "[line {}] Error at '{}': Can't have more than 255 parameters.",
                line, token
            ),
            CompileError::TooManyArguments { line, token } => write!(
                f,
                "[line {}] Error at '{}': Can't have more than 255 arguments.",
                line, token
            ),
            CompileError::TooManyUpvalues { line } => write!(
                f,
                "[line {}] Error: Too many closure variables in function.",
                line
            ),
            CompileError::TooManyConstants { line } => {
                write!(f, "[line {}] Error: Too many constants in one chunk.", line)
            }
            CompileError::JumpTooLarge { line } => {
                write!(f, "[line {}] Error: Too much code to jump over.", line)
            }
            CompileError::LoopBodyTooLarge { line } => {
                write!(f, "[line {}] Error: Loop body too large.", line)
            }
            CompileError::InvalidAssignmentTarget { line: _ } => {
                write!(f, "Error at '=': Invalid assignment target.")
            }
            CompileError::VariableAlreadyDeclared { name, line } => write!(
                f,
                "[line {}] Error: Already a variable named '{}' in this scope.",
                line, name
            ),
            CompileError::VariableInOwnInitializer { name, line } => write!(
                f,
                "[line {}] Error: Can't read local variable '{}' in its own initializer.",
                line, name
            ),
        }
    }
}

impl From<SyntaxError> for CompileError {
    fn from(e: SyntaxError) -> Self {
        CompileError::Syntax(e)
    }
}

#[derive(Clone, Copy, Default)]
pub struct ParseRule {
    pub infix_fn: Option<fn(&mut Compiler, &Interpreter) -> Result<(), CompileError>>,
    pub prefix_fn: Option<fn(&mut Compiler, &Interpreter) -> Result<(), CompileError>>,
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
        prefix_fn: Option<fn(&mut Compiler, &Interpreter) -> Result<(), CompileError>>,
        infix_fn: Option<fn(&mut Compiler, &Interpreter) -> Result<(), CompileError>>,
        precedence: Precedence,
    ) -> Self {
        Self {
            infix_fn,
            prefix_fn,
            precedence,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Local {
    name: String,
    depth: Option<usize>,
    is_captured: bool,
}

pub enum CompiledUnitType {
    Module,
    Function,
}

pub enum CompiledUnit {
    Module(SoxModule),
    Function(SoxFunction),
}

pub trait Compilable {
    fn new_empty(name: String) -> Self;

    fn new(name: String, chunk: Chunk) -> Self;
}

pub struct Parser {
    pub previous: Option<Token>,
    pub current: Option<Token>,
    tokens: Peekable<Lexer>,
    pub had_error: bool,
    pub panic_mode: bool,
}

impl Parser {
    pub fn new(source: &'static str) -> Self {
        let lexer = Lexer::new(source);
        let parser = Parser {
            previous: None,
            current: None,
            tokens: lexer.peekable(),
            had_error: false,
            panic_mode: false,
        };
        parser
    }

    /// Advances the parser to the next token, updating `self.parser.previous` and `self.parser.current`.
    pub fn advance(&mut self) {
        self.previous = self.current.take();
        self.skip_ignorable_tokens();

        self.current = self.tokens.next();
    }

    fn skip_ignorable_tokens(&mut self) {
        while let Some(_) = self
            .tokens
            .next_if(|token| TO_IGNORE.contains(&token.token_type))
        {}
    }

    /// Consumes the current token if it matches the expected type, otherwise returns an error.
    pub fn consume(&mut self, expected_type: TokenType, message: &str) -> Result<(), SyntaxError> {
        if matches!(&self.current, Some(token) if token.token_type == expected_type) {
            self.advance();
            Ok(())
        } else {
            let (current_lexeme, line) = self
                .current
                .as_ref()
                .map_or(("end", self.previous.as_ref().map_or(0, |t| t.line)), |t| {
                    (t.lexeme, t.line)
                });

            Err(SyntaxError {
                msg: format!("Error at '{}': {}", current_lexeme, message),
                line,
            })
        }
    }

    /// Checks if the current token matches a given type without consuming it.
    pub fn check(&self, token_type: TokenType) -> bool {
        self.current.is_some() && self.current.as_ref().unwrap().token_type == token_type
    }

    /// Consumes the current token if it matches any of the provided types.
    pub fn match_token(&mut self, token_types: Vec<TokenType>) -> bool {
        for token_type in token_types {
            if self.check(token_type) {
                self.advance();
                return true;
            }
        }
        false
    }

    /// Reports an error at the current token. Sets panic_mode to suppress cascading errors.
    pub fn error_at_current(&mut self, message: &str) {
        if self.panic_mode {
            return; // Suppress cascading errors
        }
        self.panic_mode = true;
        self.had_error = true;

        let (lexeme, line) = self
            .current
            .as_ref()
            .map_or(("<eof>", 0), |t| (t.lexeme, t.line));
        eprintln!("[line {}] Error at '{}': {}", line, lexeme, message);
    }

    /// Reports an error at the previous token.
    pub fn error(&mut self, message: &str) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;
        self.had_error = true;

        let (lexeme, line) = self
            .previous
            .as_ref()
            .map_or(("<eof>", 0), |t| (t.lexeme, t.line));
        eprintln!("[line {}] Error at '{}': {}", line, lexeme, message);
    }

    /// Synchronize after an error - skip to the next statement boundary
    pub fn synchronize(&mut self) {
        self.panic_mode = false;

        while self.current.is_some() {
            // If we just passed a semicolon, we're at a statement boundary
            if self
                .previous
                .as_ref()
                .map_or(false, |t| t.token_type == TokenType::Semi)
            {
                return;
            }

            // Check if current token starts a new statement
            if let Some(token) = &self.current {
                match token.token_type {
                    TokenType::Class
                    | TokenType::Def
                    | TokenType::Let
                    | TokenType::For
                    | TokenType::If
                    | TokenType::While
                    | TokenType::Print
                    | TokenType::Return => return,
                    _ => {}
                }
            }

            self.advance();
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new("")
    }
}

#[derive(Clone)]
pub struct CompilerContext {
    can_assign: bool,
    upvalue_count: usize,
    locals: Vec<Local>,
    scope_depth: usize,
    chunk: Chunk,
    upvalues: [UpValue; 256],
}

impl Default for CompilerContext {
    fn default() -> Self {
        let mut locals = Vec::new();
        let local = Local {
            name: "".to_string(),
            depth: Some(0),
            is_captured: false,
        };
        locals.push(local);
        Self {
            can_assign: false,
            locals,
            scope_depth: 0,
            upvalue_count: 0,
            upvalues: [UpValue::default(); 256],
            chunk: Chunk::default(),
        }
    }
}
macro_rules! context {
    ($self:expr) => {
        $self.context_stack.last_mut().unwrap()
    };
}

#[derive(Clone, Copy, Debug)]
pub struct UpValue {
    pub is_local: bool,
    pub index: usize,
    pub closed: bool,
}

impl Default for UpValue {
    fn default() -> Self {
        Self {
            is_local: false,
            index: 0,
            closed: false,
        }
    }
}

pub struct Compiler {
    name: String,
    parser: Parser,
    context_stack: Vec<CompilerContext>,
}

impl Compiler {
    const EMPTY_RULE: ParseRule = ParseRule::const_default();
    const PARSE_RULES: [ParseRule; 45] = {
        let mut data = [Self::EMPTY_RULE; 45];
        data[TokenType::LeftParen as usize] =
            ParseRule::new(Some(Self::grouping), Some(Self::call), Precedence::Call);
        data[TokenType::RightParen as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::LeftBrace as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::RightBrace as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::LeftSqb as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::RightSqb as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::Colon as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::Comma as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::Semi as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::Minus as usize] = ParseRule::new(
            Some(Compiler::unary),
            Some(Compiler::binary),
            Precedence::Term,
        );
        data[TokenType::Plus as usize] =
            ParseRule::new(None, Some(Compiler::binary), Precedence::Term);
        data[TokenType::Star as usize] =
            ParseRule::new(None, Some(Compiler::binary), Precedence::Factor);
        data[TokenType::Slash as usize] =
            ParseRule::new(None, Some(Compiler::binary), Precedence::Factor);
        data[TokenType::Dot as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::Rem as usize] = ParseRule::new(None, None, Precedence::None);

        data[TokenType::Bang as usize] =
            ParseRule::new(Some(Compiler::unary), None, Precedence::None);
        data[TokenType::BangEqual as usize] =
            ParseRule::new(None, Some(Compiler::binary), Precedence::Equality);
        data[TokenType::Equal as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::EqualEqual as usize] =
            ParseRule::new(None, Some(Compiler::binary), Precedence::Equality);
        data[TokenType::Greater as usize] =
            ParseRule::new(None, Some(Compiler::binary), Precedence::Comparison);
        data[TokenType::GreaterEqual as usize] =
            ParseRule::new(None, Some(Compiler::binary), Precedence::Comparison);
        data[TokenType::Less as usize] =
            ParseRule::new(None, Some(Compiler::binary), Precedence::Comparison);
        data[TokenType::LessEqual as usize] =
            ParseRule::new(None, Some(Compiler::binary), Precedence::Comparison);

        data[TokenType::Identifier as usize] =
            ParseRule::new(Some(Compiler::variable), None, Precedence::None);
        data[TokenType::Number as usize] =
            ParseRule::new(Some(Compiler::number), None, Precedence::None);
        data[TokenType::SoxString as usize] =
            ParseRule::new(Some(Compiler::string), None, Precedence::None);

        data[TokenType::And as usize] = ParseRule::new(None, Some(Compiler::and_), Precedence::And);
        data[TokenType::Class as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::Else as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::False as usize] =
            ParseRule::new(Some(Compiler::literal), None, Precedence::None);

        data[TokenType::For as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::If as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::Or as usize] = ParseRule::new(None, Some(Compiler::or_), Precedence::Or);
        data[TokenType::Return as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::Super as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::True as usize] =
            ParseRule::new(Some(Compiler::literal), None, Precedence::None);
        data[TokenType::While as usize] = ParseRule::new(None, None, Precedence::None);

        data[TokenType::Def as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::This as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::Let as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::Print as usize] = ParseRule::new(None, None, Precedence::None);

        data[TokenType::None as usize] =
            ParseRule::new(Some(Compiler::literal), None, Precedence::None);
        data[TokenType::Error as usize] = ParseRule::new(None, None, Precedence::None);
        data[TokenType::EOF as usize] = ParseRule::new(None, None, Precedence::None);

        data
    };

    pub fn new(name: String, parser: Parser) -> Self {
        let mut locals = Vec::new();
        let local = Local {
            name: "".to_string(),
            depth: Some(0),
            is_captured: false,
        };
        locals.push(local);
        let context = CompilerContext::default();
        Self {
            name,
            parser,
            context_stack: vec![context],
        }
    }

    pub fn compile(&mut self, i: &Interpreter) -> Result<SoxModule, CompileError> {
        self.parser.advance();
        while !self.parser.match_token(vec![TokenType::EOF]) && self.parser.current.is_some() {
            // Catch errors and synchronize to continue parsing
            if let Err(e) = self.declaration(i) {
                // Print the error if not already in panic mode
                if !self.parser.panic_mode {
                    eprintln!("{}", e);
                }
                self.parser.had_error = true;
                self.parser.synchronize();
                // Skip any orphaned RightBrace tokens left from exited blocks
                while self.parser.check(TokenType::RightBrace) {
                    self.parser.advance();
                }
            }
        }
        self.end();

        // If any error occurred, return an error
        if self.parser.had_error {
            return Err(CompileError::Syntax(SyntaxError {
                msg: "Compilation failed due to previous errors.".to_string(),
                line: 0,
            }));
        }

        let compiled_unit = SoxModule::new(
            self.name.clone(),
            SoxRef::new_ref(
                std::mem::take(&mut context!(self).chunk),
                i.types.co_type.to_owned(),
            ),
        );
        Ok(compiled_unit)
    }

    pub fn declaration(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        if self.parser.match_token(vec![TokenType::Def]) {
            self.function_declaration(i)?;
        } else if self.parser.match_token(vec![TokenType::Let]) {
            self.let_declaration(i)?;
        } else {
            self.statement(i)?;
        }
        Ok(())
    }

    pub fn function_declaration(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        let global = self.parse_variable(i, "Expect function name.")?;
        self.mark_initialized();
        if global.is_none() {
            self.emit_instruction_bytes((OpCode::OpNone, None));
        }
        let func_name = self.parser.previous.as_ref().unwrap().lexeme.to_string();
        let context = CompilerContext::default();
        self.context_stack.push(context);
        self.begin_scope(i);
        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after function name.")?;
        let mut arity: u16 = 0;
        if !self.parser.check(TokenType::RightParen) {
            loop {
                arity += 1;
                if arity > 255 {
                    let line = self.parser.current.as_ref().map_or(0, |t| t.line);
                    let token = self
                        .parser
                        .current
                        .as_ref()
                        .map_or(String::new(), |t| t.lexeme.to_string());
                    return Err(CompileError::TooManyParameters { line, token });
                }
                let cnst = self.parse_variable(i, "Expect parameter name.")?;
                self.define_variable(cnst, i);
                if !self.parser.match_token(vec![TokenType::Comma]) {
                    break;
                }
            }
        }
        self.parser.consume(
            TokenType::RightParen,
            "Expect ')' after function parameters.",
        )?;
        self.parser
            .consume(TokenType::LeftBrace, "Expect '{' before function body.")?;
        self.block(i)?;
        self.end();
        let mut context = self.context_stack.pop().unwrap();
        context.chunk.name = func_name.to_string();

        let fo = SoxFunction::new(
            func_name.to_string(),
            arity as usize,
            context.upvalue_count,
            SoxRef::new_ref(context.chunk, i.types.co_type.to_owned()),
        );
        let constant = self
            .make_constant(SoxObjectRef::from(SoxRef::new_ref(
                fo,
                i.types.function_type.to_owned(),
            )))
            .unwrap();
        if context.upvalue_count > 0 {
            self.emit_instruction_bytes((OpCode::OpClosure, Some(constant as u8)));
            for i in 0..context.upvalue_count {
                self.emit_bytes(context.upvalues[i].is_local as u8);
                self.emit_bytes(context.upvalues[i].index as u8);
            }
        } else {
            self.emit_instruction_bytes((OpCode::OpConstant, Some(constant as u8)));
        }
        if global.is_none() {
            let idx = (context!(self).locals.len() - 1) as u8;
            self.emit_instruction_bytes((OpCode::OpSetLocal, Some(idx)));
            self.emit_instruction_bytes((OpCode::OpPop, None));
        } else {
            self.define_variable(global, i);
        }
        Ok(())
    }

    pub fn let_declaration(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        let global = self.parse_variable(i, "Expect variable name.")?;
        if self.parser.match_token(vec![TokenType::Equal]) {
            self.expression(i)?;
        } else {
            self.emit_instruction_bytes((OpNone, None))
        }
        self.parser
            .consume(TokenType::Semi, "Expect ';' after variable declaration.")?;
        self.define_variable(global, i);
        Ok(())
    }

    pub fn define_variable(&mut self, global: Option<usize>, _i: &Interpreter) {
        if context!(self).scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.emit_instruction_bytes((OpCode::OpDefineGlobal, Some(global.unwrap() as u8)));
    }

    pub fn mark_initialized(&mut self) {
        if context!(self).scope_depth == 0 {
            return;
        }
        context!(self).locals.last_mut().unwrap().depth = Some(context!(self).scope_depth);
    }
    pub fn parse_variable(
        &mut self,
        i: &Interpreter,
        message: &str,
    ) -> Result<Option<usize>, CompileError> {
        self.parser.consume(TokenType::Identifier, message)?;

        if context!(self).scope_depth > 0 {
            self.declare_variable(i)?;
            return Ok(None);
        }
        info!(
            "Parsing variable: {}",
            self.parser.previous.as_ref().unwrap().lexeme
        );
        let global =
            self.identifier_constant(self.parser.previous.as_ref().unwrap().lexeme.to_string(), i);
        Ok(Some(global))
    }

    pub fn declare_variable(&mut self, _i: &Interpreter) -> Result<(), CompileError> {
        if context!(self).scope_depth == 0 {
            return Ok(());
        }
        let name = self.parser.previous.as_ref().unwrap().lexeme.to_string();
        info!("Declaring variable: {name}");
        let scope_depth = context!(self).scope_depth;
        let line = self.parser.previous.as_ref().map_or(0, |t| t.line);

        for local in context!(self).locals.iter().rev() {
            if local.depth.is_some() && (local.depth.unwrap() < scope_depth) {
                break;
            }
            if local.name == name {
                return Err(CompileError::VariableAlreadyDeclared { name, line });
            }
        }
        self.add_local(name);
        Ok(())
    }

    pub fn add_local(&mut self, name: String) {
        let local = Local {
            name,
            depth: None,
            is_captured: false,
        };
        context!(self).locals.push(local);
    }

    pub fn identifier_constant(&mut self, name: String, i: &Interpreter) -> usize {
        let constant_value = SoxObjectRef::from(i.new_string(name.parse().unwrap()));
        let constant = self.make_constant(constant_value).unwrap();
        constant
    }
    pub fn statement(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        if let Some(_token) = self.parser.current.as_ref() {
            if self.parser.match_token(vec![TokenType::Print]) {
                self.print_statement(i)?;
            } else if self.parser.match_token(vec![TokenType::For]) {
                self.for_statement(i)?;
            } else if self.parser.match_token(vec![TokenType::If]) {
                self.if_statement(i)?;
            } else if self.parser.match_token(vec![TokenType::Return]) {
                self.return_statement(i)?;
            } else if self.parser.match_token(vec![TokenType::While]) {
                self.while_statement(i)?;
            } else if self.parser.match_token(vec![TokenType::LeftBrace]) {
                self.begin_scope(i);
                self.block(i)?;
                self.end_scope(i);
            } else {
                self.expression_statement(i)?;
            }
        }
        Ok(())
    }

    pub fn return_statement(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        if self.parser.match_token(vec![TokenType::Semi]) {
            self.emit_instruction_bytes((OpNone, None)); // Push None for bare return
            self.emit_instruction_bytes((OpCode::OpReturn, None));
        } else {
            self.expression(i)?;
            self.parser
                .consume(TokenType::Semi, "Expect ';' after return value.")?;
            self.emit_instruction_bytes((OpCode::OpReturn, None));
        }
        Ok(())
    }
    pub fn call(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        let arg_count = self.argument_list(i)?;
        self.emit_instruction_bytes((OpCode::OpCall, Option::from(arg_count)));
        Ok(())
    }

    pub fn argument_list(&mut self, i: &Interpreter) -> Result<u8, CompileError> {
        let mut argcount: u16 = 0;
        if !self.parser.check(TokenType::RightParen) {
            loop {
                self.expression(i)?;
                argcount += 1;
                if argcount > 255 {
                    let line = self.parser.previous.as_ref().map_or(0, |t| t.line);
                    let token = self
                        .parser
                        .previous
                        .as_ref()
                        .map_or(String::new(), |t| t.lexeme.to_string());
                    return Err(CompileError::TooManyArguments { line, token });
                }
                if !self.parser.match_token(vec![TokenType::Comma]) {
                    break;
                }
            }
        }
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after arguments.")?;
        Ok(argcount as u8)
    }

    pub fn and_(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        let end_jump = self.emit_jump(OpJumpIfFalse);
        self.emit_instruction_bytes((OpPop, None));

        self.parse_with_precedence(Precedence::And, i)?;
        self.patch_jump(end_jump)?;
        Ok(())
    }

    pub fn or_(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        let else_jump = self.emit_jump(OpJumpIfFalse);
        let end_jump = self.emit_jump(OpJump);

        self.patch_jump(else_jump)?;
        self.emit_instruction_bytes((OpPop, None));

        self.parse_with_precedence(Precedence::Or, i)?;
        self.patch_jump(end_jump)?;
        Ok(())
    }

    pub fn for_statement(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        self.begin_scope(i);

        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after 'for'")?;
        if self.parser.match_token(vec![TokenType::Semi]) {
        } else if self.parser.match_token(vec![TokenType::Let]) {
            self.let_declaration(i)?;
        } else {
            self.expression_statement(i)?;
        }
        let mut loop_start = context!(self).chunk.code.len();

        let mut exit_jump = None;
        if !self.parser.match_token(vec![TokenType::Semi]) {
            self.expression(i)?;
            self.parser
                .consume(TokenType::Semi, "Expect ';' after variable declaration.")?;

            exit_jump = Some(self.emit_jump(OpJumpIfFalse));
            self.emit_instruction_bytes((OpPop, None));
        }
        if !self.parser.match_token(vec![TokenType::RightParen]) {
            let body_jump = self.emit_jump(OpJump);
            let increment_start = context!(self).chunk.code.len();
            self.expression(i)?;
            self.emit_instruction_bytes((OpPop, None));
            self.parser
                .consume(TokenType::RightParen, "Expect ')' after for clauses")?;

            self.emit_loop(loop_start)?;
            loop_start = increment_start;
            self.patch_jump(body_jump)?;
        }

        self.statement(i)?;
        self.emit_loop(loop_start)?;
        if let Some(jump) = exit_jump {
            self.patch_jump(jump)?;
            self.emit_instruction_bytes((OpPop, None));
        }
        self.end_scope(i);
        Ok(())
    }

    pub fn while_statement(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        let loop_start = context!(self).chunk.code.len();
        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after 'while'.")?;
        self.expression(i)?;
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after 'while'.")?;

        let exit_jump = self.emit_jump(OpJumpIfFalse);
        self.emit_instruction_bytes((OpPop, None));
        self.statement(i)?;

        self.emit_loop(loop_start)?;

        self.patch_jump(exit_jump)?;
        self.emit_instruction_bytes((OpPop, None));
        Ok(())
    }

    pub fn if_statement(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after 'if'.")?;
        self.expression(i)?;
        self.parser
            .consume(TokenType::RightParen, "Expect '(' after 'if'.")?;
        let then_jump = self.emit_jump(OpJumpIfFalse);

        self.emit_instruction_bytes((OpPop, None));
        self.statement(i)?;
        let jump = self.emit_jump(OpJump);
        self.patch_jump(then_jump)?;

        self.emit_instruction_bytes((OpPop, None));
        if self.parser.match_token(vec![TokenType::Else]) {
            self.statement(i)?;
        }

        self.patch_jump(jump)?;
        Ok(())
    }

    fn emit_loop(&mut self, loop_start: usize) -> Result<(), CompileError> {
        self.emit_instruction_bytes((OpCode::OpLoop, None));

        let offset = context!(self).chunk.code.len() - loop_start + 2;
        if offset as i16 > i16::MAX {
            let line = self.parser.previous.as_ref().map_or(0, |t| t.line);
            return Err(CompileError::LoopBodyTooLarge { line });
        }

        self.emit_bytes(((offset >> 8) & 0xff) as u8);
        self.emit_bytes((offset & 0xff) as u8);
        Ok(())
    }
    fn emit_jump(&mut self, op_code: OpCode) -> usize {
        self.emit_instruction_bytes((op_code, None));
        self.emit_instruction_bytes((OpNone, None));
        self.emit_instruction_bytes((OpNone, None));
        return context!(self).chunk.code.len() - 2;
    }

    fn patch_jump(&mut self, offset: usize) -> Result<(), CompileError> {
        let code_len = context!(self).chunk.code.len();
        let jump = code_len - offset - 2;

        if jump as i16 > i16::MAX {
            let line = self.parser.previous.as_ref().map_or(0, |t| t.line);
            return Err(CompileError::JumpTooLarge { line });
        }
        context!(self).chunk.code[offset] = ((jump >> 8) & 0xff) as u8;
        context!(self).chunk.code[offset + 1] = (jump & 0xff) as u8;
        Ok(())
    }

    pub fn begin_scope(&mut self, _i: &Interpreter) {
        context!(self).scope_depth += 1;
    }

    pub fn end_scope(&mut self, _interpreter: &Interpreter) {
        context!(self).scope_depth -= 1;
        let mut local_count = context!(self).locals.len();
        while local_count > 0
            && context!(self).locals[local_count - 1].depth.unwrap() > context!(self).scope_depth
        {
            if context!(self).locals[local_count - 1].is_captured {
                self.emit_instruction_bytes((OpCode::OpCloseUpvalue, None));
            } else {
                self.emit_instruction_bytes((OpCode::OpPop, None));
            }
            context!(self).locals.pop();
            local_count -= 1;
        }
    }

    pub fn block(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        while !self.parser.check(TokenType::RightBrace) && !self.parser.check(TokenType::EOF) {
            self.declaration(i)?;
        }
        self.parser
            .consume(TokenType::RightBrace, "Expect '}' after block.")?;
        Ok(())
    }

    pub fn print_statement(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        self.expression(i)?;
        self.parser
            .consume(TokenType::Semi, "Expect ';' after value.")?;
        self.emit_instruction_bytes((OpCode::OpPrint, None));
        Ok(())
    }

    pub fn expression_statement(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        self.expression(i)?;
        self.parser
            .consume(TokenType::Semi, "Expect ';' after expression.")?;
        self.emit_instruction_bytes((OpCode::OpPop, None));
        Ok(())
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
        let idx = context!(self).chunk.add_constant(value);
        if idx > i8::MAX as usize {
            Err(())
        } else {
            Ok(idx)
        }
    }

    pub fn emit_return(&mut self) {
        self.emit_instruction_bytes((OpNone, None));
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

    fn emit_bytes(&mut self, data: u8) {
        context!(self)
            .chunk
            .write_chunk(data, self.parser.previous.as_ref().unwrap().line);
    }

    pub fn expression(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        self.parse_with_precedence(Precedence::Assignment, i)
    }

    pub fn grouping(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        self.expression(i)?;
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after expression.")?;
        Ok(())
    }

    pub fn unary(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        let operator_type = self.parser.previous.as_ref().unwrap().token_type;
        self.parse_with_precedence(Precedence::Unary, i)?;

        match operator_type {
            TokenType::Bang => {
                self.emit_instruction_bytes((OpCode::OpNot, None));
            }
            TokenType::Minus => {
                self.emit_instruction_bytes((OpCode::OpNegate, None));
            }
            _ => {}
        }
        Ok(())
    }

    pub fn binary(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        if let Some(token) = self.parser.previous.as_ref() {
            let operator_type = token.token_type;
            let rule = self.get_rule(operator_type);
            let new_precedence = Precedence::try_from(rule.precedence as u8 + 1).unwrap();
            self.parse_with_precedence(new_precedence, i)?;
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
                TokenType::Greater => {
                    self.emit_instruction_bytes((OpCode::OpGreater, None));
                }
                TokenType::EqualEqual => {
                    self.emit_instruction_bytes((OpCode::OpEqual, None));
                }
                TokenType::BangEqual => {
                    self.emit_instruction_bytes((OpCode::OpEqual, None));
                    self.emit_instruction_bytes((OpCode::OpNot, None));
                }
                TokenType::GreaterEqual => {
                    self.emit_instruction_bytes((OpCode::OpLess, None));
                    self.emit_instruction_bytes((OpCode::OpNot, None));
                }
                TokenType::LessEqual => {
                    self.emit_instruction_bytes((OpCode::OpGreater, None));
                    self.emit_instruction_bytes((OpCode::OpNot, None));
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn literal(&mut self, _i: &Interpreter) -> Result<(), CompileError> {
        let value = self.parser.previous.as_ref().unwrap();
        match value.token_type {
            TokenType::False => {
                self.emit_instruction_bytes((OpCode::OpFalse, None));
            }
            TokenType::True => self.emit_instruction_bytes((OpCode::OpTrue, None)),
            TokenType::None => self.emit_instruction_bytes((OpCode::OpNone, None)),

            _ => {}
        }
        Ok(())
    }
    pub fn get_rule(&self, token_type: TokenType) -> &ParseRule {
        Self::PARSE_RULES[token_type as usize].borrow()
    }

    pub fn parse_with_precedence(
        &mut self,
        precedence: Precedence,
        i: &Interpreter,
    ) -> Result<(), CompileError> {
        self.parser.advance();
        let previous_token_type = self.parser.previous.as_ref().map(|token| token.token_type);
        if let Some(previous_token_type) = previous_token_type {
            let prefix_rule = self.get_rule(previous_token_type).prefix_fn;
            if let Some(prefix_rule_fn) = prefix_rule {
                context!(self).can_assign = precedence <= Precedence::Assignment;
                prefix_rule_fn(self, i)?;
            } else {
                let token = self.parser.previous.as_ref();
                let line = token.map_or(0, |t| t.line);
                let lexeme = token.map_or("<eof>", |t| t.lexeme);
                return Err(CompileError::Syntax(SyntaxError {
                    msg: format!("Error at '{}': Expect an expression.", lexeme),
                    line,
                }));
            }
        } else {
            return Err(CompileError::Syntax(SyntaxError {
                msg: "Unexpected end of input.".to_string(),
                line: 0,
            }));
        }
        while self.parser.current.is_some()
            && precedence
                <= self
                    .get_rule(self.parser.current.as_ref().unwrap().token_type)
                    .precedence
        {
            self.parser.advance();
            let infix_rule = self
                .get_rule(self.parser.previous.as_ref().unwrap().token_type)
                .infix_fn;
            if let Some(infix_rule_fn) = infix_rule {
                infix_rule_fn(self, i)?;
            }
        }
        // If we see '=' but can't assign, it's an invalid assignment target
        if self.parser.check(TokenType::Equal) {
            self.parser.advance(); // consume the '='
            let line = self.parser.previous.as_ref().map_or(0, |t| t.line);
            return Err(CompileError::InvalidAssignmentTarget { line });
        }
        Ok(())
    }

    pub fn number(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        let val = i64::from_str(self.parser.previous.as_ref().unwrap().lexeme).unwrap();
        let obj_payload = SoxInt { value: val };
        let obj = SoxRef::new_ref(obj_payload, i.types.int_type.to_owned());
        self.emit_constant(SoxObjectRef::from(obj));
        Ok(())
    }

    pub fn string(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        let obj_payload = SoxString {
            value: self.parser.previous.as_ref().unwrap().lexeme.to_string(),
        };
        let obj = SoxRef::new_ref(obj_payload, i.types.str_type.to_owned());
        self.emit_constant(SoxObjectRef::from(obj));
        Ok(())
    }

    pub fn variable(&mut self, i: &Interpreter) -> Result<(), CompileError> {
        self.named_variable(self.parser.previous.as_ref().unwrap().lexeme, i);
        Ok(())
    }

    pub fn named_variable(&mut self, name: &str, i: &Interpreter) {
        let get_op;
        let set_op;
        let mut arg = self.resolve_local(name, i, self.context_stack.len() - 1);
        if let Some(_arg) = arg {
            get_op = OpGetLocal;
            set_op = OpSetLocal;
        } else {
            arg = self.resolve_upvalue(name, self.context_stack.len() - 1, i);
            if arg.is_some() {
                get_op = OpGetUpvalue;
                set_op = OpSetUpvalue;
            } else {
                arg = Some(self.identifier_constant(name.to_string(), i) as u8);
                get_op = OpGetGlobal;
                set_op = OpSetGlobal;
            }
        }
        if context!(self).can_assign && self.parser.match_token(vec![TokenType::Equal]) {
            let _ = self.expression(i);
            self.emit_instruction_bytes((set_op, arg))
        } else {
            self.emit_instruction_bytes((get_op, arg))
        }
    }

    pub fn resolve_upvalue(
        &mut self,
        name: &str,
        compiler_idx: usize,
        i: &Interpreter,
    ) -> Option<u8> {
        if compiler_idx == 0 {
            return None;
        }

        let parent_idx = compiler_idx - 1;

        // Look for a local variable in the enclosing compiler.
        if let Some(local_index) = self.resolve_local(name, i, parent_idx) {
            self.context_stack[parent_idx].locals[local_index as usize].is_captured = true;
            return self.add_upvalue(compiler_idx, local_index, true);
        }

        // Recursively look for an upvalue in the enclosing compilers.
        if let Some(upvalue_index) = self.resolve_upvalue(name, parent_idx, i) {
            return self.add_upvalue(compiler_idx, upvalue_index, false);
        }

        None
    }

    pub fn add_upvalue(&mut self, compiler_idx: usize, idx: u8, is_local: bool) -> Option<u8> {
        let context = self.context_stack.get_mut(compiler_idx)?;
        let upvalue_count = context.upvalue_count;
        for i in 0..upvalue_count {
            let upvalue = &mut context.upvalues[i];
            if upvalue.index as u8 == idx && upvalue.is_local == is_local {
                return Some(i as u8);
            }
        }
        if upvalue_count == u8::MAX as usize {
            panic!("Too many upvalues in function.");
        }
        context.upvalues[upvalue_count].is_local = is_local;
        context.upvalues[upvalue_count].index = idx as usize;
        context.upvalue_count += 1;

        Some((context.upvalue_count - 1) as u8)
    }

    pub fn resolve_local(&mut self, name: &str, _i: &Interpreter, ctx_index: usize) -> Option<u8> {
        let context = &self.context_stack[ctx_index];
        for (i, local) in context.locals.iter().enumerate().rev() {
            if local.name.as_str() == name {
                if local.depth.is_none() {
                    panic!("Can't read a local variable in its own initializer.");
                }
                return Some(i as u8);
            }
        }
        None
    }
}

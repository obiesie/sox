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
}

impl Parser {
    pub fn new(source: &'static str) -> Self {
        let lexer = Lexer::new(source);
        let parser = Parser {
            previous: None,
            current: None,
            tokens: lexer.peekable(),
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
            let (previous_lexeme, line) = self
                .previous
                .as_ref()
                .map_or(("", 0), |t| (t.lexeme, t.line));

            Err(SyntaxError {
                msg: format!("Error after '{}': {}", previous_lexeme, message),
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

    pub fn compile(&mut self, i: &Interpreter) -> SoxModule {
        self.parser.advance();
        while !self.parser.match_token(vec![TokenType::EOF]) && self.parser.current.is_some() {
            self.declaration(i);
        }
        self.end();

        let compiled_unit = SoxModule::new(
            self.name.clone(),
            SoxRef::new_ref(
                std::mem::take(&mut context!(self).chunk),
                i.types.co_type.to_owned(),
            ),
        );
        compiled_unit
    }

    pub fn declaration(&mut self, i: &Interpreter) {
        if self.parser.match_token(vec![TokenType::Def]) {
            self.function_declaration(i)
        } else if self.parser.match_token(vec![TokenType::Let]) {
            self.let_declaration(i);
        } else {
            self.statement(i);
        }
    }

    pub fn function_declaration(&mut self, i: &Interpreter) {
        let global = self.parse_variable(i, "Expect function name.");
        self.mark_initialized();
        let func_name = self.parser.previous.as_ref().unwrap().lexeme.to_string();
        let context = CompilerContext::default();
        self.context_stack.push(context);
        self.begin_scope(i);
        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after function name.")
            .expect("TODO: panic message");
        let mut arity = 0;
        if !self.parser.check(TokenType::RightParen) {
            loop {
                arity += 1;
                if arity > 255 {
                    panic!("Too many parameters.");
                }
                let cnst = self.parse_variable(i, "Expect parameter name.");
                self.define_variable(cnst, i);
                if !self.parser.match_token(vec![TokenType::Comma]) {
                    break;
                }
            }
        }
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after parameters.")
            .expect("TODO: panic message");
        self.parser
            .consume(TokenType::LeftBrace, "Expect '{' before function body.")
            .expect("TODO: panic message");
        self.block(i);
        self.end();
        let mut context = self.context_stack.pop().unwrap();
        context.chunk.name = func_name.to_string();

        let fo = SoxFunction::new(
            func_name.to_string(),
            arity,
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
        self.define_variable(global, i);
    }

    pub fn let_declaration(&mut self, i: &Interpreter) {
        let global = self.parse_variable(i, "Expect variable name.");
        if self.parser.match_token(vec![TokenType::Equal]) {
            self.expression(i);
        } else {
            self.emit_instruction_bytes((OpNone, None))
        }
        self.parser
            .consume(TokenType::Semi, "Expect ';' after variable declaration.")
            .expect("");
        self.define_variable(global, i);
    }

    pub fn define_variable(&mut self, global: Option<usize>, i: &Interpreter) {
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
    pub fn parse_variable(&mut self, i: &Interpreter, message: &str) -> Option<usize> {
        self.parser
            .consume(TokenType::Identifier, message)
            .expect("Failed to parse variable. ");

        if context!(self).scope_depth > 0 {
            self.declare_variable(i);
            return None;
        }
        info!(
            "Parsing variable: {}",
            self.parser.previous.as_ref().unwrap().lexeme
        );
        let global =
            self.identifier_constant(self.parser.previous.as_ref().unwrap().lexeme.to_string(), i);
        Some(global)
    }

    pub fn declare_variable(&mut self, i: &Interpreter) {
        if context!(self).scope_depth == 0 {
            return;
        }
        let name = self.parser.previous.as_ref().unwrap().lexeme.to_string();
        info!("Declaring variable: {name}");
        let scope_depth = context!(self).scope_depth;

        for local in context!(self).locals.iter().rev() {
            if local.depth.is_some() && (local.depth.unwrap() < scope_depth) {
                break;
            }
            if local.name == name {
                panic!("Variable with this name already declared in this scope.");
            }
        }
        self.add_local(name)
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
    pub fn statement(&mut self, i: &Interpreter) {
        if let Some(token) = self.parser.current.as_ref() {
            if self.parser.match_token(vec![TokenType::Print]) {
                self.print_statement(i);
            } else if self.parser.match_token(vec![TokenType::For]) {
                self.for_statement(i)
            } else if self.parser.match_token(vec![TokenType::If]) {
                self.if_statement(i);
            } else if self.parser.match_token(vec![TokenType::Return]) {
                self.return_statement(i);
            } else if self.parser.match_token(vec![TokenType::While]) {
                self.while_statement(i);
            } else if self.parser.match_token(vec![TokenType::LeftBrace]) {
                self.begin_scope(i);
                self.block(i);
                self.end_scope(i);
            } else {
                self.expression_statement(i);
            }
        }
    }

    pub fn return_statement(&mut self, i: &Interpreter) {
        if self.parser.match_token(vec![TokenType::Semi]) {
            self.emit_instruction_bytes((OpNone, None)); // Push None for bare return
            self.emit_instruction_bytes((OpCode::OpReturn, None));
        } else {
            self.expression(i);
            self.parser
                .consume(TokenType::Semi, "Expect ';' after return value.")
                .expect("");
            self.emit_instruction_bytes((OpCode::OpReturn, None));
        }
    }
    pub fn call(&mut self, i: &Interpreter) {
        let arg_count = self.argument_list(i);
        self.emit_instruction_bytes((OpCode::OpCall, Option::from(arg_count)))
    }

    pub fn argument_list(&mut self, i: &Interpreter) -> u8 {
        let mut argcount = 0;
        if !self.parser.check(TokenType::RightParen) {
            loop {
                self.expression(i);
                argcount += 1;
                if argcount > 255 {
                    panic!("Too many arguments.");
                }
                if !self.parser.match_token(vec![TokenType::Comma]) {
                    break;
                }
            }
        }
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after arguments.")
            .expect("TODO: panic message");
        return argcount;
    }

    pub fn and_(&mut self, i: &Interpreter) {
        let end_jump = self.emit_jump(OpJumpIfFalse);
        self.emit_instruction_bytes((OpPop, None));

        self.parse_with_precedence(Precedence::And, i)
            .expect("TODO: panic message");
        self.patch_jump(end_jump);
    }

    pub fn or_(&mut self, i: &Interpreter) {
        let else_jump = self.emit_jump(OpJumpIfFalse);
        let end_jump = self.emit_jump(OpJump);

        self.patch_jump(else_jump);
        self.emit_instruction_bytes((OpPop, None));

        self.parse_with_precedence(Precedence::Or, i)
            .expect("TODO: panic message");
        self.patch_jump(end_jump);
    }

    pub fn for_statement(&mut self, i: &Interpreter) {
        self.begin_scope(i);

        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after 'for'")
            .expect("TODO: panic message");
        if self.parser.match_token(vec![TokenType::Semi]) {
        } else if self.parser.match_token(vec![TokenType::Let]) {
            self.let_declaration(i);
        } else {
            self.expression_statement(i);
        }
        let mut loop_start = context!(self).chunk.code.len();

        let mut exit_jump = None;
        if !self.parser.match_token(vec![TokenType::Semi]) {
            self.expression(i);
            self.parser
                .consume(TokenType::Semi, "Expect ';' after variable declaration.")
                .expect("");

            exit_jump = Some(self.emit_jump(OpJumpIfFalse));
            self.emit_instruction_bytes((OpPop, None));
        }
        if !self.parser.match_token(vec![TokenType::RightParen]) {
            let body_jump = self.emit_jump(OpJump);
            let increment_start = context!(self).chunk.code.len();
            self.expression(i);
            self.emit_instruction_bytes((OpPop, None));
            self.parser
                .consume(TokenType::RightParen, "Expect ')' after for clauses")
                .expect("");

            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        self.statement(i);
        self.emit_loop(loop_start);
        if let Some(jump) = exit_jump {
            self.patch_jump(jump);
            self.emit_instruction_bytes((OpPop, None));
        }
        self.end_scope(i)
    }

    pub fn while_statement(&mut self, i: &Interpreter) {
        let loop_start = context!(self).chunk.code.len();
        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after 'while'.")
            .expect("TODO: panic message");
        self.expression(i);
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after 'while'.")
            .expect("TODO: panic message");

        let exit_jump = self.emit_jump(OpJumpIfFalse);
        self.emit_instruction_bytes((OpPop, None));
        self.statement(i);

        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        self.emit_instruction_bytes((OpPop, None));
    }

    pub fn if_statement(&mut self, i: &Interpreter) {
        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after 'if'.")
            .expect("TODO: panic message");
        self.expression(i);
        self.parser
            .consume(TokenType::RightParen, "Expect '(' after 'if'.")
            .expect("TODO: panic message");
        let then_jump = self.emit_jump(OpJumpIfFalse);

        self.emit_instruction_bytes((OpPop, None));
        self.statement(i);
        let jump = self.emit_jump(OpJump);
        self.patch_jump(then_jump);

        self.emit_instruction_bytes((OpPop, None));
        if self.parser.match_token(vec![TokenType::Else]) {
            self.statement(i);
        }

        self.patch_jump(jump);
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_instruction_bytes((OpCode::OpLoop, None));

        let offset = context!(self).chunk.code.len() - loop_start + 2;
        if offset as i16 > i16::MAX {
            panic!("loop body too large.");
        }

        self.emit_bytes(((offset >> 8) & 0xff) as u8);
        self.emit_bytes((offset & 0xff) as u8);
    }
    fn emit_jump(&mut self, op_code: OpCode) -> usize {
        self.emit_instruction_bytes((op_code, None));
        self.emit_instruction_bytes((OpNone, None));
        self.emit_instruction_bytes((OpNone, None));
        return context!(self).chunk.code.len() - 2;
    }

    fn patch_jump(&mut self, offset: usize) {
        let code_len = context!(self).chunk.code.len();
        let jump = code_len - offset - 2;

        if jump as i16 > i16::MAX {
            panic!("Too much code to jump over!")
        }
        context!(self).chunk.code[offset] = ((jump >> 8) & 0xff) as u8;
        context!(self).chunk.code[offset + 1] = (jump & 0xff) as u8;
    }

    pub fn begin_scope(&mut self, i: &Interpreter) {
        context!(self).scope_depth += 1;
    }

    pub fn end_scope(&mut self, interpreter: &Interpreter) {
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

    pub fn block(&mut self, i: &Interpreter) {
        while !self.parser.check(TokenType::RightBrace) && !self.parser.check(TokenType::EOF) {
            self.declaration(i);
        }
        self.parser
            .consume(TokenType::RightBrace, "Expect '}' after block.")
            .expect("TODO: panic message");
    }

    pub fn print_statement(&mut self, i: &Interpreter) {
        self.expression(i);
        self.parser
            .consume(TokenType::Semi, "Expect ';' after value.")
            .expect("TODO: panic message");
        self.emit_instruction_bytes((OpCode::OpPrint, None));
    }

    pub fn expression_statement(&mut self, i: &Interpreter) {
        self.expression(i);
        self.parser
            .consume(TokenType::Semi, "Expect ';' after expression.")
            .expect("TODO: panic message");
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

    pub fn expression(&mut self, i: &Interpreter) {
        self.parse_with_precedence(Precedence::Assignment, i)
            .expect("TODO: panic message");
    }

    pub fn grouping(&mut self, i: &Interpreter) {
        self.expression(i);
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after expression.")
            .expect("TODO: panic message");
    }

    pub fn unary(&mut self, i: &Interpreter) {
        let operator_type = self.parser.previous.as_ref().unwrap().token_type;
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
        if let Some(token) = self.parser.previous.as_ref() {
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
        let value = self.parser.previous.as_ref().unwrap();
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
        Self::PARSE_RULES[token_type as usize].borrow()
    }

    pub fn parse_with_precedence(
        &mut self,
        precedence: Precedence,
        i: &Interpreter,
    ) -> Result<(), ()> {
        self.parser.advance();
        let previous_token_type = self.parser.previous.as_ref().map(|token| token.token_type);
        if let Some(previous_token_type) = previous_token_type {
            let prefix_rule = self.get_rule(previous_token_type).prefix_fn;
            if let Some(prefix_rule_fn) = prefix_rule {
                context!(self).can_assign = precedence <= Precedence::Assignment;
                prefix_rule_fn(self, i);
            } else {
                return Err(());
            }
        } else {
            return Err(());
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
                infix_rule_fn(self, i)
            }
        }
        if context!(self).can_assign && self.parser.match_token(vec![TokenType::Equal]) {
            panic!("Invalid assignment target.");
        }
        Ok(())
    }

    pub fn number(&mut self, i: &Interpreter) {
        let val = i64::from_str(self.parser.previous.as_ref().unwrap().lexeme).unwrap();
        let obj_payload = SoxInt { value: val };
        let obj = SoxRef::new_ref(obj_payload, i.types.int_type.to_owned());
        self.emit_constant(SoxObjectRef::from(obj));
    }

    pub fn string(&mut self, i: &Interpreter) {
        let obj_payload = SoxString {
            value: self.parser.previous.as_ref().unwrap().lexeme.to_string(),
        };
        let obj = SoxRef::new_ref(obj_payload, i.types.str_type.to_owned());
        self.emit_constant(SoxObjectRef::from(obj));
    }

    pub fn variable(&mut self, i: &Interpreter) {
        self.named_variable(self.parser.previous.as_ref().unwrap().lexeme, i);
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
        if self.parser.match_token(vec![TokenType::Equal]) && context!(self).can_assign {
            self.expression(i);
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

    // TODO modify to take a pointer to compiler context to start search for local variable
    pub fn resolve_local(&mut self, name: &str, i: &Interpreter, ctx_index: usize) -> Option<u8> {
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

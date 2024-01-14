use std::iter::Peekable;

use log::info;

use crate::expr::Expr;
use crate::stmt::Stmt;
use crate::token::{Literal, Token};
use crate::token_type::TokenType;
use crate::token_type::TokenType::{
    And, Bang, BangEqual, Class, Colon, Comma, Def, Dot, Else, Equal, EqualEqual, False, For,
    Greater, GreaterEqual, Identifier, If, LeftBrace, LeftParen, Less, LessEqual, Let, Minus, Mod,
    Number, Or, Plus, Print, Return, RightBrace, RightParen, Semi, Slash, SoxString, Star, Super,
    This, True, While,
};

pub static TO_IGNORE: &'static [TokenType] = &[TokenType::Comment, TokenType::Whitespace, TokenType::Newline];


pub struct Parser<I: Iterator<Item=Token>> {
    tokens: Peekable<I>,
    processed_tokens: Vec<Token>,
}

#[derive(Clone, Debug)]
pub struct SyntaxError {
    msg: String,
    line: usize
}

impl<I: Iterator<Item=Token>> Parser<I> {
    pub fn new(tokens: I) -> Self {
        return Parser { tokens: tokens.peekable(), processed_tokens: vec![] };
    }

    fn previous(&self) -> Token {
        let prev = self.processed_tokens.last().unwrap().clone();
        return prev;
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, Vec<SyntaxError>> {
        let mut statements = vec![];
        let mut errors = vec![];
        while !self.at_end() {
            let stmt = self.declaration();
            if let Ok(val) = stmt {
                statements.push(val);
            } else {
                if let Err(e) = stmt{
                    let err_msg = e.msg.to_string();
                    errors.push(e);
                    info!("Error while building parse tree - {:?}", err_msg);
                }

            }
        }
        if errors.is_empty(){
            return Ok(statements)
        }
        return Err(errors);
    }


    fn synchronize(&mut self) {
        self.advance();
        while !self.at_end() {
            if self.previous().token_type == Semi {
                return;
            }
            let peek_val = self.tokens.peek();
            if peek_val.is_some() && vec![Class, Def, Let, For, If, While, Print, Return].contains(&peek_val.unwrap().token_type) {
                return;
            }
            self.advance();
        }
    }

    fn declaration(&mut self) -> Result<Stmt, SyntaxError> {
        let val = if self.match_token(vec![Class]) {
            self.class_declaration()
        } else if self.match_token(vec![Def]) {
            self.function("function".into())
        } else if self.match_token(vec![Let]) {
            self.var_declaration()
        } else {
            self.statement()
        };
        if val.is_err() {
            self.synchronize();
        }
        return val;
    }


    fn class_declaration(&mut self) -> Result<Stmt, SyntaxError> {
        let name = self.consume(Identifier, "Expect a class name".into())?;

        let mut super_class = None;
        if self.match_token(vec![Colon]) {
            let _ = self.consume(Identifier, "Expect a superclass name".into())?;
            let prev = self.previous();
            super_class = Some(Expr::Variable {
                name: prev,
            });
        }
        let _ = self.consume(LeftBrace, "Expect '{' before class body".into())?;
        let mut methods = vec![];
        while !self.check(RightBrace) && !self.at_end() {
            methods.push(self.function("method".into())?);
        }
        let _ = self.consume(RightBrace, "Expect '}' after class body.".into())?;
        let class = Stmt::Class {
            name,
            methods,
            superclass: super_class,
        };
        return Ok(class);
    }

    fn function(&mut self, _kind: String) -> Result<Stmt, SyntaxError> {
        let name = self.consume(Identifier, "Expect function name.".into())?;
        let _ = self.consume(LeftParen, "Expect '(' after function name.".into())?;
        let mut params = vec![];
        if !self.check(RightParen) {
            loop {
                if params.len() >= 255 {
                    return Err(SyntaxError {
                        msg: "Cannot have more than 255 parameters.".into(),
                        line: name.line
                    });
                }
                let param = self.consume(Identifier, "Expect parameter name.".into())?;
                params.push(param);
                if !self.match_token(vec![Comma]) {
                    break;
                }
            }
        }
        let _ = self.consume(RightParen, "Expect ')' after function parameters.".into())?;
        let _ = self.consume(LeftBrace, "Expect '{' before function body.".into())?;

        let body = self.block()?;
        let stmt = Stmt::Function { name, params, body };
        return Ok(stmt);
    }

    fn var_declaration(&mut self) -> Result<Stmt, SyntaxError> {
        let name = self.consume(Identifier, "Expect variable name.".into())?;
        let mut initializer = None;
        if self.match_token(vec![Equal]) {
            initializer = Some(self.expression()?);
        }
        let _ = self.consume(Semi, "Expect ';' after variable declaration".into())?;
        return Ok(Stmt::Var { name, initializer });
    }

    fn statement(&mut self) -> Result<Stmt, SyntaxError> {
        if self.match_token(vec![For]) {
            return self.for_statement();
        }
        if self.match_token(vec![If]) {
            return self.if_statement();
        }
        if self.match_token(vec![While]) {
            return self.while_statement();
        }
        if self.match_token(vec![Print]) {
            return self.print_statement();
        }
        if self.match_token(vec![Return]) {
            return self.return_statement();
        }
        if self.match_token(vec![LeftBrace]) {
            let block_statements = self.block()?;
            return Ok(Stmt::Block(block_statements));
        }
        return self.expression_statement();
    }

    fn return_statement(&mut self) -> Result<Stmt, SyntaxError> {
        let keyword = self.previous();
        let mut value = Expr::Literal {
            value: Literal::None,
        };
        if !self.check(Semi) {
            value = self.expression()?
        }
        let _ = self.consume(Semi, "Expect ';' after return value.".into())?;
        let return_stmt = Stmt::Return { keyword, value };
        return Ok(return_stmt);
    }

    fn for_statement(&mut self) -> Result<Stmt, SyntaxError> {
        let _ = self.consume(LeftParen, "Expect '(' after 'for'.".to_string())?;
        let mut initializer = None;
        if self.match_token(vec![Semi]) {
            initializer = None;
        } else if self.match_token(vec![Let]) {
            initializer = Some(self.var_declaration()?);
        } else {
            initializer = Some(self.expression_statement()?);
        }
        let mut condition = None;
        if !self.check(Semi) {
            condition = Some(self.expression()?);
        }
        let _ = self.consume(Semi, "Expect ';' after loop condition.".to_string())?;
        let mut increment = None;
        if !self.check(RightParen) {
            increment = Some(self.expression()?);
        }
        let _ = self.consume(RightParen, "Expect ')' after for clauses.".to_string())?;
        let mut body = self.statement()?;
        if let Some(inc) = increment {
            let stmts = vec![body, Stmt::Expression(inc)];
            body = Stmt::Block(stmts)
        }
        if condition.is_none() {
            condition = Some(Expr::Literal {
                value: Literal::Boolean(true),
            });
        }
        body = Stmt::While {
            condition: condition.unwrap(),
            body: Box::new(body),
        };
        if let Some(init) = initializer {
            body = Stmt::Block(vec![init, body])
        }
        return Ok(body);
    }

    fn while_statement(&mut self) -> Result<Stmt, SyntaxError> {
        let _ = self.consume(LeftParen, "Expect '(' after 'while'.".into())?;
        let condition = self.expression()?;
        let _ = self.consume(RightParen, "Expect ')' after 'while' condition.".into())?;
        let body = self.statement()?;
        return Ok(Stmt::While {
            condition,
            body: Box::new(body),
        });
    }

    fn if_statement(&mut self) -> Result<Stmt, SyntaxError> {
        let _ = self.consume(LeftParen, "Expect '(' after 'if'.".into())?;
        let condition = self.expression()?;
        let _ = self.consume(RightParen, "Expect ')' after 'if' condition.".into())?;

        let then_branch = self.statement()?;
        let mut else_branch = None;
        if self.match_token(vec![Else]) {
            else_branch = Some(self.statement()?);
        }
        return Ok(Stmt::If {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        });
    }

    fn expression_statement(&mut self) -> Result<Stmt, SyntaxError> {
        let expr = self.expression();
        if let Ok(e) = expr {
            let _ = self.consume(Semi, "Expect ';' after expression.".into())?;
            return Ok(Stmt::Expression(e));
        } else {
            return Err(expr.err().unwrap());
        }
    }

    fn block(&mut self) -> Result<Vec<Stmt>, SyntaxError> {
        let mut statements = vec![];
        while !self.check(RightBrace) && !self.at_end() {
            let stmt = self.declaration()?;
            statements.push(stmt);
        }
        let _ = self.consume(RightBrace, "".into())?;
        return Ok(statements);
    }

    fn print_statement(&mut self) -> Result<Stmt, SyntaxError> {
        let value = self.expression();
        if let Ok(v) = value {
            let _ = self.consume(Semi, "Expect ';' after value.".into())?;
            return Ok(Stmt::Print(v));
        } else {
            return Err(value.err().unwrap());
        }
    }

    fn expression(&mut self) -> Result<Expr, SyntaxError> {
        let expr = self.or()?;
        if self.match_token(vec![Equal]) {
            let value = self.expression()?;
            if let Expr::Variable { name } = expr {
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(value),
                });
            } else if let Expr::Get { name, object } = expr {
                return Ok(Expr::Set {
                    object,
                    name,
                    value: Box::new(value),
                });
            }
        }
        return Ok(expr);
    }

    fn or(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.and()?;
        while self.match_token(vec![Or]) {
            let operator = self.previous();
            let right = self.and()?;
            expr = Expr::Logical {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }
        return Ok(expr);
    }

    fn and(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.equality()?;
        while self.match_token(vec![And]) {
            let operator = self.previous();
            let right = self.equality()?;
            expr = Expr::Logical {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }
        return Ok(expr);
    }
    fn comparison(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.term()?;

        while self.match_token(vec![Greater, GreaterEqual, Less, LessEqual]) {
            let operator = self.previous();
            let right = self.term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        return Ok(expr);
    }

    fn term(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.factor()?;

        while self.match_token(vec![Minus, Plus]) {
            let operator = self.previous();
            let right = self.factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        return Ok(expr);
    }

    fn factor(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.unary()?;

        while self.match_token(vec![Slash, Star, Mod]) {
            let operator = self.previous();
            let right = self.unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        return Ok(expr);
    }

    fn unary(&mut self) -> Result<Expr, SyntaxError> {
        if self.match_token(vec![Bang, Minus]) {
            let operator = self.previous();
            let right = self.unary()?;
            return Ok(Expr::Unary {
                operator,
                right: Box::new(right),
            });
        }
        return self.call();
    }

    fn call(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.primary()?;
        loop {
            if self.match_token(vec![LeftParen]) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(vec![Dot]) {
                let name = self.consume(Identifier, "Expect property name after '.'".into())?;
                expr = Expr::Get {
                    object: Box::new(expr),
                    name,
                }
            } else {
                break;
            }
        }
        return Ok(expr);
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr, SyntaxError> {
        let mut arguments = vec![];
        if !self.check(RightParen) {
            loop {
                if arguments.len() > 255 {
                    return Err(SyntaxError {
                        msg: "Function cannot have more than 255 arguments".to_string(),
                        line: self.previous().line
                    });
                }
                arguments.push(self.expression()?);
                if !(self.match_token(vec![Comma])) {
                    break;
                }
            }
        }
        let paren = self.consume(RightParen, "Expect ')' after arguments.".into())?;
        return Ok(Expr::Call {
            callee: Box::new(callee),
            paren,
            arguments,
        });
    }

    fn primary(&mut self) -> Result<Expr, SyntaxError> {
        if self.match_token(vec![TokenType::None]) {
            return Ok(Expr::Literal {
                value: Literal::None,
            });
        }
        if self.match_token(vec![False]) {
            return Ok(Expr::Literal {
                value: Literal::Boolean(false),
            });
        } else if self.match_token(vec![True]) {
            return Ok(Expr::Literal {
                value: Literal::Boolean(true),
            });
        } else if self.match_token(vec![Number, SoxString]) {
            return Ok(Expr::Literal {
                value: self.previous().literal,
            });
        } else if self.match_token(vec![Super]) {
            let keyword = self.previous();
            let _ = self.consume(Dot, "Expect '.' after 'super'".into())?;
            let method = self.consume(Identifier, "Expect superclass method name".into())?;
            return Ok(Expr::Super { keyword, method });
        } else if self.match_token(vec![This]) {
            return Ok(Expr::This {
                keyword: self.previous(),
            });
        } else if self.match_token(vec![Identifier]) {
            return Ok(Expr::Variable {
                name: self.previous(),
            });
        } else if self.match_token(vec![LeftParen]) {
            let expr = self.expression()?;
            let _ = self.consume(RightParen, "Expect ')' after expression.".into())?;
            return Ok(Expr::Grouping {
                expr: Box::new(expr),
            });
        }
        let token = self.tokens.peek();

        return Err(SyntaxError {
            msg: format!("Failed to parse primary token - {:?}", token),
            line: token.unwrap().line
        });
    }


    fn equality(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.comparison()?;
        while self.match_token(vec![BangEqual, EqualEqual]) {
            let operator = self.previous();
            let right = self.comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        return Ok(expr);
    }

    fn consume(&mut self, token_type: TokenType, message: String) -> Result<Token, SyntaxError> {
        if self.check(token_type) {
            let token = self.advance();
            return Ok(token.unwrap());
        }
        let prev = self.previous();
        info!("{:?}", prev);
        return Err(SyntaxError {
            msg: format!(
                "{:?}", message
            ),
            line: self.previous().line
        });
    }

    fn match_token(&mut self, token_types: Vec<TokenType>) -> bool {
        for token_type in token_types {
            if self.check(token_type) {
                self.advance();
                return true;
            }
        }
        return false;
    }

    fn check(&mut self, token_type: TokenType) -> bool {
        if self.at_end() {
            return false;
        }
        let mut peeked_value = self.tokens.peek();
        while let Some(value) = peeked_value {
            if TO_IGNORE.contains(&value.token_type) {
                self.tokens.next();
                peeked_value = self.tokens.peek();
            } else {
                break;
            }
        }
        let peeked_value = self.tokens.peek();
        return if let Some(t) = peeked_value {
            t.token_type == token_type
        } else {
            false
        };
    }

    fn advance(&mut self) -> Option<Token> {
        if !self.at_end() {
            let token = self.tokens.next();
            let return_val = token.unwrap();
            self.processed_tokens.push(return_val.clone());
            return Some(return_val);
        }
        return None
    }

    fn at_end(&mut self) -> bool {
        let mut token = self.tokens.peek();
        while token.is_some() && TO_IGNORE.contains(&token.unwrap().token_type){
            let _ = self.tokens.next();
            token = self.tokens.peek();
        }
        token.map_or(true, |t| vec![TokenType::EOF].contains(&t.token_type))
    }
}


#[cfg(test)]
mod tests {
    use crate::expr::Expr;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::stmt::Stmt::{Function, Print, Var};
    use crate::token::Literal;
    use crate::token::Token;
    use crate::token_type::TokenType::Identifier;

    #[test]
    fn test_assignment() {
        let source = "
let a = 6;
print a;";
        let tokens = Lexer::lex(source);
        let mut parser = Parser::new(tokens);

        let parse_tree = parser.parse();
        assert_eq!(parse_tree.is_ok(), true);

        let expected_stmts = vec![
            Var { name: Token { token_type: Identifier, lexeme: "a".into(), literal: Literal::None, line: 2 }, initializer: Some(Expr::Literal { value: Literal::Float(6.0) }) },
            Print(Expr::Variable { name: Token { token_type: Identifier, lexeme: "a".into(), literal: Literal::None, line: 3 } })];
        assert_eq!(parse_tree.unwrap(), expected_stmts);

    }

    #[test]
    fn test_function_statement() {
        let source = r#"
def hello_world(){
   print "hello world";
}"#;
        let tokens = Lexer::lex(source);
        let mut parser = Parser::new(tokens);

        let parse_tree = parser.parse();

        assert_eq!(parse_tree.is_ok(), true);

        let expected_stmts = vec![
            Function {
                name: Token { token_type: Identifier, lexeme: "hello_world".into(), literal: Literal::None, line: 2 },
                params: vec![],
                body: vec![Print(Expr::Literal { value: Literal::String("hello world".into()) })]
            }];
        assert_eq!(parse_tree.unwrap(), expected_stmts);

    }

    #[test]
    fn test_missing_semi_error() {
        let source = r#"
def hello_world(){
   print "hello world"
}"#;
        let tokens = Lexer::lex(source);
        let mut parser = Parser::new(tokens);

        let parse_tree = parser.parse();
        assert_eq!(parse_tree.is_err(), true);

        let errors = parse_tree.err().unwrap();
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_class_statement() {
        let source = r#"
class HelloWorld{

   hello_world(){
       return "Hello world";
   }
}"#;
        let tokens = Lexer::lex(source);
        let mut parser = Parser::new(tokens);

        let parse_tree = parser.parse();
        assert_eq!(parse_tree.is_err(), false);

    }

    #[test]
    fn test_for_statement() {
        let source = r#"
for (let i=0; i < 10; i=i+1){
    print i;
}
        "#;
        let tokens = Lexer::lex(source);
        let mut parser = Parser::new(tokens);

        let parse_tree = parser.parse();
        assert_eq!(parse_tree.is_err(), false);

    }

    #[test]
    fn test_empty_string() {
        let source = r#"

        "#;
        let tokens = Lexer::lex(source);
        let mut parser = Parser::new(tokens);

        let parse_tree = parser.parse();
        assert_eq!(parse_tree.is_err(), false);

    }
}
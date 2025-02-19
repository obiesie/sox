use log::debug;
use std::ops::Range;

use crate::token::{Float, Literal, Token};
use crate::token_type::TokenType;
use crate::token_type::TokenType::{
    And, Bang, BangEqual, Class, Colon, Comma, Def, Dot, Else, Equal, EqualEqual, False, For,
    Greater, GreaterEqual, Identifier, If, LeftBrace, LeftParen, Less, LessEqual, Let, Minus,
    Newline, Number, Or, Plus, Print, Rem, Return, RightBrace, RightParen, Semi, Slash, SoxString,
    Star, Super, This, True, While,
};

pub struct LexError {
    msg: &'static str,
}

impl LexError {
    fn new(msg: &'static str) -> Self {
        LexError { msg }
    }
}

#[derive(Clone, Debug)]
pub struct Lexer {
    source: &'static str,
    start: usize,
    current: usize,
    line: usize,
}

impl Lexer {
    pub fn new(source: &'static str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn lex(source: &'static str) -> Self {
        let lexer = Lexer::new(source);
        lexer
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn take_while<P>(&mut self, mut predicate: P) -> Option<(&'static str, Range<usize>)>
    where
        P: FnMut(char) -> bool,
    {
        let start = self.start;
        while self.peek().map_or(false, |c| predicate(c)) {
            self.advance();
        }
        let end = self.current;
        let src = &self.source[start..end];
        (start != end).then(|| (src, start..end))
    }

    fn yield_identifier(&mut self) -> Result<Token, LexError> {
        let value = self.take_while(|ch| ch.is_alphanumeric() || ch == '_');

        if let Some((ident, _)) = value {
            let token_type = match ident {
                "and" => And,
                "class" => Class,
                "else" => Else,
                "false" => False,
                "for" => For,
                "if" => If,
                "or" => Or,
                "return" => Return,
                "super" => Super,
                "this" => This,
                "true" => True,
                "let" => Let,
                "while" => While,
                "def" => Def,
                "print" => Print,
                "None" => TokenType::None,
                _ => Identifier,
            };
            Ok(self.yield_token(token_type.clone()))
        } else {
            Err(LexError::new(""))
        }
    }

    fn yield_number(&mut self) -> Result<Token, LexError> {
        let value = self.take_while(|ch| ch.is_digit(10));
        if let Some((_, rng)) = value {
            let start = rng.start;
            let mut end = rng.end;
            if let (Some(val), Some(next_val)) = (self.peek(), self.peek_next()) {
                if val == '.' && next_val.is_digit(10) {
                    self.advance();
                    let fr_value = self.take_while(|ch| ch.is_digit(10));
                    if let Some((_, rng2)) = fr_value {
                        end = rng2.end;
                    }
                }
            }
            let value: &str = &self.source[start..end];
            if value.contains(".") {
                let parsed_value = value.parse::<f64>().unwrap();
                Ok(self.yield_literal_token(Number, Literal::Float(Float(parsed_value))))
            } else {
                let parsed_value = value.parse::<i64>().unwrap();
                Ok(self.yield_literal_token(Number, Literal::Integer(parsed_value)))
            }
        } else {
            Err(LexError::new(""))
        }
    }

    fn yield_string(&mut self) -> Result<Token, LexError> {
        let value = {
            let t = self.take_while(|ch| ch != '"');
            t
        };
        self.advance();
        if let Some((str_literal, _)) = value {
            if self.is_at_end() && self.source.chars().last().unwrap() != '"' {
                panic!("Unterminated string");
            }
            let token = self.yield_literal_token(SoxString, Literal::String(&str_literal[1..]));
            Ok(token)
        } else {
            Err(LexError::new(""))
        }
    }

    fn advance(&mut self) -> Option<char> {
        let curr_char = self.source.chars().nth(self.current);
        self.current += 1;
        curr_char
    }

    fn yield_token(&mut self, token_type: TokenType) -> Token {
        self.yield_literal_token(token_type, Literal::None)
    }

    fn yield_literal_token(&mut self, token_type: TokenType, literal: Literal) -> Token {
        let text = self.source.get(self.start..self.current).unwrap_or("");
        Token::new(token_type, text, literal, self.line)
    }

    fn char_matches(&mut self, expected: char) -> bool {
        if self.peek().unwrap_or('\0') != expected {
            return false;
        }
        self.current += 1;
        true
    }

    fn token_from_result(&self, input: Result<Token, LexError>) -> Option<Token> {
        match input {
            Ok(v) => Some(v),
            Err(e) => Some(Token::new(
                TokenType::Error,
                e.msg,
                Literal::None,
                self.line,
            )),
        }
    }
    fn peek(&self) -> Option<char> {
        self.source.chars().nth(self.current)
    }

    fn peek_next(&self) -> Option<char> {
        self.source.chars().nth(self.current + 1)
    }
}




impl Iterator for Lexer {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.is_at_end() {
            self.start = self.current;
            let character = self.advance();
            let token = if let Some(character) = character {
                match character {
                    '(' => Some(self.yield_token(LeftParen)),
                    ')' => Some(self.yield_token(RightParen)),
                    '{' => Some(self.yield_token(LeftBrace)),
                    '}' => Some(self.yield_token(RightBrace)),
                    ',' => Some(self.yield_token(Comma)),
                    '.' => Some(self.yield_token(Dot)),
                    '-' => Some(self.yield_token(Minus)),
                    '+' => Some(self.yield_token(Plus)),
                    ';' => Some(self.yield_token(Semi)),
                    ':' => Some(self.yield_token(Colon)),
                    '%' => Some(self.yield_token(Rem)),
                    '*' => Some(self.yield_token(Star)),
                    '!' => {
                        let token = if self.char_matches('=') {
                            BangEqual
                        } else {
                            Bang
                        };
                        Some(self.yield_token(token))
                    }
                    '=' => {
                        let token = if self.char_matches('=') {
                            EqualEqual
                        } else {
                            Equal
                        };
                        Some(self.yield_token(token))
                    }
                    '<' => {
                        let token = if self.char_matches('=') {
                            LessEqual
                        } else {
                            Less
                        };
                        Some(self.yield_token(token))
                    }
                    '>' => {
                        let token = if self.char_matches('=') {
                            GreaterEqual
                        } else {
                            Greater
                        };
                        Some(self.yield_token(token))
                    }
                    '/' => {
                        if self.char_matches('/') {
                            let comment_text = self.take_while(|ch| ch != '\n');
                            match comment_text {
                                Some((comment, _)) => Some(Token::new(
                                    TokenType::Comment,
                                    comment,
                                    Literal::String(comment),
                                    self.line,
                                )),
                                None => Some(Token::new(
                                    TokenType::Error,
                                    "Error fetching comment tokens".into(),
                                    Literal::None,
                                    self.line,
                                )),
                            }
                        } else if self.char_matches('*') {
                            let mut comment_ranges = vec![];
                            loop {
                                let comment_text = self.take_while(|ch| ch != '*');
                                match comment_text {
                                    Some((_, b)) => {
                                        self.advance();
                                        comment_ranges.push(b);
                                        if self.peek() == Some('/') {
                                            self.advance();
                                            break;
                                        }
                                    }
                                    None => {
                                        panic!("Unterminated comment");
                                    }
                                }
                            }
                            let comments = self
                                .source
                                .get(
                                    comment_ranges[0].start
                                        ..comment_ranges[comment_ranges.len() - 1].end,
                                )
                                .unwrap_or("");
                            let newline_count = comments.matches('\n').count();
                            self.line = self.line + newline_count;
                            println!("Found comment: {}", comments);
                            Some(Token::new(
                                TokenType::Comment,
                                comments,
                                Literal::String(comments),
                                self.line,
                            ))
                        } else {
                            Some(self.yield_token(Slash))
                        }
                    }
                    '\n' => {
                        let newline_token = self.yield_token(Newline);
                        self.line += 1;
                        Some(newline_token)
                    }
                    '"' => {
                        let sox_string = self.yield_string();
                        self.token_from_result(sox_string)
                    }
                    'A'..='Z' | 'a'..='z' | '_' => {
                        let ident_val = self.yield_identifier();
                        self.token_from_result(ident_val)
                    }
                    '0'..='9' => {
                        let numer_val = self.yield_number();
                        self.token_from_result(numer_val)
                    }
                    ' ' => Some(self.yield_token(TokenType::Whitespace)),
                    _ => {
                        debug!("Token -{character} - not in allowed set of valid tokens");
                        Some(Token::new(
                            TokenType::Error,
                            "Token -{character} - not in allowed set of valid tokens".into(),
                            Literal::None,
                            self.line,
                        ))
                    }
                }
            } else {
                Some(Token::new(
                    TokenType::Error,
                    "No more characters to lex".into(),
                    Literal::None,
                    self.line,
                ))
            };
            token
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::TO_IGNORE;
    use crate::token::{Literal, Token};
    use crate::token_type::TokenType;

    #[test]
    fn test_class_lex() {
        let source = r#"
class A {
    method(){
        print "A method";
    }
}"#;
        let lexer = Lexer::lex(source);
        let tokens = lexer.collect::<Vec<Token>>();

        let non_whitespace_tokens = tokens
            .into_iter()
            .filter(|token| !TO_IGNORE.contains(&token.token_type))
            .collect::<Vec<Token>>();
        assert_eq!(non_whitespace_tokens.len(), 12);
        assert_eq!(
            vec![
                Token::new(TokenType::Class, "class".into(), Literal::None, 2),
                Token::new(TokenType::Identifier, "A".into(), Literal::None, 2)
            ],
            non_whitespace_tokens[..2]
        )
    }

    #[test]
    fn test_var_lex() {
        let source = "let v = 10;";
        let lexer = Lexer::lex(source);
        let tokens = lexer.collect::<Vec<Token>>();

        let non_whitespace_tokens = tokens
            .into_iter()
            .filter(|token| !TO_IGNORE.contains(&token.token_type))
            .collect::<Vec<Token>>();
        assert_eq!(non_whitespace_tokens.len(), 5);
    }

    #[test]
    fn test_func_lex() {
        let source = "
def fib(n) {
    if (n == 0 or n == 1) {
        return n;
    }
    return fib(n-1) + fib(n-2);
}";
        let lexer = Lexer::lex(source);
        let tokens = lexer.collect::<Vec<Token>>();

        let non_whitespace_tokens = tokens
            .into_iter()
            .filter(|token| !TO_IGNORE.contains(&token.token_type))
            .collect::<Vec<Token>>();
        assert_eq!(non_whitespace_tokens.len(), 37);
    }

    #[test]
    fn test_line_numbers() {
        let source = r#"/*
A very simple program to test our vm.

def fib(n) {
    if (n == 0 or n == 1) {
        return n;
    }
    return fib(n-1) + fib(n-2);
}

let a = fib(6);

print a;*/

for (let i=0; i < 10; i=i+1){
    print i
}
"#;
        let lexer = Lexer::lex(source);
        let tokens = lexer.collect::<Vec<Token>>();

        for token in tokens {
            if token.token_type == TokenType::For {
                assert_eq!(token.line, 15);
            }
            if token.token_type == TokenType::Print {
                assert_eq!(token.line, 16)
            }
        }
    }
}

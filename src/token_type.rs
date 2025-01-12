#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Hash, Ord)]
pub enum TokenType {
    // Single character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftSqb,
    RightSqb,
    Colon,
    Comma,
    Semi,
    Plus,
    Minus,
    Star,
    Slash,
    Dot,
    Rem,

    // One or two character token
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual, 
    Less,
    LessEqual,
   
   

    // Literals
    Identifier,
    Number,
    SoxString,

    // Keywords
    And,
    Class,
    Else,
    False,
    For,
    If,
    Or,
    Return,
    Super,
    True,
    While,
    Def,
    This,
    Let,
    Print,
    None,
    
    Error,
    EOF,

    Newline,
    Whitespace,
    Indent,
    Dedent,
    Comment,
    CommentMarker,
}

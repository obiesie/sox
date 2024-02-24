#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Hash)]
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
    Less,
    Greater,
    Equal,
    EqualEqual,
    NotEqual,
    LessEqual,
    GreaterEqual,
    Bang,
    BangEqual,

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
    None,
    Print,

    Newline,
    Whitespace,
    Indent,
    Dedent,
    Comment,
    CommentMarker,

    Error,
    EOF,
}

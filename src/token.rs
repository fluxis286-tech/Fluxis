// FLUXIS — token.rs (v10.0)
// Added: InterpolatedString, DotDot (range), QuestionDot (optional chain),
//        QuestionQuestion (null coalesce), Async, Await, Try, Catch, Arrow (->)

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}
impl Span {
    pub fn new(l: usize, c: usize) -> Self {
        Self { line: l, col: c }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
impl Token {
    pub fn new(kind: TokenKind, l: usize, c: usize) -> Self {
        Self {
            kind,
            span: Span::new(l, c),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Number(i64),
    Float(f64),
    String(String),
    /// A string with interpolated expressions: `"Hello {name}!"`
    /// Stored as alternating literal/expr segments
    Identifier(String),

    // core keywords
    Start,
    If,
    Else,
    Fn,
    Return,
    Struct,
    Enum,
    Break,
    Continue,
    While,
    Do,
    For,
    Async,
    Await,

    // nil literal + match + try/catch
    Nil,
    Match,
    Try,
    Catch,

    // DOP keywords
    Dotion,
    On,
    Tick,
    Self_,
    Actor,
    With,
    Extends,
    Import,

    // for-in / in operator
    In,
    NotIn,

    // default parameter
    // (no new token needed — parsed as Assign in param list)

    // delimiters
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    ColonColon,
    Dot,
    FatArrow,
    DotDot,           // .. (range operator)
    QuestionDot,      // ?. (optional chain)
    QuestionQuestion, // ?? (null coalesce)
    Arrow,            // -> (return type annotation)

    // operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusPlus,
    MinusMinus,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    Assign,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    EqualEqual,
    NotEqual,
    And,
    Or,
    Not,
    End,
    EOF,
}

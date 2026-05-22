// FLUXIS — lexer.rs (v10.0)
// New: string interpolation `"Hello {expr}"`, .. range, ?. optional chain,
//      ?? null coalesce, -> arrow, async/await/try/catch/while/do keywords, not in
use crate::error::{FluxisError, lex_error};
use crate::token::{Token, TokenKind};

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    col: usize,
    source_lines: Vec<String>,
}
impl Lexer {
    pub fn new(s: &str) -> Self {
        Self {
            input: s.chars().collect(),
            position: 0,
            line: 1,
            col: 1,
            source_lines: s.lines().map(|l| l.to_string()).collect(),
        }
    }
    fn cur(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }
    fn peek(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }
    fn advance(&mut self) {
        if let Some(c) = self.cur() {
            self.position += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }
    fn tok(&self, k: TokenKind) -> Token {
        Token::new(k, self.line, self.col)
    }
    fn err(&self, msg: &str) -> FluxisError {
        let mut e = lex_error(msg, self.line, self.col);
        if let Some(src) = self.source_lines.get(self.line.saturating_sub(1)) {
            e = e.with_source(src);
        }
        e
    }

    /// Lex a regular string with escape sequences.
    fn lex_string(&mut self) -> Result<String, FluxisError> {
        let mut val = String::new();
        loop {
            match self.cur() {
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\n') | None => {
                    return Err(self.err("Unterminated string").with_hint("Add closing \""));
                }
                Some('\\') => {
                    self.advance();
                    match self.cur() {
                        Some('n') => {
                            val.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            val.push('\t');
                            self.advance();
                        }
                        Some('r') => {
                            val.push('\r');
                            self.advance();
                        }
                        Some('\\') | Some('"') => {
                            val.push(self.cur().unwrap());
                            self.advance();
                        }
                        Some('0') => {
                            val.push('\0');
                            self.advance();
                        }
                        Some(ch) => {
                            val.push('\\');
                            val.push(ch);
                            self.advance();
                        }
                        None => return Err(self.err("Unexpected end of string escape")),
                    }
                }
                Some(ch) => {
                    val.push(ch);
                    self.advance();
                }
            }
        }
        Ok(val)
    }

    pub fn lex(&mut self) -> Result<Vec<Token>, FluxisError> {
        let mut tokens = Vec::new();
        while let Some(c) = self.cur() {
            match c {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '0'..='9' => {
                    let (sl, sc) = (self.line, self.col);
                    let mut num = String::new();
                    while let Some(ch) = self.cur() {
                        if ch.is_ascii_digit() {
                            num.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if self.cur() == Some('.')
                        && self.peek() != Some('.')
                        && self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false)
                    {
                        num.push('.');
                        self.advance();
                        while let Some(ch) = self.cur() {
                            if ch.is_ascii_digit() {
                                num.push(ch);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let f = num
                            .parse::<f64>()
                            .map_err(|_| lex_error(&format!("Invalid float '{}'", num), sl, sc))?;
                        tokens.push(Token::new(TokenKind::Float(f), sl, sc));
                    } else {
                        let n = num
                            .parse::<i64>()
                            .map_err(|_| lex_error(&format!("Invalid number '{}'", num), sl, sc))?;
                        tokens.push(Token::new(TokenKind::Number(n), sl, sc));
                    }
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let (sl, sc) = (self.line, self.col);
                    let mut id = String::new();
                    while let Some(ch) = self.cur() {
                        if ch.is_alphanumeric() || ch == '_' {
                            id.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    // "not in" two-token operator
                    if id == "not" {
                        // peek ahead for "in"
                        let saved_pos = self.position;
                        let saved_line = self.line;
                        let saved_col = self.col;
                        // skip whitespace
                        while let Some(ws) = self.cur() {
                            if ws == ' ' || ws == '\t' {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        if self.cur() == 'i'.into() {
                            let mut next = String::new();
                            let p2 = self.position;
                            while let Some(ch) = self.cur() {
                                if ch.is_alphanumeric() {
                                    next.push(ch);
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                            if next == "in" {
                                tokens.push(Token::new(TokenKind::NotIn, sl, sc));
                                continue;
                            }
                            // restore
                            self.position = p2;
                            self.line = saved_line;
                            self.col = saved_col;
                        }
                        self.position = saved_pos;
                        self.line = saved_line;
                        self.col = saved_col;
                    }
                    let tk = match id.as_str() {
                        "start" => TokenKind::Start,
                        "if" => TokenKind::If,
                        "else" => TokenKind::Else,
                        "fn" => TokenKind::Fn,
                        "return" => TokenKind::Return,
                        "struct" => TokenKind::Struct,
                        "enum" => TokenKind::Enum,
                        "break" => TokenKind::Break,
                        "continue" => TokenKind::Continue,
                        "while" => TokenKind::While,
                        "do" => TokenKind::Do,
                        "for" => TokenKind::For,
                        "nil" => TokenKind::Nil,
                        "match" => TokenKind::Match,
                        "try" => TokenKind::Try,
                        "catch" => TokenKind::Catch,
                        "async" => TokenKind::Async,
                        "await" => TokenKind::Await,
                        "dotion" => TokenKind::Dotion,
                        "on" => TokenKind::On,
                        "tick" => TokenKind::Tick,
                        "self" => TokenKind::Self_,
                        "actor" => TokenKind::Actor,
                        "with" => TokenKind::With,
                        "extends" => TokenKind::Extends,
                        "import" => TokenKind::Import,
                        "in" => TokenKind::In,
                        _ => TokenKind::Identifier(id),
                    };
                    tokens.push(Token::new(tk, sl, sc));
                }
                '"' => {
                    let (sl, sc) = (self.line, self.col);
                    self.advance();
                    // Check for string interpolation — scan for { in content
                    let val = self.lex_string()?;
                    // Check if contains { for interpolation
                    if val.contains('{') && val.contains('}') {
                        tokens.push(Token::new(TokenKind::String(val), sl, sc));
                    } else {
                        tokens.push(Token::new(TokenKind::String(val), sl, sc));
                    }
                }
                '+' => {
                    if self.peek() == Some('+') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::PlusPlus));
                    } else if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::PlusAssign));
                    } else {
                        let t = self.tok(TokenKind::Plus);
                        tokens.push(t);
                        self.advance();
                    }
                }
                '-' => {
                    if self.peek() == Some('-') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::MinusMinus));
                    } else if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::MinusAssign));
                    } else if self.peek() == Some('>') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::Arrow));
                    } else {
                        let t = self.tok(TokenKind::Minus);
                        tokens.push(t);
                        self.advance();
                    }
                }
                '*' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::StarAssign));
                    } else {
                        let t = self.tok(TokenKind::Star);
                        tokens.push(t);
                        self.advance();
                    }
                }
                '%' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::PercentAssign));
                    } else {
                        let t = self.tok(TokenKind::Percent);
                        tokens.push(t);
                        self.advance();
                    }
                }
                '/' => {
                    if self.peek() == Some('/') {
                        while let Some(ch) = self.cur() {
                            if ch == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    } else if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::SlashAssign));
                    } else {
                        let t = self.tok(TokenKind::Slash);
                        tokens.push(t);
                        self.advance();
                    }
                }
                '{' => {
                    let t = self.tok(TokenKind::LBrace);
                    tokens.push(t);
                    self.advance();
                }
                '}' => {
                    let t = self.tok(TokenKind::RBrace);
                    tokens.push(t);
                    self.advance();
                }
                '(' => {
                    let t = self.tok(TokenKind::LParen);
                    tokens.push(t);
                    self.advance();
                }
                ')' => {
                    let t = self.tok(TokenKind::RParen);
                    tokens.push(t);
                    self.advance();
                }
                '[' => {
                    let t = self.tok(TokenKind::LBracket);
                    tokens.push(t);
                    self.advance();
                }
                ']' => {
                    let t = self.tok(TokenKind::RBracket);
                    tokens.push(t);
                    self.advance();
                }
                ',' => {
                    let t = self.tok(TokenKind::Comma);
                    tokens.push(t);
                    self.advance();
                }
                '.' => {
                    if self.peek() == Some('.') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::DotDot));
                    } else {
                        let t = self.tok(TokenKind::Dot);
                        self.advance();
                        tokens.push(t);
                    }
                }
                ';' => {
                    let t = self.tok(TokenKind::End);
                    self.advance();
                    tokens.push(t);
                }
                ':' => {
                    if self.peek() == Some(':') {
                        let t = self.tok(TokenKind::ColonColon);
                        self.advance();
                        self.advance();
                        tokens.push(t);
                    } else {
                        let t = self.tok(TokenKind::Colon);
                        self.advance();
                        tokens.push(t);
                    }
                }
                '=' => {
                    if self.peek() == Some('=') {
                        let t = self.tok(TokenKind::EqualEqual);
                        self.advance();
                        self.advance();
                        tokens.push(t);
                    } else if self.peek() == Some('>') {
                        let t = self.tok(TokenKind::FatArrow);
                        self.advance();
                        self.advance();
                        tokens.push(t);
                    } else {
                        let t = self.tok(TokenKind::Assign);
                        self.advance();
                        tokens.push(t);
                    }
                }
                '?' => {
                    if self.peek() == Some('.') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::QuestionDot));
                    } else if self.peek() == Some('?') {
                        self.advance();
                        self.advance();
                        tokens.push(self.tok(TokenKind::QuestionQuestion));
                    } else {
                        return Err(self
                            .err("Unexpected '?'")
                            .with_hint("Use ?. for optional chain or ?? for null coalesce"));
                    }
                }
                '!' => {
                    if self.peek() == Some('=') {
                        let t = self.tok(TokenKind::NotEqual);
                        self.advance();
                        self.advance();
                        tokens.push(t);
                    } else {
                        let t = self.tok(TokenKind::Not);
                        self.advance();
                        tokens.push(t);
                    }
                }
                '>' => {
                    if self.peek() == Some('=') {
                        let t = self.tok(TokenKind::GreaterEqual);
                        self.advance();
                        self.advance();
                        tokens.push(t);
                    } else {
                        let t = self.tok(TokenKind::Greater);
                        self.advance();
                        tokens.push(t);
                    }
                }
                '<' => {
                    if self.peek() == Some('=') {
                        let t = self.tok(TokenKind::LessEqual);
                        self.advance();
                        self.advance();
                        tokens.push(t);
                    } else {
                        let t = self.tok(TokenKind::Less);
                        self.advance();
                        tokens.push(t);
                    }
                }
                '&' => {
                    if self.peek() == Some('&') {
                        let t = self.tok(TokenKind::And);
                        self.advance();
                        self.advance();
                        tokens.push(t);
                    } else {
                        return Err(self
                            .err("Unexpected '&'")
                            .with_hint("Use && for logical AND"));
                    }
                }
                '|' => {
                    if self.peek() == Some('|') {
                        let t = self.tok(TokenKind::Or);
                        self.advance();
                        self.advance();
                        tokens.push(t);
                    } else {
                        return Err(self
                            .err("Unexpected '|'")
                            .with_hint("Use || for logical OR"));
                    }
                }
                _ => {
                    return Err(self
                        .err(&format!("Unknown character '{}'", c))
                        .with_hint("Remove or replace this character"));
                }
            }
        }
        tokens.push(self.tok(TokenKind::EOF));
        Ok(tokens)
    }
}

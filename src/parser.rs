// FLUXIS — parser.rs  (v4.0)
// NEW: nil, match, for-in, compound assignment (+=/-=/*=/=/%=), self compound assign,
//      string escapes handled in lexer (transparent here), => fat arrow for match arms
#[allow(unused_imports)]
use crate::ast::{
    DotionMethod, Expr, Handler, MatchArm, MatchPattern, Param, Statement, TypeAnnotation,
};
use crate::error::{FluxisError, parse_error};
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    source_lines: Vec<String>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, source: &str) -> Self {
        Self {
            tokens,
            position: 0,
            source_lines: source.lines().map(|l| l.to_string()).collect(),
        }
    }
    fn cur(&self) -> &Token {
        self.tokens
            .get(self.position)
            .unwrap_or(self.tokens.last().unwrap())
    }
    fn peek_kind(&self, o: usize) -> &TokenKind {
        self.tokens
            .get(self.position + o)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::EOF)
    }
    fn advance(&mut self) {
        if self.position < self.tokens.len() - 1 {
            self.position += 1;
        }
    }
    fn err<T>(&self, msg: &str) -> Result<T, FluxisError> {
        let t = self.cur();
        let mut e = parse_error(msg, t.span.line, t.span.col);
        if let Some(src) = self.source_lines.get(t.span.line.saturating_sub(1)) {
            e = e.with_source(src);
        }
        Err(e)
    }
    fn expect(&mut self, kind: TokenKind) -> Result<(), FluxisError> {
        if std::mem::discriminant(&self.cur().kind) != std::mem::discriminant(&kind) {
            let t = self.cur();
            let mut e = parse_error(
                &format!("Expected {:?} but found {:?}", kind, self.cur().kind),
                t.span.line,
                t.span.col,
            );
            if let Some(src) = self.source_lines.get(t.span.line.saturating_sub(1)) {
                e = e.with_source(src);
            }
            let hint = match &kind {
                TokenKind::End => Some("Statements end with ;"),
                TokenKind::LBrace => Some("Expected {"),
                TokenKind::RBrace => Some("Missing }"),
                TokenKind::LParen => Some("Expected ("),
                TokenKind::RParen => Some("Missing )"),
                TokenKind::RBracket => Some("Missing ]"),
                TokenKind::Assign => Some("Did you forget = before the value?"),
                _ => None,
            };
            if let Some(h) = hint {
                e = e.with_hint(h);
            }
            return Err(e);
        }
        self.advance();
        Ok(())
    }
    fn try_type(&mut self) -> Option<TypeAnnotation> {
        if let TokenKind::Identifier(ref n) = self.cur().kind.clone() {
            if let Some(ta) = TypeAnnotation::from_str(n) {
                self.advance();
                return Some(ta);
            }
        }
        None
    }

    pub fn parse(&mut self) -> Result<Vec<Statement>, FluxisError> {
        let mut prog = Vec::new();
        while !matches!(self.cur().kind, TokenKind::Start | TokenKind::EOF) {
            prog.push(self.parse_stmt()?);
        }
        if matches!(self.cur().kind, TokenKind::Start) {
            self.advance();
            self.expect(TokenKind::LBrace)?;
            while !matches!(self.cur().kind, TokenKind::RBrace | TokenKind::EOF) {
                prog.push(self.parse_stmt()?);
            }
            self.advance();
        }
        Ok(prog)
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, FluxisError> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !matches!(self.cur().kind, TokenKind::RBrace | TokenKind::EOF) {
            stmts.push(self.parse_stmt()?);
        }
        self.advance();
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Statement, FluxisError> {
        match self.cur().kind.clone() {
            TokenKind::Import => {
                self.advance();
                let m = match self.cur().kind.clone() {
                    TokenKind::String(s) => {
                        self.advance();
                        s
                    }
                    _ => return self.err("Expected module name string after 'import'"),
                };
                self.expect(TokenKind::End)?;
                return Ok(Statement::Import { module: m });
            }
            TokenKind::Dotion => return self.parse_dotion_def(),
            TokenKind::Actor => return self.parse_actor_def(),
            TokenKind::Struct => return self.parse_struct_def(),
            TokenKind::Enum => return self.parse_enum_def(),
            TokenKind::Fn => return self.parse_fn_def(),
            TokenKind::Tick => return self.parse_tick(),
            TokenKind::Return => {
                self.advance();
                // bare return.. with no expression → return nil
                if matches!(self.cur().kind, TokenKind::End) {
                    self.advance();
                    return Ok(Statement::Return { value: Expr::Nil });
                }
                let v = self.parse_expr()?;
                self.expect(TokenKind::End)?;
                return Ok(Statement::Return { value: v });
            }
            TokenKind::Break => {
                self.advance();
                self.expect(TokenKind::End)?;
                return Ok(Statement::Break);
            }
            TokenKind::While => return self.parse_while(),
            TokenKind::Do => return self.parse_do_while(),
            TokenKind::Try => return self.parse_try_catch(),
            TokenKind::Async => return self.parse_async_fn(),
            TokenKind::Continue => {
                self.advance();
                self.expect(TokenKind::End)?;
                return Ok(Statement::Continue);
            }
            TokenKind::If => return self.parse_if(),
            TokenKind::Match => return self.parse_match(),
            TokenKind::For => return self.parse_for(),
            // self.field = val..  OR  self.field += val..
            TokenKind::Self_ => {
                if matches!(self.peek_kind(1), TokenKind::Dot) {
                    if let TokenKind::Identifier(ref f) = self.peek_kind(2).clone() {
                        let field = f.clone();
                        // self.field[index] = val..   OR  self.field[index] += val..
                        if matches!(self.peek_kind(3), TokenKind::LBracket) {
                            self.advance();
                            self.advance();
                            self.advance();
                            self.advance(); // self . field [
                            let idx = self.parse_expr()?;
                            self.expect(TokenKind::RBracket)?;
                            // = val
                            if matches!(self.cur().kind, TokenKind::Assign) {
                                self.advance();
                                let v = self.parse_expr()?;
                                self.expect(TokenKind::End)?;
                                // Desugar: self.field[idx] = v
                                // → temp = self.field; temp[idx] = v; self.field = temp
                                // We encode this as SelfFieldAssign with an IndexExpr trick:
                                // Store as SelfIndexAssign which we'll add
                                return Ok(Statement::SelfFieldAssign {
                                    field: field.clone(),
                                    value: Expr::Binary {
                                        left: Box::new(Expr::Binary {
                                            left: Box::new(Expr::Field {
                                                object: Box::new(Expr::Self_),
                                                field: field.clone(),
                                            }),
                                            op: "__idx_set__".to_string(),
                                            right: Box::new(idx),
                                        }),
                                        op: "__val__".to_string(),
                                        right: Box::new(v),
                                    },
                                });
                            }
                            // += val
                            let op_opt = match self.cur().kind {
                                TokenKind::PlusAssign => Some("+"),
                                TokenKind::MinusAssign => Some("-"),
                                TokenKind::StarAssign => Some("*"),
                                TokenKind::SlashAssign => Some("/"),
                                TokenKind::PercentAssign => Some("%"),
                                _ => None,
                            };
                            if let Some(op) = op_opt {
                                let op = op.to_string();
                                self.advance();
                                let rhs = self.parse_expr()?;
                                self.expect(TokenKind::End)?;
                                // Desugar: self.field[idx] += rhs
                                // → self.field[idx] = self.field[idx] op rhs
                                let idx_expr = idx.clone();
                                let current = Expr::Index {
                                    object: Box::new(Expr::Field {
                                        object: Box::new(Expr::Self_),
                                        field: field.clone(),
                                    }),
                                    index: Box::new(idx_expr.clone()),
                                };
                                let combined = Expr::Binary {
                                    left: Box::new(current),
                                    op,
                                    right: Box::new(rhs),
                                };
                                return Ok(Statement::SelfFieldAssign {
                                    field: field.clone(),
                                    value: Expr::Binary {
                                        left: Box::new(Expr::Binary {
                                            left: Box::new(Expr::Field {
                                                object: Box::new(Expr::Self_),
                                                field: field.clone(),
                                            }),
                                            op: "__idx_set__".to_string(),
                                            right: Box::new(idx_expr),
                                        }),
                                        op: "__val__".to_string(),
                                        right: Box::new(combined),
                                    },
                                });
                            }
                        }
                        // self.field = val..
                        if matches!(self.peek_kind(3), TokenKind::Assign) {
                            self.advance();
                            self.advance();
                            self.advance();
                            self.advance();
                            let v = self.parse_expr()?;
                            self.expect(TokenKind::End)?;
                            return Ok(Statement::SelfFieldAssign { field, value: v });
                        }
                        // self.field += val..  etc.
                        let op_opt = match self.peek_kind(3) {
                            TokenKind::PlusAssign => Some("+"),
                            TokenKind::MinusAssign => Some("-"),
                            TokenKind::StarAssign => Some("*"),
                            TokenKind::SlashAssign => Some("/"),
                            TokenKind::PercentAssign => Some("%"),
                            _ => None,
                        };
                        if let Some(op) = op_opt {
                            let op = op.to_string();
                            self.advance();
                            self.advance();
                            self.advance();
                            self.advance();
                            let v = self.parse_expr()?;
                            self.expect(TokenKind::End)?;
                            return Ok(Statement::SelfCompoundAssign {
                                field,
                                op,
                                value: v,
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        if let TokenKind::Identifier(name) = self.cur().kind.clone() {
            if name == "out" {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let e = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                self.expect(TokenKind::End)?;
                return Ok(Statement::Print { value: e });
            }
            // name[idx]=val..
            if matches!(self.peek_kind(1), TokenKind::LBracket) {
                self.advance();
                self.advance();
                let idx = self.parse_expr()?;
                self.expect(TokenKind::RBracket)?;
                self.expect(TokenKind::Assign)?;
                let v = self.parse_expr()?;
                self.expect(TokenKind::End)?;
                return Ok(Statement::IndexAssign {
                    object: name,
                    index: idx,
                    value: v,
                });
            }
            // name.method(args)..  OR  name.field = val..  OR  name.field += val..
            if matches!(self.peek_kind(1), TokenKind::Dot) {
                let member_opt = match self.peek_kind(2) {
                    TokenKind::Identifier(f) => Some(f.clone()),
                    TokenKind::Tick => Some("tick".to_string()),
                    TokenKind::On => Some("on".to_string()),
                    TokenKind::With => Some("with".to_string()),
                    _ => None,
                };
                if let Some(member) = member_opt {
                    // name.method(args)..
                    if matches!(self.peek_kind(3), TokenKind::LParen) {
                        self.advance();
                        self.advance();
                        self.advance();
                        self.advance(); // (
                        let mut args = Vec::new();
                        while !matches!(self.cur().kind, TokenKind::RParen | TokenKind::EOF) {
                            args.push(self.parse_expr()?);
                            if matches!(self.cur().kind, TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::RParen)?;
                        self.expect(TokenKind::End)?;
                        return Ok(Statement::MethodCallStmt {
                            object: name,
                            method: member,
                            args,
                        });
                    }
                    // name.field = val..
                    if matches!(self.peek_kind(3), TokenKind::Assign) {
                        self.advance();
                        self.advance();
                        self.advance();
                        self.advance();
                        let v = self.parse_expr()?;
                        self.expect(TokenKind::End)?;
                        return Ok(Statement::FieldAssign {
                            object: name,
                            field: member,
                            value: v,
                        });
                    }
                    // name.field += val..  etc.
                    let op_opt = match self.peek_kind(3) {
                        TokenKind::PlusAssign => Some("+"),
                        TokenKind::MinusAssign => Some("-"),
                        TokenKind::StarAssign => Some("*"),
                        TokenKind::SlashAssign => Some("/"),
                        TokenKind::PercentAssign => Some("%"),
                        _ => None,
                    };
                    if let Some(op) = op_opt {
                        let op = op.to_string();
                        // Desugar: name.field += v  =>  FieldAssign { name, member, Binary(Field(name,member), op, v) }
                        self.advance();
                        self.advance();
                        self.advance();
                        self.advance();
                        let rhs = self.parse_expr()?;
                        self.expect(TokenKind::End)?;
                        let lhs = Expr::Field {
                            object: Box::new(Expr::Identifier(name.clone())),
                            field: member.clone(),
                        };
                        let combined = Expr::Binary {
                            left: Box::new(lhs),
                            op,
                            right: Box::new(rhs),
                        };
                        return Ok(Statement::FieldAssign {
                            object: name,
                            field: member,
                            value: combined,
                        });
                    }
                }
            }
            // name: type = val..
            if matches!(self.peek_kind(1), TokenKind::Colon) {
                if let TokenKind::Identifier(ref tn) = self.peek_kind(2).clone() {
                    if TypeAnnotation::from_str(tn).is_some()
                        && matches!(self.peek_kind(3), TokenKind::Assign)
                    {
                        self.advance();
                        self.advance();
                        let ta = self.try_type().unwrap();
                        self.advance();
                        let v = self.parse_expr()?;
                        self.expect(TokenKind::End)?;
                        return Ok(Statement::Assignment {
                            name,
                            type_annotation: Some(ta),
                            value: v,
                        });
                    }
                }
            }
            // name = val..
            if matches!(self.peek_kind(1), TokenKind::Assign) {
                self.advance();
                self.advance();
                let v = self.parse_expr()?;
                self.expect(TokenKind::End)?;
                return Ok(Statement::Assignment {
                    name,
                    type_annotation: None,
                    value: v,
                });
            }
            // compound assignment: name += val..  name -= val..  etc.
            let op_opt2 = match self.peek_kind(1) {
                TokenKind::PlusAssign => Some("+"),
                TokenKind::MinusAssign => Some("-"),
                TokenKind::StarAssign => Some("*"),
                TokenKind::SlashAssign => Some("/"),
                TokenKind::PercentAssign => Some("%"),
                _ => None,
            };
            if let Some(op) = op_opt2 {
                let op = op.to_string();
                self.advance();
                self.advance();
                let v = self.parse_expr()?;
                self.expect(TokenKind::End)?;
                return Ok(Statement::CompoundAssign { name, op, value: v });
            }
            // name(args)..
            if matches!(self.peek_kind(1), TokenKind::LParen) {
                let e = self.parse_expr()?;
                self.expect(TokenKind::End)?;
                return Ok(Statement::Assignment {
                    name: "__discard__".into(),
                    type_annotation: None,
                    value: e,
                });
            }
            // name++..
            if matches!(self.peek_kind(1), TokenKind::PlusPlus) {
                self.advance();
                self.advance();
                self.expect(TokenKind::End)?;
                return Ok(Statement::Increment { name });
            }
            // name--..
            if matches!(self.peek_kind(1), TokenKind::MinusMinus) {
                self.advance();
                self.advance();
                self.expect(TokenKind::End)?;
                return Ok(Statement::Decrement { name });
            }
        }
        self.err(&format!("Unexpected token {:?}", self.cur().kind))
    }

    // ── MATCH ─────────────────────────────────────────────────────────
    // match expr {
    //   Color::Red   => { ... }
    //   "hello"      => { ... }
    //   42           => { ... }
    //   true         => { ... }
    //   nil          => { ... }
    //   _            => { ... }
    // }
    fn parse_match(&mut self) -> Result<Statement, FluxisError> {
        self.advance(); // consume 'match'
        let value = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while !matches!(self.cur().kind, TokenKind::RBrace | TokenKind::EOF) {
            let pattern = self.parse_match_pattern()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_block()?;
            arms.push(crate::ast::MatchArm { pattern, body });
            // optional comma between arms
            if matches!(self.cur().kind, TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Statement::Match { value, arms })
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, FluxisError> {
        match self.cur().kind.clone() {
            // wildcard: _
            TokenKind::Identifier(ref n) if n == "_" => {
                self.advance();
                Ok(MatchPattern::Wildcard)
            }
            // enum variant: Name::Variant
            TokenKind::Identifier(ref enum_name)
                if matches!(self.peek_kind(1), TokenKind::ColonColon) =>
            {
                let en = enum_name.clone();
                self.advance();
                self.advance();
                match self.cur().kind.clone() {
                    TokenKind::Identifier(v) => {
                        self.advance();
                        Ok(MatchPattern::EnumVariant {
                            enum_name: en,
                            variant: v,
                        })
                    }
                    _ => self.err("Expected variant name after '::'"),
                }
            }
            // nil
            TokenKind::Nil => {
                self.advance();
                Ok(MatchPattern::Literal(Expr::Nil))
            }
            // bool
            TokenKind::Identifier(ref n) if n == "true" => {
                self.advance();
                Ok(MatchPattern::Literal(Expr::Bool(true)))
            }
            TokenKind::Identifier(ref n) if n == "false" => {
                self.advance();
                Ok(MatchPattern::Literal(Expr::Bool(false)))
            }
            // number (possibly negative)
            TokenKind::Minus if matches!(self.peek_kind(1), TokenKind::Number(_)) => {
                self.advance();
                if let TokenKind::Number(n) = self.cur().kind.clone() {
                    self.advance();
                    Ok(MatchPattern::Literal(Expr::Number(-n)))
                } else {
                    self.err("Expected number after '-' in match pattern")
                }
            }
            TokenKind::Number(n) => {
                let n = n;
                self.advance();
                Ok(MatchPattern::Literal(Expr::Number(n)))
            }
            TokenKind::Float(f) => {
                let f = f;
                self.advance();
                Ok(MatchPattern::Literal(Expr::Float(f)))
            }
            TokenKind::String(s) => {
                let s = s;
                self.advance();
                Ok(MatchPattern::Literal(Expr::String(s)))
            }
            _ => self
                .err("Expected match pattern: _, EnumName::Variant, number, string, bool, or nil"),
        }
    }

    // ── DOTION DEF ────────────────────────────────────────────────────
    fn parse_dotion_def(&mut self) -> Result<Statement, FluxisError> {
        self.advance();
        let name = match self.cur().kind.clone() {
            TokenKind::Identifier(n) => {
                self.advance();
                n
            }
            _ => return self.err("Expected dotion name"),
        };

        // extends: optional parent type
        let extends = if matches!(self.cur().kind, TokenKind::Extends) {
            self.advance();
            match self.cur().kind.clone() {
                TokenKind::Identifier(p) => {
                    self.advance();
                    Some(p)
                }
                _ => return self.err("Expected parent dotion name after 'extends'"),
            }
        } else {
            None
        };

        self.expect(TokenKind::LBrace)?;
        let (fields, methods, handlers) = self.parse_dotion_body()?;

        // Optional modifiers after closing brace
        let mut brain: Option<String> = None;
        let mut tags: Vec<String> = Vec::new();
        let mut tick_priority: i64 = 0;

        loop {
            match self.cur().kind.clone() {
                // with ActorName
                TokenKind::With => {
                    self.advance();
                    match self.cur().kind.clone() {
                        TokenKind::Identifier(b) => {
                            self.advance();
                            brain = Some(b);
                        }
                        _ => return self.err("Expected actor name after 'with'"),
                    }
                }
                // tags: ["enemy", "ground"]
                TokenKind::Identifier(ref kw) if kw == "tags" => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    self.expect(TokenKind::LBracket)?;
                    while !matches!(self.cur().kind, TokenKind::RBracket | TokenKind::EOF) {
                        match self.cur().kind.clone() {
                            TokenKind::String(s) => {
                                self.advance();
                                tags.push(s);
                                if matches!(self.cur().kind, TokenKind::Comma) {
                                    self.advance();
                                }
                            }
                            _ => return self.err("tags must be string literals"),
                        }
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                // tick_priority: 2
                TokenKind::Identifier(ref kw) if kw == "tick_priority" => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    tick_priority = match self.cur().kind.clone() {
                        TokenKind::Number(n) => {
                            self.advance();
                            n
                        }
                        TokenKind::Minus => {
                            self.advance();
                            match self.cur().kind.clone() {
                                TokenKind::Number(n) => {
                                    self.advance();
                                    -n
                                }
                                _ => return self.err("Expected number after -"),
                            }
                        }
                        _ => return self.err("tick_priority must be a number"),
                    };
                }
                _ => break,
            }
        }

        Ok(Statement::DotionDef {
            name,
            fields,
            methods,
            handlers,
            brain,
            extends,
            tags,
            tick_priority,
        })
    }

    fn parse_dotion_body(
        &mut self,
    ) -> Result<(Vec<(String, Expr)>, Vec<DotionMethod>, Vec<Handler>), FluxisError> {
        let (mut fields, mut methods, mut handlers) = (Vec::new(), Vec::new(), Vec::new());
        while !matches!(
            self.cur().kind,
            TokenKind::RBrace | TokenKind::EOF | TokenKind::With
        ) {
            match self.cur().kind.clone() {
                TokenKind::On => {
                    self.advance();
                    let msg = match self.cur().kind.clone() {
                        TokenKind::String(s) => {
                            self.advance();
                            s
                        }
                        _ => return self.err("Expected message name string after 'on'"),
                    };
                    let param = if matches!(self.cur().kind, TokenKind::LParen) {
                        self.advance();
                        let p = match self.cur().kind.clone() {
                            TokenKind::RParen => None,
                            TokenKind::Identifier(p) => {
                                self.advance();
                                Some(p)
                            }
                            _ => return self.err("Expected param name or )"),
                        };
                        self.expect(TokenKind::RParen)?;
                        p
                    } else {
                        None
                    };
                    handlers.push(Handler {
                        msg,
                        param,
                        body: self.parse_block()?,
                    });
                }
                TokenKind::Fn => {
                    self.advance();
                    let mname = match self.cur().kind.clone() {
                        TokenKind::Identifier(n) => {
                            self.advance();
                            n
                        }
                        _ => return self.err("Expected method name"),
                    };
                    self.expect(TokenKind::LParen)?;
                    let mut params = Vec::new();
                    while !matches!(self.cur().kind, TokenKind::RParen | TokenKind::EOF) {
                        match self.cur().kind.clone() {
                            TokenKind::Identifier(p) => {
                                params.push(p);
                                self.advance();
                                if matches!(self.cur().kind, TokenKind::Comma) {
                                    self.advance();
                                }
                            }
                            _ => return self.err("Expected param name"),
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    methods.push(DotionMethod {
                        name: mname,
                        params,
                        body: self.parse_block()?,
                    });
                }
                TokenKind::Identifier(fn_) => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    let fv = self.parse_expr()?;
                    fields.push((fn_, fv));
                    if matches!(self.cur().kind, TokenKind::Comma) {
                        self.advance();
                    }
                }
                _ => {
                    return self.err(&format!(
                        "Unexpected token {:?} in dotion body",
                        self.cur().kind
                    ));
                }
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok((fields, methods, handlers))
    }

    fn parse_actor_def(&mut self) -> Result<Statement, FluxisError> {
        self.advance();
        let name = match self.cur().kind.clone() {
            TokenKind::Identifier(n) => {
                self.advance();
                n
            }
            _ => return self.err("Expected actor name"),
        };
        self.expect(TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while !matches!(self.cur().kind, TokenKind::RBrace | TokenKind::EOF) {
            if matches!(self.cur().kind, TokenKind::Fn) {
                self.advance();
                let mn = match self.cur().kind.clone() {
                    TokenKind::Identifier(n) => {
                        self.advance();
                        n
                    }
                    _ => return self.err("Expected method name"),
                };
                self.expect(TokenKind::LParen)?;
                let mut params = Vec::new();
                while !matches!(self.cur().kind, TokenKind::RParen | TokenKind::EOF) {
                    match self.cur().kind.clone() {
                        TokenKind::Identifier(p) => {
                            params.push(p);
                            self.advance();
                            if matches!(self.cur().kind, TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        _ => return self.err("Expected param name"),
                    }
                }
                self.expect(TokenKind::RParen)?;
                methods.push(DotionMethod {
                    name: mn,
                    params,
                    body: self.parse_block()?,
                });
            } else {
                return self.err("Actor body can only contain fn methods");
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Statement::ActorDef { name, methods })
    }

    fn parse_tick(&mut self) -> Result<Statement, FluxisError> {
        self.advance();
        if matches!(self.cur().kind, TokenKind::LBrace) {
            return Ok(Statement::TickBlock {
                body: self.parse_block()?,
            });
        }
        self.expect(TokenKind::LParen)?;
        let count = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::End)?;
        Ok(Statement::TickRun { count })
    }

    fn parse_if(&mut self) -> Result<Statement, FluxisError> {
        self.advance();
        self.expect(TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        let then_ = self.parse_block()?;
        let mut else_ = Vec::new();
        if matches!(self.cur().kind, TokenKind::Else) {
            self.advance();
            if matches!(self.cur().kind, TokenKind::If) {
                else_.push(self.parse_if()?);
            } else {
                else_ = self.parse_block()?;
            }
        }
        Ok(Statement::If {
            condition: cond,
            then_branch: then_,
            else_branch: else_,
        })
    }

    fn parse_struct_def(&mut self) -> Result<Statement, FluxisError> {
        self.advance();
        let name = match self.cur().kind.clone() {
            TokenKind::Identifier(n) => {
                self.advance();
                n
            }
            _ => return self.err("Expected struct name"),
        };
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !matches!(self.cur().kind, TokenKind::RBrace | TokenKind::EOF) {
            match self.cur().kind.clone() {
                TokenKind::Identifier(f) => {
                    fields.push(f);
                    self.advance();
                    if matches!(self.cur().kind, TokenKind::Comma) {
                        self.advance();
                    }
                }
                _ => return self.err("Expected field name"),
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Statement::StructDef { name, fields })
    }

    fn parse_enum_def(&mut self) -> Result<Statement, FluxisError> {
        self.advance();
        let name = match self.cur().kind.clone() {
            TokenKind::Identifier(n) => {
                self.advance();
                n
            }
            _ => return self.err("Expected enum name"),
        };
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while !matches!(self.cur().kind, TokenKind::RBrace | TokenKind::EOF) {
            match self.cur().kind.clone() {
                TokenKind::Identifier(v) => {
                    variants.push(v);
                    self.advance();
                    if matches!(self.cur().kind, TokenKind::Comma) {
                        self.advance();
                    }
                }
                _ => return self.err("Expected variant name"),
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Statement::EnumDef { name, variants })
    }

    fn parse_fn_def(&mut self) -> Result<Statement, FluxisError> {
        self.advance();
        let return_type = if matches!(self.cur().kind, TokenKind::Colon) {
            self.advance();
            match self.try_type() {
                Some(ta) => Some(ta),
                None => return self.err("Expected return type after 'fn:'"),
            }
        } else {
            None
        };
        let name = match self.cur().kind.clone() {
            TokenKind::Identifier(n) => {
                self.advance();
                n
            }
            _ => return self.err("Expected function name"),
        };
        let params = self.parse_params()?;
        Ok(Statement::FunctionDef {
            name,
            params,
            return_type,
            body: self.parse_block()?,
        })
    }

    /// Parse a parameter list (shared by fn defs and closures)
    fn parse_params(&mut self) -> Result<Vec<Param>, FluxisError> {
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !matches!(self.cur().kind, TokenKind::RParen | TokenKind::EOF) {
            // variadic: ...name
            let is_variadic = if matches!(self.cur().kind, TokenKind::DotDot) {
                self.advance();
                true
            } else {
                false
            };
            let pname = match self.cur().kind.clone() {
                TokenKind::Identifier(p) => {
                    self.advance();
                    p
                }
                _ => return self.err("Expected parameter name"),
            };
            let type_ann = if matches!(self.cur().kind, TokenKind::Colon) {
                self.advance();
                match self.try_type() {
                    Some(ta) => Some(ta),
                    None => return self.err("Expected type after ':'"),
                }
            } else {
                None
            };
            // default value: name = expr
            let default = if matches!(self.cur().kind, TokenKind::Assign) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };
            params.push(Param {
                name: pname,
                type_ann,
                default,
                is_variadic,
            });
            if matches!(self.cur().kind, TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(params)
    }

    fn parse_for(&mut self) -> Result<Statement, FluxisError> {
        self.advance(); // consume 'for'
        // for-in: for var in iterable { }
        if let TokenKind::Identifier(var) = self.cur().kind.clone() {
            if matches!(self.peek_kind(1), TokenKind::In) {
                self.advance(); // consume var
                self.advance(); // consume 'in'
                let iterable = self.parse_expr()?;
                let body = self.parse_block()?;
                return Ok(Statement::ForIn {
                    var,
                    iterable,
                    body,
                });
            }
        }
        // C-style: for(init; cond; update) { }
        self.expect(TokenKind::LParen)?;
        let init = self.parse_stmt()?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::End)?;
        let upd = self.parse_stmt()?;
        self.expect(TokenKind::RParen)?;
        Ok(Statement::For {
            init: Box::new(init),
            condition: cond,
            update: Box::new(upd),
            body: self.parse_block()?,
        })
    }

    fn parse_while(&mut self) -> Result<Statement, FluxisError> {
        self.advance();
        self.expect(TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        Ok(Statement::While {
            condition: cond,
            body: self.parse_block()?,
        })
    }

    fn parse_do_while(&mut self) -> Result<Statement, FluxisError> {
        self.advance();
        let body = self.parse_block()?;
        // expect 'while'
        if !matches!(self.cur().kind, TokenKind::While) {
            return self.err("Expected 'while' after do block");
        }
        self.advance();
        self.expect(TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::End)?;
        Ok(Statement::DoWhile {
            body,
            condition: cond,
        })
    }

    fn parse_try_catch(&mut self) -> Result<Statement, FluxisError> {
        self.advance(); // consume 'try'
        let try_body = self.parse_block()?;
        if !matches!(self.cur().kind, TokenKind::Catch) {
            return self.err("Expected 'catch' after try block");
        }
        self.advance();
        self.expect(TokenKind::LParen)?;
        let catch_var = match self.cur().kind.clone() {
            TokenKind::Identifier(n) => {
                self.advance();
                n
            }
            _ => return self.err("Expected error variable name in catch()"),
        };
        self.expect(TokenKind::RParen)?;
        let catch_body = self.parse_block()?;
        Ok(Statement::TryCatch {
            try_body,
            catch_var,
            catch_body,
        })
    }

    fn parse_async_fn(&mut self) -> Result<Statement, FluxisError> {
        self.advance(); // consume 'async'
        if !matches!(self.cur().kind, TokenKind::Fn) {
            return self.err("Expected 'fn' after 'async'");
        }
        // Parse as regular fn — async is just a marker for now
        self.parse_fn_def()
    }

    // ── EXPRESSIONS ──────────────────────────────────────────────────
    fn parse_expr(&mut self) -> Result<Expr, FluxisError> {
        self.parse_null_coalesce()
    }

    /// ?? null coalesce — lowest precedence
    fn parse_null_coalesce(&mut self) -> Result<Expr, FluxisError> {
        let mut l = self.parse_or()?;
        while matches!(self.cur().kind, TokenKind::QuestionQuestion) {
            self.advance();
            let r = self.parse_or()?;
            l = Expr::NullCoalesce {
                left: Box::new(l),
                right: Box::new(r),
            };
        }
        Ok(l)
    }

    fn parse_or(&mut self) -> Result<Expr, FluxisError> {
        let mut l = self.parse_and()?;
        while matches!(self.cur().kind, TokenKind::Or) {
            self.advance();
            let r = self.parse_and()?;
            l = Expr::Binary {
                left: Box::new(l),
                op: "||".into(),
                right: Box::new(r),
            };
        }
        Ok(l)
    }
    fn parse_and(&mut self) -> Result<Expr, FluxisError> {
        let mut l = self.parse_eq()?;
        while matches!(self.cur().kind, TokenKind::And) {
            self.advance();
            let r = self.parse_eq()?;
            l = Expr::Binary {
                left: Box::new(l),
                op: "&&".into(),
                right: Box::new(r),
            };
        }
        Ok(l)
    }
    fn parse_eq(&mut self) -> Result<Expr, FluxisError> {
        let mut l = self.parse_cmp()?;
        while matches!(self.cur().kind, TokenKind::EqualEqual | TokenKind::NotEqual) {
            let op = match self.cur().kind {
                TokenKind::EqualEqual => "==",
                TokenKind::NotEqual => "!=",
                _ => unreachable!(),
            }
            .to_string();
            self.advance();
            let r = self.parse_cmp()?;
            l = Expr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            };
        }
        Ok(l)
    }
    fn parse_cmp(&mut self) -> Result<Expr, FluxisError> {
        let mut l = self.parse_in()?;
        while matches!(
            self.cur().kind,
            TokenKind::Greater | TokenKind::Less | TokenKind::GreaterEqual | TokenKind::LessEqual
        ) {
            let op = match self.cur().kind {
                TokenKind::Greater => ">",
                TokenKind::Less => "<",
                TokenKind::GreaterEqual => ">=",
                TokenKind::LessEqual => "<=",
                _ => unreachable!(),
            }
            .to_string();
            self.advance();
            let r = self.parse_in()?;
            l = Expr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            };
        }
        Ok(l)
    }
    /// `x in arr` and `x not in arr`
    fn parse_in(&mut self) -> Result<Expr, FluxisError> {
        let l = self.parse_term()?;
        if matches!(self.cur().kind, TokenKind::In | TokenKind::NotIn) {
            let negated = matches!(self.cur().kind, TokenKind::NotIn);
            self.advance();
            let collection = self.parse_term()?;
            return Ok(Expr::In {
                value: Box::new(l),
                collection: Box::new(collection),
                negated,
            });
        }
        Ok(l)
    }
    fn parse_term(&mut self) -> Result<Expr, FluxisError> {
        let mut l = self.parse_factor()?;
        while matches!(self.cur().kind, TokenKind::Plus | TokenKind::Minus) {
            let op = match self.cur().kind {
                TokenKind::Plus => "+",
                TokenKind::Minus => "-",
                _ => unreachable!(),
            }
            .to_string();
            self.advance();
            let r = self.parse_factor()?;
            l = Expr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            };
        }
        Ok(l)
    }
    fn parse_factor(&mut self) -> Result<Expr, FluxisError> {
        let mut l = self.parse_unary()?;
        while matches!(
            self.cur().kind,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent
        ) {
            let op = match self.cur().kind {
                TokenKind::Star => "*",
                TokenKind::Slash => "/",
                TokenKind::Percent => "%",
                _ => unreachable!(),
            }
            .to_string();
            self.advance();
            let r = self.parse_unary()?;
            l = Expr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            };
        }
        Ok(l)
    }
    fn parse_unary(&mut self) -> Result<Expr, FluxisError> {
        if matches!(self.cur().kind, TokenKind::Not) {
            self.advance();
            let e = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: "!".into(),
                expr: Box::new(e),
            });
        }
        if matches!(self.cur().kind, TokenKind::Minus) {
            self.advance();
            let e = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: "-".into(),
                expr: Box::new(e),
            });
        }
        if matches!(self.cur().kind, TokenKind::Await) {
            self.advance();
            let e = self.parse_unary()?;
            return Ok(Expr::Await(Box::new(e)));
        }
        let e = self.parse_postfix()?;
        // Range: expr..expr or expr..expr..step
        if matches!(self.cur().kind, TokenKind::DotDot) {
            self.advance();
            let end = self.parse_postfix()?;
            let step = if matches!(self.cur().kind, TokenKind::DotDot) {
                self.advance();
                Some(Box::new(self.parse_postfix()?))
            } else {
                None
            };
            return Ok(Expr::Range {
                start: Box::new(e),
                end: Box::new(end),
                step,
            });
        }
        Ok(e)
    }

    fn parse_postfix(&mut self) -> Result<Expr, FluxisError> {
        let mut e = self.parse_primary()?;
        loop {
            match self.cur().kind.clone() {
                TokenKind::LBracket => {
                    self.advance();
                    let i = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    e = Expr::Index {
                        object: Box::new(e),
                        index: Box::new(i),
                    };
                }
                TokenKind::Dot => {
                    self.advance();
                    let member_name = match self.cur().kind.clone() {
                        TokenKind::Identifier(f) => f,
                        TokenKind::Tick => "tick".to_string(),
                        TokenKind::On => "on".to_string(),
                        TokenKind::Self_ => "self".to_string(),
                        TokenKind::With => "with".to_string(),
                        _ => return self.err("Expected field or method name after '.'"),
                    };
                    self.advance();
                    if matches!(self.cur().kind, TokenKind::LParen) {
                        self.advance();
                        let mut args = Vec::new();
                        while !matches!(self.cur().kind, TokenKind::RParen | TokenKind::EOF) {
                            args.push(self.parse_expr()?);
                            if matches!(self.cur().kind, TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::RParen)?;
                        e = Expr::MethodCall {
                            object: Box::new(e),
                            method: member_name,
                            args,
                        };
                    } else {
                        e = Expr::Field {
                            object: Box::new(e),
                            field: member_name,
                        };
                    }
                }
                // Optional chain: ?.field
                TokenKind::QuestionDot => {
                    self.advance();
                    let field = match self.cur().kind.clone() {
                        TokenKind::Identifier(f) => {
                            self.advance();
                            f
                        }
                        _ => return self.err("Expected field name after '?.'"),
                    };
                    e = Expr::OptionalChain {
                        object: Box::new(e),
                        field,
                    };
                }
                // Closure call: my_fn(args)
                TokenKind::LParen if !matches!(e, Expr::Identifier(_) | Expr::Call { .. }) => {
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.cur().kind, TokenKind::RParen | TokenKind::EOF) {
                        args.push(self.parse_expr()?);
                        if matches!(self.cur().kind, TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    e = Expr::CallExpr {
                        callee: Box::new(e),
                        args,
                    };
                }
                _ => break,
            }
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> Result<Expr, FluxisError> {
        match self.cur().kind.clone() {
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut els = Vec::new();
                while !matches!(self.cur().kind, TokenKind::RBracket | TokenKind::EOF) {
                    els.push(self.parse_expr()?);
                    if matches!(self.cur().kind, TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::Array(els))
            }
            TokenKind::LBrace => {
                self.advance();
                let mut pairs = Vec::new();
                while !matches!(self.cur().kind, TokenKind::RBrace | TokenKind::EOF) {
                    let k = self.parse_expr()?;
                    self.expect(TokenKind::Colon)?;
                    let v = self.parse_expr()?;
                    pairs.push((k, v));
                    if matches!(self.cur().kind, TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBrace)?;
                Ok(Expr::Map(pairs))
            }
            TokenKind::Nil => {
                self.advance();
                Ok(Expr::Nil)
            }
            TokenKind::Self_ => {
                self.advance();
                Ok(Expr::Self_)
            }

            // Closure literal: fn(params) { body }
            TokenKind::Fn => {
                // Only a closure if followed by ( without a name
                if matches!(self.peek_kind(1), TokenKind::LParen) {
                    self.advance(); // consume 'fn'
                    let params = self.parse_params()?;
                    let body = self.parse_block()?;
                    return Ok(Expr::Closure { params, body });
                }
                // Otherwise it's a statement-level fn def parsed elsewhere
                self.err("Unexpected 'fn' in expression position")
            }

            // in() — user input
            TokenKind::In => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::Input)
            }

            // dotion literal
            TokenKind::Dotion => {
                self.advance();
                if let TokenKind::Identifier(_) = self.cur().kind.clone() {
                    if matches!(self.peek_kind(1), TokenKind::LBrace) {
                        let type_name = match self.cur().kind.clone() {
                            TokenKind::Identifier(n) => {
                                self.advance();
                                n
                            }
                            _ => unreachable!(),
                        };
                        self.advance();
                        let mut overrides = Vec::new();
                        while !matches!(self.cur().kind, TokenKind::RBrace | TokenKind::EOF) {
                            let fn_ = match self.cur().kind.clone() {
                                TokenKind::Identifier(f) => {
                                    self.advance();
                                    f
                                }
                                _ => return self.err("Expected field name"),
                            };
                            self.expect(TokenKind::Colon)?;
                            let fv = self.parse_expr()?;
                            overrides.push((fn_, fv));
                            if matches!(self.cur().kind, TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::RBrace)?;
                        return Ok(Expr::Call {
                            name: format!("__dotion__{}", type_name),
                            args: overrides.into_iter().map(|(_, v)| v).collect(),
                        });
                    }
                }
                self.expect(TokenKind::LBrace)?;
                let (fields, methods, handlers) = self.parse_dotion_body()?;
                Ok(Expr::DotionLit {
                    fields,
                    methods,
                    handlers,
                })
            }

            TokenKind::Identifier(name) => {
                if name == "true" {
                    self.advance();
                    return Ok(Expr::Bool(true));
                }
                if name == "false" {
                    self.advance();
                    return Ok(Expr::Bool(false));
                }
                if matches!(self.peek_kind(1), TokenKind::ColonColon) {
                    let en = name;
                    self.advance();
                    self.advance();
                    match self.cur().kind.clone() {
                        TokenKind::Identifier(v) => {
                            self.advance();
                            return Ok(Expr::EnumVariant {
                                enum_name: en,
                                variant: v,
                            });
                        }
                        _ => return self.err("Expected variant after '::'"),
                    }
                }
                // Struct init: Name { field: val } or Name {}
                let is_struct_init = (matches!(self.peek_kind(1), TokenKind::LBrace)
                    && matches!(self.peek_kind(2), TokenKind::Identifier(_))
                    && matches!(self.peek_kind(3), TokenKind::Colon))
                    || (matches!(self.peek_kind(1), TokenKind::LBrace)
                        && matches!(self.peek_kind(2), TokenKind::RBrace));
                if is_struct_init {
                    let sn = name;
                    self.advance();
                    self.advance();
                    let mut fs = Vec::new();
                    while !matches!(self.cur().kind, TokenKind::RBrace | TokenKind::EOF) {
                        let fn_ = match self.cur().kind.clone() {
                            TokenKind::Identifier(f) => {
                                self.advance();
                                f
                            }
                            _ => return self.err("Expected field name"),
                        };
                        self.expect(TokenKind::Colon)?;
                        let fv = self.parse_expr()?;
                        fs.push((fn_, fv));
                        if matches!(self.cur().kind, TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RBrace)?;
                    return Ok(Expr::StructInit {
                        name: sn,
                        fields: fs,
                    });
                }
                // Function call
                if matches!(self.peek_kind(1), TokenKind::LParen) {
                    self.advance();
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.cur().kind, TokenKind::RParen | TokenKind::EOF) {
                        args.push(self.parse_expr()?);
                        if matches!(self.cur().kind, TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    return Ok(Expr::Call { name, args });
                }
                self.advance();
                Ok(Expr::Identifier(name))
            }

            // String — check for interpolation {varname}
            TokenKind::String(v) => {
                self.advance();
                // Parse interpolation: find {expr} segments
                let segments = Self::parse_interp(&v);
                if segments.len() == 1 && segments[0].1.is_none() {
                    Ok(Expr::String(v))
                } else {
                    Ok(Expr::InterpolatedStr(segments))
                }
            }
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            TokenKind::Float(f) => {
                self.advance();
                Ok(Expr::Float(f))
            }
            _ => self.err(&format!("Unexpected token {:?}", self.cur().kind)),
        }
    }

    /// Parse a string into interpolation segments.
    /// "Hello {name}, you are {age}!" →
    ///   [("Hello ", Some(Identifier("name"))), (", you are ", Some(Identifier("age"))), ("!", None)]
    fn parse_interp(s: &str) -> Vec<(String, Option<Box<Expr>>)> {
        let mut segments = Vec::new();
        let mut literal = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' {
                // Check for {{ escape
                if chars.peek() == Some(&'{') {
                    chars.next();
                    literal.push('{');
                    continue;
                }
                // Collect expr until }
                let mut expr_src = String::new();
                let mut depth = 1;
                for ch in chars.by_ref() {
                    if ch == '{' {
                        depth += 1;
                        expr_src.push(ch);
                    } else if ch == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        expr_src.push(ch);
                    } else {
                        expr_src.push(ch);
                    }
                }
                let expr_src = expr_src.trim().to_string();
                if expr_src.is_empty() {
                    literal.push('{');
                    literal.push('}');
                } else {
                    // Try to parse the inner expression
                    let inner_expr = Self::parse_interp_expr(&expr_src);
                    segments.push((std::mem::take(&mut literal), Some(Box::new(inner_expr))));
                }
            } else if c == '}' && chars.peek() == Some(&'}') {
                chars.next();
                literal.push('}');
            } else {
                literal.push(c);
            }
        }
        if !literal.is_empty() || segments.is_empty() {
            segments.push((literal, None));
        }
        segments
    }

    fn parse_interp_expr(src: &str) -> Expr {
        // Simple: if it's an identifier, emit Identifier; if it's a.b, emit Field
        // For more complex cases, fallback to Call(format, [Identifier])
        let trimmed = src.trim();
        if trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
            Expr::Identifier(trimmed.to_string())
        } else if let Some(dot_pos) = trimmed.find('.') {
            let obj = trimmed[..dot_pos].trim().to_string();
            let field = trimmed[dot_pos + 1..].trim().to_string();
            Expr::Field {
                object: Box::new(Expr::Identifier(obj)),
                field,
            }
        } else {
            // Treat as identifier fallback
            Expr::Identifier(trimmed.to_string())
        }
    }
}

#![allow(dead_code)]
// FLUXIS — error.rs
// Unified error type with multi-error collection support.

const RED:     &str = "\x1b[31m";
const YELLOW:  &str = "\x1b[33m";
const CYAN:    &str = "\x1b[36m";
const BOLD:    &str = "\x1b[1m";
const DIM:     &str = "\x1b[2m";
const RESET:   &str = "\x1b[0m";
const MAGENTA: &str = "\x1b[35m";
const GREEN:   &str = "\x1b[32m";
const WHITE:   &str = "\x1b[97m";

#[derive(Debug, Clone)]
pub enum ErrorKind {
    LexError,
    ParseError,
    RuntimeError,
    TypeError,
    ScopeError,
    ArityError,
    CompileError,
}

#[derive(Debug, Clone)]
pub struct FluxisError {
    pub kind:    ErrorKind,
    pub message: String,
    pub line:    usize,
    pub col:     usize,
    pub source:  Option<String>,
    pub hint:    Option<String>,
}

impl FluxisError {
    pub fn new(kind: ErrorKind, message: &str, line: usize, col: usize) -> Self {
        Self { kind, message: message.to_string(), line, col, source: None, hint: None }
    }

    pub fn with_source(mut self, src: &str) -> Self {
        self.source = Some(src.to_string()); self
    }

    pub fn with_hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_string()); self
    }

    pub fn with_line(mut self, line: usize, col: usize) -> Self {
        if self.line == 0 { self.line = line; self.col = col; }
        self
    }

    pub fn display(&self) {
        let (label, color) = match self.kind {
            ErrorKind::LexError     => ("Lex Error",     RED),
            ErrorKind::ParseError   => ("Parse Error",   RED),
            ErrorKind::RuntimeError => ("Runtime Error", RED),
            ErrorKind::TypeError    => ("Type Error",    MAGENTA),
            ErrorKind::ScopeError   => ("Scope Error",   YELLOW),
            ErrorKind::ArityError   => ("Arity Error",   CYAN),
            ErrorKind::CompileError => ("Compile Error", RED),
        };

        eprintln!();
        eprintln!("{}{}┌─ {} ──────────────────────────{}", BOLD, color, label, RESET);
        eprintln!("{}{}│{} {}{}{}", BOLD, color, RESET, BOLD, WHITE, RESET);
        eprintln!("{}{}│{}   {}", BOLD, color, RESET, self.message);
        eprintln!("{}{}│{}", BOLD, color, RESET);

        if self.line > 0 {
            eprintln!("{}{}│{}{}   {}→{}  line {}{}{}, col {}",
                BOLD, color, RESET, DIM, RESET,
                BOLD, CYAN, self.line, RESET,
                self.col);
        }

        if let Some(ref src) = self.source {
            let ln = self.line.to_string();
            let pad = " ".repeat(ln.len());
            eprintln!("{}{}│{}", BOLD, color, RESET);
            eprintln!("{}{}│{}  {} │ {}{}{}",
                BOLD, color, RESET, ln, WHITE, src.trim_end(), RESET);
            let caret_col = if self.col > 0 { self.col.saturating_sub(1) } else { 0 };
            let spaces = " ".repeat(caret_col);
            eprintln!("{}{}│{}  {} │ {}{}{}^── here{}",
                BOLD, color, RESET, pad, BOLD, color, spaces, RESET);
        }

        if let Some(ref hint) = self.hint {
            eprintln!("{}{}│{}", BOLD, color, RESET);
            eprintln!("{}{}│{}  {}{}💡 Hint:{} {}{}{}",
                BOLD, color, RESET, BOLD, GREEN, RESET, DIM, hint, RESET);
        }

        eprintln!("{}{}└────────────────────────────────────{}", BOLD, color, RESET);
        eprintln!();
    }
}

// ── MULTI-ERROR COLLECTOR ─────────────────────────────────────────────────
/// Collects multiple errors from a full-program analysis pass.
/// Errors are accumulated rather than stopping at the first one.
pub struct ErrorCollector {
    pub errors: Vec<FluxisError>,
    pub source_lines: Vec<String>,
}

impl ErrorCollector {
    pub fn new(source: &str) -> Self {
        Self {
            errors: Vec::new(),
            source_lines: source.lines().map(|l| l.to_string()).collect(),
        }
    }

    pub fn add(&mut self, mut err: FluxisError) {
        // Attach source line snippet if we have line info
        if err.line > 0 && err.source.is_none() {
            if let Some(line) = self.source_lines.get(err.line.saturating_sub(1)) {
                err.source = Some(line.clone());
            }
        }
        self.errors.push(err);
    }

    pub fn has_errors(&self) -> bool { !self.errors.is_empty() }

    pub fn display_all(&self, filename: &str) {
        if self.errors.is_empty() { return; }
        let count = self.errors.len();
        eprintln!();
        eprintln!("{}[{}]{} Found {} error{}:",
            "\x1b[1m\x1b[36m", filename, "\x1b[0m",
            count, if count == 1 { "" } else { "s" });
        for (i, err) in self.errors.iter().enumerate() {
            eprint!("  [{}{}{}] ", "\x1b[1m", i+1, "\x1b[0m");
            err.display();
        }
    }

    pub fn into_errors(self) -> Vec<FluxisError> { self.errors }
}

// ── ERROR CONSTRUCTORS ────────────────────────────────────────────────────
pub fn lex_error    (msg: &str, line: usize, col: usize) -> FluxisError { FluxisError::new(ErrorKind::LexError,     msg, line, col) }
pub fn parse_error  (msg: &str, line: usize, col: usize) -> FluxisError { FluxisError::new(ErrorKind::ParseError,   msg, line, col) }
pub fn runtime_error(msg: &str)                          -> FluxisError { FluxisError::new(ErrorKind::RuntimeError, msg, 0,    0)   }
pub fn type_error   (msg: &str)                          -> FluxisError { FluxisError::new(ErrorKind::TypeError,    msg, 0,    0)   }
pub fn scope_error  (msg: &str)                          -> FluxisError { FluxisError::new(ErrorKind::ScopeError,   msg, 0,    0)   }
pub fn compile_error(msg: &str, line: usize, col: usize) -> FluxisError { FluxisError::new(ErrorKind::CompileError, msg, line, col) }
pub fn arity_error  (name: &str, expected: usize, got: usize) -> FluxisError {
    FluxisError::new(ErrorKind::ArityError,
        &format!("'{}' expects {} argument(s) but got {}", name, expected, got), 0, 0)
}


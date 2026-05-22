#![allow(dead_code)]
// FLUXIS — ast.rs (v10.0)
// New: Closure, InterpolatedStr, Spread, TryCatch, Await, Range, OptionalChain, NullCoalesce

#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Num,
    Float,
    Str,
    Bool,
    Array,
    Map,
    Any,
}
impl TypeAnnotation {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "num" => Some(Self::Num),
            "float" => Some(Self::Float),
            "str" => Some(Self::Str),
            "bool" => Some(Self::Bool),
            "array" => Some(Self::Array),
            "map" => Some(Self::Map),
            "any" => Some(Self::Any),
            _ => None,
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Num => "num",
            Self::Float => "float",
            Self::Str => "str",
            Self::Bool => "bool",
            Self::Array => "array",
            Self::Map => "map",
            Self::Any => "any",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Handler {
    pub msg: String,
    pub param: Option<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct DotionMethod {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum MatchPattern {
    Wildcard,
    Literal(Expr),
    EnumVariant {
        enum_name: String,
        variant: String,
    },
    /// Struct destructure: Point{x, y} — binds x and y
    Struct {
        type_name: String,
        bindings: Vec<String>,
    },
}

/// A closure parameter: (name, optional default value)
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub default: Option<Expr>,
    pub is_variadic: bool,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    Float(f64),
    Identifier(String),
    String(String),
    Bool(bool),
    Nil,
    Input,

    /// String interpolation: `"Hello {name}, you are {age} years old!"`
    /// Stored as Vec of (literal_text, optional_expr_to_interpolate)
    InterpolatedStr(Vec<(String, Option<Box<Expr>>)>),

    /// First-class function / closure:  fn(a, b) { return a + b; }
    Closure {
        params: Vec<Param>,
        body: Vec<Statement>,
    },

    /// Range:  0..10  or  0..10..2  (start..end or start..end..step)
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        step: Option<Box<Expr>>,
    },

    /// Optional chain:  user?.name  →  nil if user is nil
    OptionalChain {
        object: Box<Expr>,
        field: String,
    },

    /// Null coalesce:  x ?? default_val
    NullCoalesce {
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Await expression:  await ai_ask("prompt")
    Await(Box<Expr>),

    Binary {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Unary {
        op: String,
        expr: Box<Expr>,
    },

    /// Regular call — name can be a string (builtin) or a closure variable
    Call {
        name: String,
        args: Vec<Expr>,
    },

    /// Call a closure stored in a variable:  my_fn(1, 2)
    CallExpr {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },

    Array(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        object: Box<Expr>,
        field: String,
    },
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
    },
    DotionLit {
        fields: Vec<(String, Expr)>,
        methods: Vec<DotionMethod>,
        handlers: Vec<Handler>,
    },
    Self_,
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },

    /// `in` operator:  "apple" in fruits
    In {
        value: Box<Expr>,
        collection: Box<Expr>,
        negated: bool,
    },
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assignment {
        name: String,
        type_annotation: Option<TypeAnnotation>,
        value: Expr,
    },
    CompoundAssign {
        name: String,
        op: String,
        value: Expr,
    },
    SelfCompoundAssign {
        field: String,
        op: String,
        value: Expr,
    },
    Print {
        value: Expr,
    },
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
    If {
        condition: Expr,
        then_branch: Vec<Statement>,
        else_branch: Vec<Statement>,
    },
    For {
        init: Box<Statement>,
        condition: Expr,
        update: Box<Statement>,
        body: Vec<Statement>,
    },
    ForIn {
        var: String,
        iterable: Expr,
        body: Vec<Statement>,
    },
    DoWhile {
        body: Vec<Statement>,
        condition: Expr,
    },

    /// Function def now supports default params and variadics
    FunctionDef {
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Vec<Statement>,
    },

    Return {
        value: Expr,
    },
    StructDef {
        name: String,
        fields: Vec<String>,
    },
    EnumDef {
        name: String,
        variants: Vec<String>,
    },
    IndexAssign {
        object: String,
        index: Expr,
        value: Expr,
    },
    FieldAssign {
        object: String,
        field: String,
        value: Expr,
    },
    SelfFieldAssign {
        field: String,
        value: Expr,
    },
    Break,
    Continue,
    Increment {
        name: String,
    },
    Decrement {
        name: String,
    },

    Match {
        value: Expr,
        arms: Vec<MatchArm>,
    },

    /// try { } catch(e) { }
    TryCatch {
        try_body: Vec<Statement>,
        catch_var: String,
        catch_body: Vec<Statement>,
    },

    DotionDef {
        name: String,
        fields: Vec<(String, Expr)>,
        methods: Vec<DotionMethod>,
        handlers: Vec<Handler>,
        brain: Option<String>,
        extends: Option<String>,
        tags: Vec<String>,
        tick_priority: i64,
    },
    ActorDef {
        name: String,
        methods: Vec<DotionMethod>,
    },
    TickBlock {
        body: Vec<Statement>,
    },
    TickRun {
        count: Expr,
    },
    MethodCallStmt {
        object: String,
        method: String,
        args: Vec<Expr>,
    },
    Import {
        module: String,
    },
}

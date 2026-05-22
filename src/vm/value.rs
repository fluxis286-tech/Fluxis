// FLUXIS — vm/value.rs
// The Value type: every runtime value in FLUXIS is one of these variants.

use std::collections::HashMap;
use crate::ast::{Handler, DotionMethod};

#[derive(Clone, Debug)]
pub enum Value {
    Number(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Struct  { name: String, fields: HashMap<String, Value> },
    EnumVariant { enum_name: String, variant: String },
    Dotion {
        id:            u64,
        name:          String,
        fields:        HashMap<String, Value>,
        methods:       Vec<DotionMethod>,
        handlers:      Vec<Handler>,
        mailbox:       Vec<(String, Value)>,
        brain:         Option<String>,
        tags:          Vec<String>,
        tick_priority: i64,
    },
}

impl Value {
    /// Human-readable type name used in error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_)      => "num",
            Value::Float(_)       => "float",
            Value::Str(_)         => "str",
            Value::Bool(_)        => "bool",
            Value::Nil            => "nil",
            Value::Array(_)       => "array",
            Value::Map(_)         => "map",
            Value::Struct { .. }  => "struct",
            Value::EnumVariant { .. } => "enum",
            Value::Dotion { .. }  => "dotion",
        }
    }

    /// Convert any value to its string representation.
    pub fn display(&self) -> String {
        match self {
            Value::Number(n)  => n.to_string(),
            Value::Float(f)   => {
                let s = format!("{:.6}", f);
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            }
            Value::Str(s)     => s.clone(),
            Value::Bool(b)    => b.to_string(),
            Value::Nil        => "nil".to_string(),
            Value::Array(a)   => format!("[{}]", a.iter().map(|v| v.display()).collect::<Vec<_>>().join(", ")),
            Value::Map(m)     => format!("{{{}}}", m.iter().map(|(k, v)| format!("{}: {}", k, v.display())).collect::<Vec<_>>().join(", ")),
            Value::Struct { name, fields } =>
                format!("{} {{{}}}", name, fields.iter().map(|(k, v)| format!("{}: {}", k, v.display())).collect::<Vec<_>>().join(", ")),
            Value::EnumVariant { enum_name, variant } =>
                format!("{}::{}", enum_name, variant),
            Value::Dotion { name, fields, .. } =>
                format!("dotion({}) {{{}}}", name, fields.iter().map(|(k, v)| format!("{}: {}", k, v.display())).collect::<Vec<_>>().join(", ")),
        }
    }

    /// Truthiness — used for if/while conditions.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b)    => *b,
            Value::Number(n)  => *n != 0,
            Value::Float(f)   => *f != 0.0,
            Value::Str(s)     => !s.is_empty(),
            Value::Nil        => false,
            Value::Array(a)   => !a.is_empty(),
            Value::Map(m)     => !m.is_empty(),
            _                 => true,
        }
    }

    /// Equality comparison that handles cross-numeric types.
    pub fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b))   => a == b,
            (Value::Float(a),  Value::Float(b))    => (a - b).abs() < 1e-12,
            (Value::Number(a), Value::Float(b))    => (*a as f64 - b).abs() < 1e-12,
            (Value::Float(a),  Value::Number(b))   => (a - *b as f64).abs() < 1e-12,
            (Value::Str(a),    Value::Str(b))      => a == b,
            (Value::Bool(a),   Value::Bool(b))     => a == b,
            (Value::Nil,       Value::Nil)         => true,
            (Value::EnumVariant { enum_name: e1, variant: v1 },
             Value::EnumVariant { enum_name: e2, variant: v2 }) => e1 == e2 && v1 == v2,
            _ => self.display() == other.display(),
        }
    }
}


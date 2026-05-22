#![allow(dead_code)]
// FLUXIS — stdlib/mod.rs

pub mod math;
pub mod string;
pub mod io;
pub mod ml;
pub mod gfx;
pub mod ai;

pub fn load_module(name: &str) -> Option<&'static str> {
    match name {
        "math"   => Some("math"),
        "string" => Some("string"),
        "io"     => Some("io"),
        "ai"     => Some("ai"),
        "ml"     => Some("ml"),
        "gfx"    => Some("gfx"),
        _        => None,
    }
}

pub fn is_math_fn(name: &str) -> bool   { math::is_math_fn(name) }
pub fn is_string_fn(name: &str) -> bool { string::is_string_fn(name) }
pub fn is_io_fn(name: &str) -> bool     { io::is_io_fn(name) }
pub fn is_ml_fn(name: &str) -> bool     { ml::is_ml_fn(name) }
pub fn is_gfx_fn(name: &str) -> bool    { gfx::is_gfx_fn(name) }
pub fn is_ai_fn(name: &str) -> bool     { ai::is_ai_fn(name) }


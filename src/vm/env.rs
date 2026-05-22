#![allow(dead_code)]
// FLUXIS — vm/env.rs
// Environment: variable storage and lexical scope chain.
// The tree-walking VM uses Env directly.
// The bytecode VM uses its own scope Vec but mirrors this design.

use std::collections::HashMap;
use crate::vm::value::Value;

/// A lexical scope chain.
/// Each frame is a HashMap of name → Value.
/// Variable lookup walks frames from innermost to outermost.
pub struct Env {
    pub frames: Vec<HashMap<String, Value>>,
}

impl Env {
    pub fn new() -> Self {
        Self { frames: vec![HashMap::new()] }
    }

    /// Open a new inner scope (entering a block, function, etc.).
    pub fn push_scope(&mut self) {
        self.frames.push(HashMap::new());
    }

    /// Close the innermost scope.
    pub fn pop_scope(&mut self) {
        if self.frames.len() > 1 { self.frames.pop(); }
    }

    /// Look up a variable, walking outward through scopes.
    pub fn get(&self, name: &str) -> Option<Value> {
        for frame in self.frames.iter().rev() {
            if let Some(v) = frame.get(name) { return Some(v.clone()); }
        }
        None
    }

    /// Update an existing binding, walking outward.
    /// If not found, creates in the current (innermost) scope.
    pub fn set(&mut self, name: &str, val: Value) {
        for frame in self.frames.iter_mut().rev() {
            if frame.contains_key(name) {
                frame.insert(name.to_string(), val);
                return;
            }
        }
        self.define(name, val);
    }

    /// Create a new binding in the current (innermost) scope.
    pub fn define(&mut self, name: &str, val: Value) {
        if let Some(f) = self.frames.last_mut() {
            f.insert(name.to_string(), val);
        }
    }

    /// Mutable access to an existing binding (for in-place mutation).
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        for frame in self.frames.iter_mut().rev() {
            if frame.contains_key(name) { return frame.get_mut(name); }
        }
        None
    }

    /// All variable names currently visible (for DOP dotion scanning).
    pub fn all_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for frame in &self.frames {
            for k in frame.keys() {
                if !names.contains(k) { names.push(k.clone()); }
            }
        }
        names
    }
}

/// Bytecode call frame — saved state for a function call.
#[allow(dead_code)]
pub struct CallFrame {
    /// Index into the chunk's instruction list to return to.
    pub return_ip: usize,
    /// Saved operand stack depth before this call.
    pub stack_base: usize,
}

/// Bytecode scope stack — mirrors Env but lives separately
/// because the bytecode VM manages its own execution frames.
#[derive(Clone)]
pub struct BvmEnv {
    pub frames: Vec<HashMap<String, Value>>,
}

impl BvmEnv {
    pub fn new() -> Self { Self { frames: vec![HashMap::new()] } }

    pub fn push(&mut self) { self.frames.push(HashMap::new()); }
    pub fn pop(&mut self)  { if self.frames.len() > 1 { self.frames.pop(); } }

    pub fn get(&self, name: &str) -> Option<Value> {
        for f in self.frames.iter().rev() {
            if let Some(v) = f.get(name) { return Some(v.clone()); }
        }
        None
    }

    pub fn set(&mut self, name: &str, val: Value) {
        for f in self.frames.iter_mut().rev() {
            if f.contains_key(name) { f.insert(name.to_string(), val); return; }
        }
        if let Some(f) = self.frames.last_mut() { f.insert(name.to_string(), val); }
    }
}


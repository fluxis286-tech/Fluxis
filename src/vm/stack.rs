#![allow(dead_code)]
// FLUXIS — vm/stack.rs
// Operand stack for the bytecode VM.
// All push/pop goes through here — no raw Vec access outside this module.

use crate::error::{FluxisError, runtime_error};
use crate::vm::value::Value;

pub struct Stack {
    pub data: Vec<Value>,
}

impl Stack {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Push a value onto the stack.
    pub fn push(&mut self, val: Value) {
        self.data.push(val);
    }

    /// Pop from the stack, returning an error on underflow.
    pub fn pop(&mut self) -> Result<Value, FluxisError> {
        self.data
            .pop()
            .ok_or_else(|| runtime_error("VM stack underflow"))
    }

    /// Peek at the top without consuming it.
    pub fn peek(&self) -> Result<&Value, FluxisError> {
        self.data
            .last()
            .ok_or_else(|| runtime_error("VM stack is empty"))
    }

    /// Pop N values in the order they were pushed (oldest first).
    /// Used for building arrays, maps, and function argument lists.
    pub fn pop_n(&mut self, n: usize) -> Result<Vec<Value>, FluxisError> {
        if self.data.len() < n {
            return Err(runtime_error(&format!(
                "VM stack underflow: need {} values, have {}",
                n,
                self.data.len()
            )));
        }
        let start = self.data.len() - n;
        Ok(self.data.drain(start..).collect())
    }

    /// Current depth — used for call frame saving.
    pub fn depth(&self) -> usize {
        self.data.len()
    }

    /// Truncate to a saved depth (for unwinding on error or return).
    pub fn truncate(&mut self, depth: usize) {
        self.data.truncate(depth);
    }

    /// Consume the stack entirely, returning all values (for debug/REPL).
    pub fn drain_all(&mut self) -> Vec<Value> {
        self.data.drain(..).collect()
    }
}

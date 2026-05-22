#![allow(unused_imports)]
// FLUXIS — vm/runtime.rs
// Runtime glue: function calls, builtin dispatch, DOP bridging.
// The Core execution loop calls into here for anything that isn't
// a pure stack/arithmetic operation.

use crate::error::{FluxisError, runtime_error, type_error};
use crate::stdlib;
use crate::vm::core::Core;
use crate::vm::opcodes::Chunk;
use crate::vm::value::Value;
use std::collections::HashMap;

/// The Runtime holds all compiled function chunks and routes
/// builtin/DOP calls. It is the bridge between Core and the outside world.
pub struct Runtime {
    /// Compiled function bodies indexed by name.
    pub fn_chunks: HashMap<String, Chunk>,
    /// DOP: global tick count
    pub tick_count: u64,
    /// DOP: current __self__ dotion variable name (set during method/handler calls)
    pub self_var: Option<String>,
    /// DOP: mailboxes — var_name → vec of (msg, arg) pending delivery
    pub mailboxes: HashMap<String, Vec<(String, Value)>>,
    /// DOP: tick block function name if registered
    pub tick_block: Option<String>,
    /// DOP: global dotion registry — always visible regardless of call frame
    pub dotions: HashMap<String, Value>,
    /// Import tracking — prevent double-loading
    pub loaded_modules: std::collections::HashSet<String>,
}

impl Runtime {
    pub fn new(fn_chunks: HashMap<String, Chunk>) -> Self {
        Self {
            fn_chunks,
            tick_count: 0,
            self_var: None,
            mailboxes: HashMap::new(),
            tick_block: None,
            dotions: HashMap::new(),
            loaded_modules: std::collections::HashSet::new(),
        }
    }

    // ── FUNCTION CALLS ────────────────────────────────────────────────

    /// Call a named user-defined function. Saves/restores Core env and stack.
    pub fn call_function(
        &mut self,
        core: &mut Core,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, FluxisError> {
        // Direct lookup first
        let chunk = if let Some(c) = self.fn_chunks.get(name).cloned() {
            c
        } else {
            // The name might be a variable holding a closure name string
            // e.g. double = fn(x){ return x*2; }; map_fn(nums, "double")
            // Check env for a Str value that is an actual fn_chunk key
            let resolved = core.env.get(name).and_then(|v| match v {
                Value::Str(s) => self.fn_chunks.get(&s).cloned(),
                _ => None,
            });
            resolved.ok_or_else(|| {
                runtime_error(&format!("Undefined function '{}'", name))
                    .with_hint(&format!("Define it with: fn {}(...) {{ ... }}", name))
            })?
        };

        // Save current env and stack — fresh frame for the callee
        use crate::vm::env::BvmEnv;
        use crate::vm::stack::Stack;
        let saved_env = std::mem::replace(&mut core.env, BvmEnv::new());
        let saved_stack = std::mem::replace(&mut core.stack, Stack::new());

        // Push args onto fresh stack. Pad with Nil for missing params so
        // default params (which come last) get Nil and trigger their defaults.
        // We detect param count from the chunk's leading Store instructions.
        let param_count = chunk
            .instructions
            .iter()
            .take_while(|op| matches!(op, crate::vm::opcodes::Opcode::Store(_)))
            .count();
        let mut padded = args;
        while padded.len() < param_count {
            padded.push(Value::Nil);
        }
        for v in padded {
            core.stack.push(v);
        }

        core.run(&chunk, self)?;
        let return_val = core.stack.pop().unwrap_or(Value::Nil);

        // Restore caller's env and stack
        core.env = saved_env;
        core.stack = saved_stack;

        Ok(return_val)
    }

    // ── BUILTIN DISPATCH ──────────────────────────────────────────────

    /// Route a builtin call to the correct stdlib module.
    pub fn call_builtin(
        &mut self,
        name: &str,
        args: Vec<Value>,
        vm_core: &mut Core,
    ) -> Result<Value, FluxisError> {
        // Core builtins that don't belong to any stdlib module
        match name {
            "len" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("len", 1, args.len()));
                }
                Ok(match &args[0] {
                    Value::Array(a) => Value::Number(a.len() as i64),
                    Value::Map(m) => Value::Number(m.len() as i64),
                    Value::Str(s) => Value::Number(s.len() as i64),
                    other => {
                        return Err(type_error(&format!(
                            "len() not supported on {}",
                            other.type_name()
                        )));
                    }
                })
            }
            "push" => {
                if args.len() != 2 {
                    return Err(crate::error::arity_error("push", 2, args.len()));
                }
                if let Value::Array(mut a) = args[0].clone() {
                    a.push(args[1].clone());
                    Ok(Value::Array(a))
                } else {
                    Err(type_error("push() requires array"))
                }
            }
            "pop" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("pop", 1, args.len()));
                }
                if let Value::Array(mut a) = args[0].clone() {
                    a.pop();
                    Ok(Value::Array(a))
                } else {
                    Err(type_error("pop() requires array"))
                }
            }
            "type_of" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("type_of", 1, args.len()));
                }
                Ok(Value::Str(args[0].type_name().to_string()))
            }
            "to_str" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("to_str", 1, args.len()));
                }
                Ok(Value::Str(args[0].display()))
            }
            "to_num" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("to_num", 1, args.len()));
                }
                Ok(match &args[0] {
                    Value::Str(s) => s
                        .trim()
                        .parse::<i64>()
                        .map(Value::Number)
                        .unwrap_or(Value::Number(0)),
                    Value::Number(n) => Value::Number(*n),
                    Value::Float(f) => Value::Number(*f as i64),
                    Value::Bool(b) => Value::Number(if *b { 1 } else { 0 }),
                    _ => Value::Number(0),
                })
            }
            "to_float" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("to_float", 1, args.len()));
                }
                Ok(match &args[0] {
                    Value::Number(n) => Value::Float(*n as f64),
                    Value::Float(f) => Value::Float(*f),
                    Value::Str(s) => s
                        .trim()
                        .parse::<f64>()
                        .map(Value::Float)
                        .unwrap_or(Value::Float(0.0)),
                    Value::Bool(b) => Value::Float(if *b { 1.0 } else { 0.0 }),
                    _ => Value::Float(0.0),
                })
            }
            "assert" => {
                if args.is_empty() {
                    return Err(crate::error::arity_error("assert", 1, 0));
                }
                let ok = match &args[0] {
                    Value::Bool(b) => *b,
                    Value::Nil => false,
                    Value::Number(n) => *n != 0,
                    _ => true,
                };
                if !ok {
                    let msg = if args.len() >= 2 {
                        args[1].display()
                    } else {
                        "Assertion failed".to_string()
                    };
                    return Err(runtime_error(&msg).with_hint("assert(condition, \"message\")"));
                }
                Ok(Value::Nil)
            }
            "is_num" => Ok(Value::Bool(matches!(&args[0], Value::Number(_)))),
            "is_float" => Ok(Value::Bool(matches!(&args[0], Value::Float(_)))),
            "is_str" => Ok(Value::Bool(matches!(&args[0], Value::Str(_)))),
            "is_bool" => Ok(Value::Bool(matches!(&args[0], Value::Bool(_)))),
            "is_array" => Ok(Value::Bool(matches!(&args[0], Value::Array(_)))),
            "is_map" => Ok(Value::Bool(matches!(&args[0], Value::Map(_)))),
            "is_nil" => Ok(Value::Bool(matches!(&args[0], Value::Nil))),

            "range" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(runtime_error(
                        "range() takes 2 or 3 args: range(start,end) or range(start,end,step)",
                    ));
                }
                let start = match &args[0] {
                    Value::Number(n) => *n,
                    Value::Float(f) => *f as i64,
                    other => {
                        return Err(type_error(&format!(
                            "range() start must be num, got {}",
                            other.type_name()
                        )));
                    }
                };
                let end = match &args[1] {
                    Value::Number(n) => *n,
                    Value::Float(f) => *f as i64,
                    other => {
                        return Err(type_error(&format!(
                            "range() end must be num, got {}",
                            other.type_name()
                        )));
                    }
                };
                let step = if args.len() == 3 {
                    match &args[2] {
                        Value::Number(n) => *n,
                        Value::Float(f) => *f as i64,
                        other => {
                            return Err(type_error(&format!(
                                "range() step must be num, got {}",
                                other.type_name()
                            )));
                        }
                    }
                } else {
                    if start <= end { 1 } else { -1 }
                };
                if step == 0 {
                    return Err(runtime_error("range() step cannot be zero"));
                }
                let mut arr = Vec::new();
                let mut cur = start;
                if step > 0 {
                    while cur < end {
                        arr.push(Value::Number(cur));
                        cur += step;
                    }
                } else {
                    while cur > end {
                        arr.push(Value::Number(cur));
                        cur += step;
                    }
                }
                Ok(Value::Array(arr))
            }
            "sort_arr" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("sort_arr", 1, args.len()));
                }
                let mut arr = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "sort_arr() requires array, got {}",
                            other.type_name()
                        )));
                    }
                };
                arr.sort_by(|a, b| match (a, b) {
                    (Value::Number(x), Value::Number(y)) => x.cmp(y),
                    (Value::Float(x), Value::Float(y)) => {
                        x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Value::Number(x), Value::Float(y)) => (*x as f64)
                        .partial_cmp(y)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Float(x), Value::Number(y)) => x
                        .partial_cmp(&(*y as f64))
                        .unwrap_or(std::cmp::Ordering::Equal),
                    (a, b) => a.display().cmp(&b.display()),
                });
                Ok(Value::Array(arr))
            }
            "sort_desc" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("sort_desc", 1, args.len()));
                }
                let mut arr = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "sort_desc() requires array, got {}",
                            other.type_name()
                        )));
                    }
                };
                arr.sort_by(|a, b| match (a, b) {
                    (Value::Number(x), Value::Number(y)) => y.cmp(x),
                    (Value::Float(x), Value::Float(y)) => {
                        y.partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Value::Number(x), Value::Float(y)) => y
                        .partial_cmp(&(*x as f64))
                        .unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Float(x), Value::Number(y)) => (*y as f64)
                        .partial_cmp(x)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    (a, b) => b.display().cmp(&a.display()),
                });
                Ok(Value::Array(arr))
            }
            "remove" => {
                if args.len() != 2 {
                    return Err(crate::error::arity_error("remove", 2, args.len()));
                }
                let mut arr = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "remove() requires array, got {}",
                            other.type_name()
                        )));
                    }
                };
                let idx = match &args[1] {
                    Value::Number(n) => *n as usize,
                    other => {
                        return Err(type_error(&format!(
                            "remove() index must be num, got {}",
                            other.type_name()
                        )));
                    }
                };
                if idx >= arr.len() {
                    return Err(runtime_error(&format!(
                        "remove() index {} out of bounds (len={})",
                        idx,
                        arr.len()
                    )));
                }
                arr.remove(idx);
                Ok(Value::Array(arr))
            }
            "insert" => {
                if args.len() != 3 {
                    return Err(crate::error::arity_error("insert", 3, args.len()));
                }
                let mut arr = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "insert() requires array, got {}",
                            other.type_name()
                        )));
                    }
                };
                let idx = match &args[1] {
                    Value::Number(n) => (*n as usize).min(arr.len()),
                    other => {
                        return Err(type_error(&format!(
                            "insert() index must be num, got {}",
                            other.type_name()
                        )));
                    }
                };
                arr.insert(idx, args[2].clone());
                Ok(Value::Array(arr))
            }
            "slice" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(runtime_error(
                        "slice() takes 2 or 3 args: slice(arr,start) or slice(arr,start,end)",
                    ));
                }
                let arr = match args[0].clone() {
                    Value::Array(a) => a,
                    Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    other => {
                        return Err(type_error(&format!(
                            "slice() first arg must be array or string, got {}",
                            other.type_name()
                        )));
                    }
                };
                let start = match &args[1] {
                    Value::Number(n) => (*n).max(0) as usize,
                    other => {
                        return Err(type_error(&format!(
                            "slice() start must be num, got {}",
                            other.type_name()
                        )));
                    }
                };
                let end = if args.len() == 3 {
                    match &args[2] {
                        Value::Number(n) => (*n).max(0) as usize,
                        other => {
                            return Err(type_error(&format!(
                                "slice() end must be num, got {}",
                                other.type_name()
                            )));
                        }
                    }
                } else {
                    arr.len()
                };
                let end = end.min(arr.len());
                let start = start.min(end);
                Ok(Value::Array(arr[start..end].to_vec()))
            }
            "flatten" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("flatten", 1, args.len()));
                }
                let arr = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "flatten() requires array, got {}",
                            other.type_name()
                        )));
                    }
                };
                let mut result = Vec::new();
                for item in arr {
                    match item {
                        Value::Array(inner) => result.extend(inner),
                        other => result.push(other),
                    }
                }
                Ok(Value::Array(result))
            }
            "reverse" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("reverse", 1, args.len()));
                }
                match args[0].clone() {
                    Value::Array(mut a) => {
                        a.reverse();
                        Ok(Value::Array(a))
                    }
                    Value::Str(s) => Ok(Value::Str(s.chars().rev().collect())),
                    other => Err(type_error(&format!(
                        "reverse() requires array or string, got {}",
                        other.type_name()
                    ))),
                }
            }
            "zip" => {
                if args.len() != 2 {
                    return Err(crate::error::arity_error("zip", 2, args.len()));
                }
                let a = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "zip() first arg must be array, got {}",
                            other.type_name()
                        )));
                    }
                };
                let b = match args[1].clone() {
                    Value::Array(b) => b,
                    other => {
                        return Err(type_error(&format!(
                            "zip() second arg must be array, got {}",
                            other.type_name()
                        )));
                    }
                };
                Ok(Value::Array(
                    a.iter()
                        .zip(b.iter())
                        .map(|(x, y)| Value::Array(vec![x.clone(), y.clone()]))
                        .collect(),
                ))
            }
            "keys" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("keys", 1, args.len()));
                }
                match &args[0] {
                    Value::Map(m) => Ok(Value::Array(
                        m.keys().map(|k| Value::Str(k.clone())).collect(),
                    )),
                    Value::Struct { fields, .. } => Ok(Value::Array(
                        fields.keys().map(|k| Value::Str(k.clone())).collect(),
                    )),
                    other => Err(type_error(&format!(
                        "keys() not supported on {}",
                        other.type_name()
                    ))),
                }
            }
            "has" => {
                if args.len() != 2 {
                    return Err(crate::error::arity_error("has", 2, args.len()));
                }
                let key = args[1].display();
                Ok(match &args[0] {
                    Value::Map(m) => Value::Bool(m.contains_key(&key)),
                    Value::Struct { fields, .. } => Value::Bool(fields.contains_key(&key)),
                    other => {
                        return Err(type_error(&format!(
                            "has() not supported on {}",
                            other.type_name()
                        )));
                    }
                })
            }
            "del" => {
                if args.len() != 2 {
                    return Err(crate::error::arity_error("del", 2, args.len()));
                }
                let key = args[1].display();
                if let Value::Map(mut m) = args[0].clone() {
                    m.remove(&key);
                    Ok(Value::Map(m))
                } else {
                    Err(type_error("del() requires a map"))
                }
            }
            "tick_count" => Ok(Value::Number(self.tick_count as i64)),
            "send" => {
                // send(dotion, "msg") or send(dotion, "msg", arg)
                if args.len() < 2 {
                    return Err(runtime_error(
                        "send() takes 2 or 3 args: send(dotion, \"msg\") or send(dotion, \"msg\", val)",
                    ));
                }
                let arg = if args.len() >= 3 {
                    args[2].clone()
                } else {
                    Value::Nil
                };
                let msg = match &args[1] {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(type_error(&format!(
                            "send() msg must be string, got {}",
                            other.type_name()
                        )));
                    }
                };
                let target_id = match &args[0] {
                    Value::Dotion { id, .. } => *id,
                    other => {
                        return Err(type_error(&format!(
                            "send() first arg must be dotion, got {}",
                            other.type_name()
                        )));
                    }
                };
                // Find variable name for this dotion ID in core's env
                for (vname, dotion) in &self.dotions {
                    if let Value::Dotion { id, .. } = dotion {
                        if *id == target_id {
                            self.mailboxes
                                .entry(vname.clone())
                                .or_default()
                                .push((msg, arg));
                            return Ok(Value::Nil);
                        }
                    }
                }
                Err(runtime_error(
                    "send() could not find target dotion in scope",
                ))
            }
            "send_self" => {
                let msg = match &args[0] {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(type_error(&format!(
                            "send_self() msg must be string, got {}",
                            other.type_name()
                        )));
                    }
                };
                let arg = if args.len() >= 2 {
                    args[1].clone()
                } else {
                    Value::Nil
                };
                if let Some(ref vname) = self.self_var.clone() {
                    self.mailboxes
                        .entry(vname.clone())
                        .or_default()
                        .push((msg, arg));
                    Ok(Value::Nil)
                } else {
                    Err(runtime_error(
                        "send_self() used outside of a dotion method or handler",
                    ))
                }
            }
            "broadcast" => {
                let msg = match &args[0] {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(type_error(&format!(
                            "broadcast() msg must be string, got {}",
                            other.type_name()
                        )));
                    }
                };
                let arg = if args.len() >= 2 {
                    args[1].clone()
                } else {
                    Value::Nil
                };
                for vname in self.dotions.keys().cloned().collect::<Vec<_>>() {
                    self.mailboxes
                        .entry(vname)
                        .or_default()
                        .push((msg.clone(), arg.clone()));
                }
                Ok(Value::Nil)
            }
            "broadcast_to" => {
                if args.len() < 2 {
                    return Err(runtime_error("broadcast_to() takes 2 or 3 args"));
                }
                let tag = match &args[0] {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(type_error(&format!(
                            "broadcast_to() tag must be string, got {}",
                            other.type_name()
                        )));
                    }
                };
                let msg = match &args[1] {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(type_error(&format!(
                            "broadcast_to() msg must be string, got {}",
                            other.type_name()
                        )));
                    }
                };
                let arg = if args.len() >= 3 {
                    args[2].clone()
                } else {
                    Value::Nil
                };
                for (vname, dotion) in &self.dotions {
                    if let Value::Dotion { tags, .. } = dotion {
                        if tags.contains(&tag) {
                            self.mailboxes
                                .entry(vname.clone())
                                .or_default()
                                .push((msg.clone(), arg.clone()));
                        }
                    }
                }
                Ok(Value::Nil)
            }
            "clone" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("clone", 1, args.len()));
                }
                match args[0].clone() {
                    Value::Dotion {
                        name,
                        fields,
                        methods,
                        handlers,
                        brain,
                        tags,
                        tick_priority,
                        ..
                    } => Ok(Value::Dotion {
                        id: crate::dop::new_id(),
                        name,
                        fields,
                        methods,
                        handlers,
                        mailbox: Vec::new(),
                        brain,
                        tags,
                        tick_priority,
                    }),
                    other => Err(type_error(&format!(
                        "clone() requires a dotion, got {}",
                        other.type_name()
                    ))),
                }
            }
            "dotion_list" => {
                let mut dotions: Vec<Value> = self.dotions.values().cloned().collect();
                dotions.sort_by_key(|d| match d {
                    Value::Dotion { tick_priority, .. } => *tick_priority,
                    _ => 0,
                });
                Ok(Value::Array(dotions))
            }
            "dotion_count" => Ok(Value::Number(self.dotions.len() as i64)),
            "dotion_where" => {
                if args.len() != 2 {
                    return Err(crate::error::arity_error("dotion_where", 2, args.len()));
                }
                let field = match &args[0] {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(type_error(&format!(
                            "dotion_where() field must be string, got {}",
                            other.type_name()
                        )));
                    }
                };
                let target = args[1].display();
                let result = self.dotions.values()
                    .filter(|v| matches!(v, Value::Dotion { fields, .. } if fields.get(&field).map(|f| f.display()) == Some(target.clone())))
                    .cloned().collect();
                Ok(Value::Array(result))
            }
            "__import__" => {
                if args.len() != 1 {
                    return Err(crate::error::arity_error("import", 1, args.len()));
                }
                let module = match &args[0] {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(type_error(&format!(
                            "import expects string, got {}",
                            other.type_name()
                        )));
                    }
                };
                if module.ends_with(".fx") {
                    // Load and compile external .fx file
                    if self.loaded_modules.contains(&module) {
                        return Ok(Value::Nil);
                    }
                    let source = std::fs::read_to_string(&module).map_err(|e| {
                        runtime_error(&format!("import \"{}\": {}", module, e)).with_hint(
                            "Check the file path is correct relative to where you run fluxis",
                        )
                    })?;
                    self.loaded_modules.insert(module.clone());
                    use crate::compiler::Compiler;
                    use crate::lexer::Lexer;
                    use crate::parser::Parser;
                    let tokens = Lexer::new(&source).lex().map_err(|e| e)?;
                    let program = Parser::new(tokens, &source).parse().map_err(|e| e)?;
                    let mut comp = Compiler::new();
                    let chunk = comp.compile(program)?;
                    for (name, fn_chunk) in comp.functions {
                        self.fn_chunks.insert(name, fn_chunk);
                    }
                    vm_core.run(&chunk, self)?;
                    let _ = vm_core.stack.pop();
                } else {
                    // stdlib module — already available, just mark as loaded
                    if crate::stdlib::load_module(&module).is_none() {
                        return Err(runtime_error(&format!(
                            "Unknown module '{}'. Available: math, string, io, ai, ml, gfx, or a .fx file",
                            module
                        )));
                    }
                    self.loaded_modules.insert(module);
                }
                Ok(Value::Nil)
            }

            // ── DOP INTERNAL BUILTINS ─────────────────────────────────
            "__dotion_new__" => {
                // Stack (in args): [field_name0, field_val0, ..., override_count, type_name]
                // Last arg is type name, second-to-last is override count
                let type_name = match args.last() {
                    Some(Value::Str(s)) => s.clone(),
                    _ => return Err(runtime_error("__dotion_new__: missing type name")),
                };
                let override_count = match args.get(args.len().saturating_sub(2)) {
                    Some(Value::Number(n)) => *n as usize,
                    _ => 0,
                };
                let overrides_flat = &args[..override_count * 2];

                // Build override map
                let mut overrides: HashMap<String, Value> = HashMap::new();
                let mut i = 0;
                while i + 1 < overrides_flat.len() {
                    if let Value::Str(k) = &overrides_flat[i] {
                        overrides.insert(k.clone(), overrides_flat[i + 1].clone());
                    }
                    i += 2;
                }

                // Instantiate from type definition
                self.instantiate_dotion(vm_core, &type_name, overrides)
            }

            "__tick_run__" => {
                let n = match &args[0] {
                    Value::Number(n) => *n as u64,
                    other => {
                        return Err(type_error(&format!(
                            "tick() expects num, got {}",
                            other.type_name()
                        )));
                    }
                };
                // Register tick block if not yet done
                if self.tick_block.is_none() && self.fn_chunks.contains_key("__tick_block__") {
                    self.tick_block = Some("__tick_block__".to_string());
                }
                for _ in 0..n {
                    self.run_tick(vm_core)?;
                }
                Ok(Value::Nil)
            }

            // self.field getter — called inside method/handler compiled code
            _ if name.starts_with("__self_get__") => {
                let field = &name["__self_get__".len()..];
                match vm_core.env.get("__self__") {
                    Some(Value::Dotion { fields, .. }) => {
                        Ok(fields.get(field).cloned().unwrap_or(Value::Nil))
                    }
                    _ => {
                        // Fallback: try self_var name
                        let self_name = self.self_var.clone().ok_or_else(|| {
                            runtime_error("'self' used outside of a dotion method or handler")
                        })?;
                        match vm_core.env.get(&self_name) {
                            Some(Value::Dotion { fields, .. }) => {
                                Ok(fields.get(field).cloned().unwrap_or(Value::Nil))
                            }
                            _ => Err(runtime_error(&format!(
                                "self.{} — __self__ is not a dotion",
                                field
                            ))),
                        }
                    }
                }
            }

            _ if name.starts_with("__self_set__") => {
                let field = &name["__self_set__".len()..];
                let new_val = args.into_iter().next().unwrap_or(Value::Nil);
                match vm_core.env.get("__self__") {
                    Some(Value::Dotion {
                        id,
                        name: dn,
                        mut fields,
                        methods,
                        handlers,
                        mailbox,
                        brain,
                        tags,
                        tick_priority,
                    }) => {
                        fields.insert(field.to_string(), new_val);
                        vm_core.env.set(
                            "__self__",
                            Value::Dotion {
                                id,
                                name: dn,
                                fields,
                                methods,
                                handlers,
                                mailbox,
                                brain,
                                tags,
                                tick_priority,
                            },
                        );
                        Ok(Value::Nil)
                    }
                    _ => {
                        let self_name = self.self_var.clone().ok_or_else(|| {
                            runtime_error("'self' used outside of a dotion method or handler")
                        })?;
                        match vm_core.env.get(&self_name) {
                            Some(Value::Dotion {
                                id,
                                name: dn,
                                mut fields,
                                methods,
                                handlers,
                                mailbox,
                                brain,
                                tags,
                                tick_priority,
                            }) => {
                                fields.insert(field.to_string(), new_val);
                                vm_core.env.set(
                                    &self_name,
                                    Value::Dotion {
                                        id,
                                        name: dn,
                                        fields,
                                        methods,
                                        handlers,
                                        mailbox,
                                        brain,
                                        tags,
                                        tick_priority,
                                    },
                                );
                                Ok(Value::Nil)
                            }
                            _ => Err(runtime_error(&format!(
                                "self.{} — __self__ is not a dotion",
                                field
                            ))),
                        }
                    }
                }
            }

            // dotion method call: __dopcall__METHODNAME
            // Last arg is the var name string (pushed by compiler)
            _ if name.starts_with("__dopcall__") => {
                let method_name = name["__dopcall__".len()..].to_string();
                let var_name = match args.last() {
                    Some(Value::Str(s)) => s.clone(),
                    _ => return Err(runtime_error("__dopcall__: missing var name")),
                };
                let method_args = args[..args.len() - 1].to_vec();
                self.call_dotion_method(vm_core, &var_name, &method_name, method_args)
            }

            "map_fn" => {
                if args.len() != 2 {
                    return Err(crate::error::arity_error("map_fn", 2, args.len()));
                }
                let arr = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "map_fn() first arg must be array, got {}",
                            other.type_name()
                        )));
                    }
                };
                let fn_name = match args[1].clone() {
                    Value::Str(s) => s,
                    other => {
                        return Err(type_error(&format!(
                            "map_fn() second arg must be function name string, got {}",
                            other.type_name()
                        )));
                    }
                };
                let mut result = Vec::new();
                for item in arr {
                    result.push(self.call_user_fn_by_name(&fn_name, vec![item])?);
                }
                Ok(Value::Array(result))
            }
            "filter_fn" => {
                if args.len() != 2 {
                    return Err(crate::error::arity_error("filter_fn", 2, args.len()));
                }
                let arr = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "filter_fn() first arg must be array, got {}",
                            other.type_name()
                        )));
                    }
                };
                let fn_name = match args[1].clone() {
                    Value::Str(s) => s,
                    other => {
                        return Err(type_error(&format!(
                            "filter_fn() second arg must be function name string, got {}",
                            other.type_name()
                        )));
                    }
                };
                let mut result = Vec::new();
                for item in arr {
                    let keep = self.call_user_fn_by_name(&fn_name, vec![item.clone()])?;
                    if keep.is_truthy() {
                        result.push(item);
                    }
                }
                Ok(Value::Array(result))
            }
            "reduce_fn" => {
                if args.len() != 3 {
                    return Err(crate::error::arity_error("reduce_fn", 3, args.len()));
                }
                let arr = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "reduce_fn() first arg must be array, got {}",
                            other.type_name()
                        )));
                    }
                };
                let fn_name = match args[1].clone() {
                    Value::Str(s) => s,
                    other => {
                        return Err(type_error(&format!(
                            "reduce_fn() second arg must be function name string, got {}",
                            other.type_name()
                        )));
                    }
                };
                let mut acc = args[2].clone();
                for item in arr {
                    acc = self.call_user_fn_by_name(&fn_name, vec![acc, item])?;
                }
                Ok(acc)
            }
            "any_fn" => {
                if args.len() != 2 {
                    return Err(crate::error::arity_error("any_fn", 2, args.len()));
                }
                let arr = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "any_fn() first arg must be array, got {}",
                            other.type_name()
                        )));
                    }
                };
                let fn_name = match args[1].clone() {
                    Value::Str(s) => s,
                    other => {
                        return Err(type_error(&format!(
                            "any_fn() second arg must be function name string, got {}",
                            other.type_name()
                        )));
                    }
                };
                for item in arr {
                    if self.call_user_fn_by_name(&fn_name, vec![item])?.is_truthy() {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
            "all_fn" => {
                if args.len() != 2 {
                    return Err(crate::error::arity_error("all_fn", 2, args.len()));
                }
                let arr = match args[0].clone() {
                    Value::Array(a) => a,
                    other => {
                        return Err(type_error(&format!(
                            "all_fn() first arg must be array, got {}",
                            other.type_name()
                        )));
                    }
                };
                let fn_name = match args[1].clone() {
                    Value::Str(s) => s,
                    other => {
                        return Err(type_error(&format!(
                            "all_fn() second arg must be function name string, got {}",
                            other.type_name()
                        )));
                    }
                };
                for item in arr {
                    if !self.call_user_fn_by_name(&fn_name, vec![item])?.is_truthy() {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            }

            // ── Route to stdlib modules ───────────────────────────────
            _ if stdlib::is_math_fn(name) => crate::stdlib::math::call(name, &args),
            _ if stdlib::is_string_fn(name) => crate::stdlib::string::call(name, &args),
            _ if stdlib::is_io_fn(name) => crate::stdlib::io::call(name, &args),
            _ if stdlib::is_ml_fn(name) => crate::stdlib::ml::call(name, &args),
            _ if stdlib::is_gfx_fn(name) => crate::stdlib::gfx::call(name, &args),
            _ if stdlib::is_ai_fn(name) => crate::stdlib::ai::call(name, &args),

            // ── CLOSURES ─────────────────────────────────────────────
            "__call_closure__" => {
                let fn_name = match args.last() {
                    Some(Value::Str(s)) => s.clone(),
                    _ => {
                        return Err(runtime_error(
                            "__call_closure__: expected closure name as last arg",
                        ));
                    }
                };
                let fn_args = args[..args.len() - 1].to_vec();
                let chunk = self.fn_chunks.get(&fn_name).cloned().ok_or_else(|| {
                    runtime_error(&format!("Undefined closure '{}'", fn_name))
                        .with_hint("Make sure the closure variable is in scope")
                })?;
                use crate::vm::env::BvmEnv;
                use crate::vm::stack::Stack;
                let saved_env = std::mem::replace(&mut vm_core.env, BvmEnv::new());
                let saved_stack = std::mem::replace(&mut vm_core.stack, Stack::new());
                for v in fn_args {
                    vm_core.stack.push(v);
                }
                vm_core.run(&chunk, self)?;
                let ret = vm_core.stack.pop().unwrap_or(Value::Nil);
                vm_core.env = saved_env;
                vm_core.stack = saved_stack;
                Ok(ret)
            }

            // ── OPTIONAL CHAIN ─────────────────────────────────────
            "__optional_chain__" => {
                let field = match &args[1] {
                    Value::Str(s) => s.clone(),
                    _ => return Err(type_error("optional chain field must be string")),
                };
                match &args[0] {
                    Value::Nil => Ok(Value::Nil),
                    Value::Struct { fields, .. } | Value::Dotion { fields, .. } => {
                        Ok(fields.get(&field).cloned().unwrap_or(Value::Nil))
                    }
                    Value::Map(m) => Ok(m.get(&field).cloned().unwrap_or(Value::Nil)),
                    _ => Ok(Value::Nil),
                }
            }

            // ── IN OPERATOR ──────────────────────────────────────────
            "__in__" => {
                let val = &args[0];
                match &args[1] {
                    Value::Array(arr) => Ok(Value::Bool(arr.iter().any(|x| x.equals(val)))),
                    Value::Map(m) => Ok(Value::Bool(m.contains_key(&val.display()))),
                    Value::Str(s) => Ok(Value::Bool(s.contains(&*val.display()))),
                    other => Err(type_error(&format!(
                        "'in' operator not supported for {}",
                        other.type_name()
                    ))),
                }
            }

            // ── VARIADIC / TRY-CATCH stubs ───────────────────────────
            "__collect_variadic__" => Ok(Value::Array(Vec::new())),
            "__match_struct__" => {
                // Check if value is a struct/dotion with the given type name
                let type_name = match &args[1] {
                    Value::Str(s) => s.clone(),
                    _ => return Ok(Value::Bool(false)),
                };
                Ok(Value::Bool(match &args[0] {
                    Value::Struct { name, .. } => name == &type_name,
                    Value::Dotion { name, .. } => name == &type_name,
                    _ => false,
                }))
            }
            "__try_start__" | "__try_end__" | "__catch_start__" | "__catch_end__" => Ok(Value::Nil),

            _ => Err(runtime_error(&format!("Unknown builtin '{}'", name))),
        }
    }

    /// Call a user-defined function by name without a Core reference.
    fn call_user_fn_by_name(&mut self, name: &str, args: Vec<Value>) -> Result<Value, FluxisError> {
        let mut core = Core::new();
        // If name is a variable holding a closure string, resolve it
        let actual_name = if self.fn_chunks.contains_key(name) {
            name.to_string()
        } else {
            // Check if it's stored in global dotions or just try direct
            name.to_string()
        };
        self.call_function(&mut core, &actual_name, args)
    }

    // ── DOP HELPERS ───────────────────────────────────────────────────

    /// Instantiate a dotion type by name, applying field overrides.
    fn instantiate_dotion(
        &mut self,
        _core: &mut Core,
        type_name: &str,
        overrides: HashMap<String, Value>,
    ) -> Result<Value, FluxisError> {
        // Collect inherited chain (extends)
        let mut type_chain = vec![type_name.to_string()];
        let mut current = type_name.to_string();
        loop {
            let reg_key = format!("__dopreg__{}", current);
            if let Some(chunk) = self.fn_chunks.get(&reg_key).cloned() {
                // chunk pushes: name, extends, brain, tags, tick_priority

                let mut tmp = Core::new();
                self.call_chunk_raw(&mut tmp, &chunk)?;
                // stack bottom to top: name, extends, brain, tags, tick_priority
                let vals: Vec<Value> = tmp.stack.drain_all();
                let parent = match vals.get(1) {
                    Some(Value::Str(s)) if !s.is_empty() => s.clone(),
                    _ => break,
                };
                type_chain.insert(0, parent.clone());
                current = parent;
            } else {
                break;
            }
        }

        // Build fields, methods, handlers by walking chain base → derived
        let mut fields: HashMap<String, Value> = HashMap::new();
        let mut methods: Vec<crate::ast::DotionMethod> = Vec::new();
        let mut handlers: Vec<crate::ast::Handler> = Vec::new();
        let mut brain: Option<String> = None;
        let mut tags: Vec<String> = Vec::new();
        let mut tick_priority: i64 = 0;

        for tname in &type_chain {
            // Evaluate default field values
            let def_key = format!("__doptype__{}", tname);
            if let Some(chunk) = self.fn_chunks.get(&def_key).cloned() {
                let mut tmp = Core::new();
                self.call_chunk_raw(&mut tmp, &chunk)?;
                let vals = tmp.stack.drain_all();
                // vals: [name0, val0, name1, val1, ..., count]
                if let Some(Value::Number(count)) = vals.last() {
                    let count = *count as usize;
                    let pairs = &vals[..count * 2];
                    let mut i = 0;
                    while i + 1 < pairs.len() {
                        if let Value::Str(k) = &pairs[i] {
                            fields.insert(k.clone(), pairs[i + 1].clone());
                        }
                        i += 2;
                    }
                }
            }

            // Collect metadata
            let reg_key = format!("__dopreg__{}", tname);
            if let Some(chunk) = self.fn_chunks.get(&reg_key).cloned() {
                let mut tmp = Core::new();
                self.call_chunk_raw(&mut tmp, &chunk)?;
                let vals = tmp.stack.drain_all();
                // vals: [name, extends, brain, tags, tick_priority]
                if let Some(Value::Str(b)) = vals.get(2) {
                    if !b.is_empty() {
                        brain = Some(b.clone());
                    }
                }
                if let Some(Value::Array(t)) = vals.get(3) {
                    tags = t
                        .iter()
                        .filter_map(|v| {
                            if let Value::Str(s) = v {
                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                }
                if let Some(Value::Number(p)) = vals.get(4) {
                    tick_priority = *p;
                }
            }

            // Collect method names (they're stored as fn chunks)
            for key in self
                .fn_chunks
                .keys()
                .filter(|k| k.starts_with(&format!("__dopmethod__{}_", tname)))
                .cloned()
                .collect::<Vec<_>>()
            {
                let mname = key[format!("__dopmethod__{}_", tname).len()..].to_string();
                // Remove old override if exists, add new
                methods.retain(|m: &crate::ast::DotionMethod| m.name != mname);
                methods.push(crate::ast::DotionMethod {
                    name: mname,
                    params: Vec::new(),
                    body: Vec::new(),
                });
            }

            // Collect handler names
            for key in self
                .fn_chunks
                .keys()
                .filter(|k| k.starts_with(&format!("__dophandler__{}_", tname)))
                .cloned()
                .collect::<Vec<_>>()
            {
                let hname = key[format!("__dophandler__{}_", tname).len()..].to_string();
                handlers.retain(|h: &crate::ast::Handler| h.msg != hname);
                handlers.push(crate::ast::Handler {
                    msg: hname,
                    param: None,
                    body: Vec::new(),
                });
            }
        }

        // Apply overrides
        for (k, v) in overrides {
            fields.insert(k, v);
        }

        Ok(Value::Dotion {
            id: crate::dop::new_id(),
            name: type_name.to_string(),
            fields,
            methods,
            handlers,
            mailbox: Vec::new(),
            brain,
            tags,
            tick_priority,
        })
    }

    /// Run a chunk without saving/restoring core state.
    fn call_chunk_raw(&mut self, core: &mut Core, chunk: &Chunk) -> Result<(), FluxisError> {
        core.run(chunk, self)
    }

    /// Call a method on a dotion stored in the env by var name.
    pub fn call_dotion_method(
        &mut self,
        core: &mut Core,
        var_name: &str,
        method_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, FluxisError> {
        let dotion = self
            .dotions
            .get(var_name)
            .cloned()
            .or_else(|| core.env.get(var_name))
            .ok_or_else(|| crate::error::scope_error(&format!("'{}' is not defined", var_name)))?;
        let type_name = match &dotion {
            Value::Dotion { name, .. } => name.clone(),
            other => {
                return Err(type_error(&format!(
                    "'{}' is not a dotion, got {}",
                    var_name,
                    other.type_name()
                )));
            }
        };

        // Walk the inheritance chain to find the method chunk
        let chunk = self
            .find_method_chunk(&type_name, method_name)
            .ok_or_else(|| {
                runtime_error(&format!(
                    "Dotion '{}' has no method '{}'",
                    type_name, method_name
                ))
            })?;

        let prev_self = self.self_var.clone();
        self.self_var = Some(var_name.to_string());

        use crate::vm::env::BvmEnv;
        use crate::vm::stack::Stack;
        let saved_env = std::mem::replace(&mut core.env, BvmEnv::new());
        let saved_stack = std::mem::replace(&mut core.stack, Stack::new());

        core.env.set("__self__", dotion);

        for v in args {
            core.stack.push(v);
        }
        core.run(&chunk, self)?;
        let ret = core.stack.pop().unwrap_or(Value::Nil);

        let mutated_self = core.env.get("__self__");
        core.env = saved_env;
        core.stack = saved_stack;
        self.self_var = prev_self;

        if let Some(mutated) = mutated_self {
            self.dotions.insert(var_name.to_string(), mutated.clone());
            core.env.set(var_name, mutated);
        }

        Ok(ret)
    }

    /// Walk the inheritance chain to find a method chunk.
    fn find_method_chunk(&self, type_name: &str, method_name: &str) -> Option<Chunk> {
        let mut current = type_name.to_string();
        loop {
            let key = format!("__dopmethod__{}_{}", current, method_name);
            if let Some(chunk) = self.fn_chunks.get(&key) {
                return Some(chunk.clone());
            }
            // Look up parent via reg chunk (index 1 = extends string)
            let reg_key = format!("__dopreg__{}", current);
            let chunk = self.fn_chunks.get(&reg_key)?.clone();
            // Run the reg chunk to get metadata — but we can't run it here without a Core.
            // Instead, cache parent info in a simpler way: scan for the extends value.
            // The reg chunk pushes: name, extends, brain, tags, tick_priority as constants.
            // We can read the extends directly from the chunk's instructions.
            use crate::vm::opcodes::Opcode;
            let parent = chunk.instructions.iter().nth(1).and_then(|op| {
                if let Opcode::Push(Value::Str(s)) = op {
                    if !s.is_empty() {
                        return Some(s.clone());
                    }
                }
                None
            })?;
            current = parent;
        }
    }

    /// Walk the inheritance chain to find a handler chunk.
    fn find_handler_chunk(&self, type_name: &str, msg: &str) -> Option<Chunk> {
        let mut current = type_name.to_string();
        loop {
            let key = format!("__dophandler__{}_{}", current, msg);
            if let Some(chunk) = self.fn_chunks.get(&key) {
                return Some(chunk.clone());
            }
            let reg_key = format!("__dopreg__{}", current);
            let chunk = self.fn_chunks.get(&reg_key)?.clone();
            use crate::vm::opcodes::Opcode;
            let parent = chunk.instructions.iter().nth(1).and_then(|op| {
                if let Opcode::Push(Value::Str(s)) = op {
                    if !s.is_empty() {
                        return Some(s.clone());
                    }
                }
                None
            })?;
            current = parent;
        }
    }

    /// Run one full tick cycle: process mailboxes → run actor brains → run tick block.
    pub fn run_tick(&mut self, core: &mut Core) -> Result<(), FluxisError> {
        // Sort dotions by tick_priority
        let mut dotion_vars: Vec<(String, i64)> = self
            .dotions
            .iter()
            .map(|(n, d)| {
                (
                    n.clone(),
                    match d {
                        Value::Dotion { tick_priority, .. } => *tick_priority,
                        _ => 0,
                    },
                )
            })
            .collect();
        dotion_vars.sort_by_key(|(_, p)| *p);

        // Phase 1: deliver mailboxes
        let pending: HashMap<String, Vec<(String, Value)>> = std::mem::take(&mut self.mailboxes);
        for (var_name, messages) in pending {
            for (msg, arg) in messages {
                self.deliver_message(core, &var_name, &msg, arg)?;
            }
        }

        // Phase 2: actor brains
        let brain_vars: Vec<(String, String)> = dotion_vars
            .iter()
            .filter_map(|(n, _)| match self.dotions.get(n) {
                Some(Value::Dotion { brain: Some(b), .. }) => Some((n.clone(), b.clone())),
                _ => None,
            })
            .collect();

        for (var_name, brain_name) in brain_vars {
            let fn_key = format!("__actomethod__{}_decide", brain_name);
            if let Some(chunk) = self.fn_chunks.get(&fn_key).cloned() {
                let dotion_val = self.dotions.get(&var_name).cloned().unwrap_or(Value::Nil);
                use crate::vm::env::BvmEnv;
                use crate::vm::stack::Stack;
                let saved_env = std::mem::replace(&mut core.env, BvmEnv::new());
                let saved_stack = std::mem::replace(&mut core.stack, Stack::new());
                let prev_self = self.self_var.clone();
                self.self_var = Some(var_name.clone());
                core.env.set("__self__", dotion_val.clone());
                core.stack.push(dotion_val);
                core.run(&chunk, self)?;
                // Write back mutated self
                let mutated = core.env.get("__self__");
                core.env = saved_env;
                core.stack = saved_stack;
                self.self_var = prev_self;
                if let Some(m) = mutated {
                    self.dotions.insert(var_name.clone(), m.clone());
                    core.env.set(&var_name, m);
                }
                // Process any messages sent by brain immediately
                let brain_msgs: HashMap<String, Vec<(String, Value)>> =
                    std::mem::take(&mut self.mailboxes);
                for (vn, msgs) in brain_msgs {
                    for (msg, arg) in msgs {
                        self.deliver_message(core, &vn, &msg, arg)?;
                    }
                }
            }
        }

        // Phase 3: tick block — run directly in current env
        if let Some(ref block_name) = self.tick_block.clone() {
            if let Some(chunk) = self.fn_chunks.get(block_name).cloned() {
                core.run(&chunk, self)?;
                let _ = core.stack.pop();
            }
        }

        self.tick_count += 1;
        Ok(())
    }

    /// Deliver one message to a dotion's handler.
    fn deliver_message(
        &mut self,
        core: &mut Core,
        var_name: &str,
        msg: &str,
        arg: Value,
    ) -> Result<(), FluxisError> {
        // Look up dotion from global registry first, fall back to env
        let dotion = match self
            .dotions
            .get(var_name)
            .cloned()
            .or_else(|| core.env.get(var_name))
        {
            Some(v) => v,
            None => return Ok(()),
        };
        let type_name = match &dotion {
            Value::Dotion { name, .. } => name.clone(),
            _ => return Ok(()),
        };

        let fn_key = format!("__dophandler__{}_{}", type_name, msg);
        let chunk = match self.find_handler_chunk(&type_name, msg) {
            Some(c) => c,
            None => return Ok(()),
        };
        let _ = fn_key;

        use crate::vm::env::BvmEnv;
        use crate::vm::stack::Stack;
        let saved_env = std::mem::replace(&mut core.env, BvmEnv::new());
        let saved_stack = std::mem::replace(&mut core.stack, Stack::new());
        let prev_self = self.self_var.clone();
        self.self_var = Some(var_name.to_string());

        core.env.set("__self__", dotion);
        core.stack.push(arg);
        core.run(&chunk, self)?;

        // Write back mutated self to both env and dotion registry
        let mutated = core.env.get("__self__");
        core.env = saved_env;
        core.stack = saved_stack;
        self.self_var = prev_self;

        if let Some(m) = mutated {
            self.dotions.insert(var_name.to_string(), m.clone());
            core.env.set(var_name, m);
        }

        Ok(())
    }

    // ── DOP SIGNALS ───────────────────────────────────────────────────

    /// Enqueue a message on a dotion found by ID in the Core's env.
    pub fn send_message(
        &mut self,
        core: &mut Core,
        dotion: Value,
        msg: String,
        arg: Value,
    ) -> Result<(), FluxisError> {
        let tid = match &dotion {
            Value::Dotion { id, .. } => *id,
            other => {
                return Err(type_error(&format!(
                    "send() requires a dotion, got {}",
                    other.type_name()
                )));
            }
        };
        // Find the dotion in env by ID and enqueue
        let names: Vec<String> = core
            .env
            .frames
            .iter()
            .rev()
            .flat_map(|f| f.keys().cloned())
            .collect();
        for name in names {
            if let Some(Value::Dotion {
                id,
                name: dn,
                fields,
                methods,
                handlers,
                mut mailbox,
                brain,
                tags,
                tick_priority,
            }) = core.env.get(&name)
            {
                if id == tid {
                    mailbox.push((msg, arg));
                    core.env.set(
                        &name,
                        Value::Dotion {
                            id,
                            name: dn,
                            fields,
                            methods,
                            handlers,
                            mailbox,
                            brain,
                            tags,
                            tick_priority,
                        },
                    );
                    return Ok(());
                }
            }
        }
        Err(runtime_error("send() could not find target dotion"))
    }

    /// Call a method on a dotion value. Stub — full DOP runs in tree-walking VM.
    pub fn call_method(
        &mut self,
        _core: &mut Core,
        _dotion: Value,
        name: &str,
        _args: Vec<Value>,
    ) -> Result<Value, FluxisError> {
        // Full DOP method dispatch lives in the tree-walking VM.
        // The bytecode VM forwards method calls here; future work will
        // implement full dispatch at this layer.
        Err(runtime_error(&format!(
            "Method calls on dotions require the tree-walking VM (method: '{}')",
            name
        ))
        .with_hint("Run without --vm flag for full DOP support"))
    }

    /// Handle a TickStep opcode — signal the DOP scheduler.
    pub fn tick_step(&mut self, _core: &mut Core) -> Result<(), FluxisError> {
        // Tick scheduling is owned by the DOP module.
        // In bytecode mode, TickStep is a no-op until the DOP scheduler
        // is wired into the bytecode pipeline.
        Ok(())
    }
}

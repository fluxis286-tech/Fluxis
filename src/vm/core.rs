// FLUXIS — vm/core.rs
// The bytecode execution loop. Only two things live here:
//   1. The program counter (ip)
//   2. The instruction dispatch loop
// Nothing else. No builtins. No scope logic. No value coercions.
// Those all live in runtime.rs, env.rs, stack.rs, and value.rs.

use crate::error::{FluxisError, runtime_error, scope_error, type_error};
use crate::vm::env::BvmEnv;
use crate::vm::opcodes::{Chunk, Opcode};
use crate::vm::runtime::Runtime;
use crate::vm::stack::Stack;
use crate::vm::value::Value;
use std::collections::HashMap;

/// The bytecode core — owns the program counter, operand stack,
/// and scope env. Delegates all logic to Runtime.
pub struct Core {
    pub stack: Stack,
    pub env: BvmEnv,
}

impl Core {
    pub fn new() -> Self {
        Self {
            stack: Stack::new(),
            env: BvmEnv::new(),
        }
    }

    /// Execute a single opcode in a sub-context. Used by TryCatch.
    fn exec_op(&mut self, op: Opcode, chunk: &Chunk, rt: &mut Runtime) -> Result<(), FluxisError> {
        let mut mini = crate::vm::opcodes::Chunk::new("__try__");
        mini.instructions.push(op);
        mini.instructions.push(Opcode::Halt);
        mini.constants = chunk.constants.clone();
        let mut sub = Core::new();
        sub.env = std::mem::replace(&mut self.env, BvmEnv::new());
        sub.stack = std::mem::replace(&mut self.stack, Stack::new());
        let result = sub.run(&mini, rt);
        self.env = sub.env;
        self.stack = sub.stack;
        result
    }

    /// Run a compiled Chunk to completion.
    /// `rt` is the Runtime that handles calls, builtins, and DOP signals.
    pub fn run(&mut self, chunk: &Chunk, rt: &mut Runtime) -> Result<(), FluxisError> {
        let mut ip: usize = 0;
        let instrs = &chunk.instructions;

        loop {
            if ip >= instrs.len() {
                break;
            }
            let op = instrs[ip].clone();
            ip += 1;

            match op {
                // ── HALT / NOP ────────────────────────────────────────
                Opcode::Halt => break,
                Opcode::Nop => {}

                Opcode::TryCatch(catch_addr) => {
                    let save_stack = self.stack.data.len();
                    let mut caught: Option<String> = None;
                    let mut after_catch = catch_addr; // updated when TryEnd is found

                    // Run try body instructions until TryEnd or error
                    while ip < instrs.len() {
                        let inner_op = instrs[ip].clone();
                        if let Opcode::TryEnd(after) = inner_op {
                            after_catch = after; // record skip-past-catch target
                            ip += 1;
                            break;
                        }
                        ip += 1;
                        if let Err(e) = self.exec_op(inner_op, chunk, rt) {
                            caught = Some(e.message.clone());
                            self.stack.data.truncate(save_stack);
                            ip = catch_addr;
                            break;
                        }
                    }

                    if caught.is_none() {
                        // Success — jump past catch block
                        ip = after_catch;
                    } else {
                        // Error — store message, ip already set to catch_addr
                        if let Some(msg) = caught {
                            self.env.set("__try_err__", Value::Str(msg));
                        }
                    }
                }

                Opcode::TryEnd(_) => {
                    // Only reached if somehow not consumed by TryCatch inner loop
                    // Safe no-op
                }

                Opcode::CatchBind(var_name) => {
                    let err = self
                        .env
                        .get("__try_err__")
                        .unwrap_or(Value::Str("unknown error".to_string()));
                    self.env.set(&var_name, err);
                }

                // ── STACK ─────────────────────────────────────────────
                Opcode::Push(v) => self.stack.push(v),
                Opcode::Pop => {
                    self.stack.pop()?;
                }
                Opcode::Dup => {
                    let v = self.stack.peek()?.clone();
                    self.stack.push(v);
                }

                // ── SCOPE ─────────────────────────────────────────────
                Opcode::PushScope => self.env.push(),
                Opcode::PopScope => self.env.pop(),

                // ── VARIABLES ─────────────────────────────────────────
                Opcode::Load(name) => {
                    let val = self.env.get(&name).ok_or_else(|| {
                        scope_error(&format!("'{}' is not defined", name))
                            .with_hint(&format!("Declare it first: {} = <value>;", name))
                    })?;
                    self.stack.push(val);
                }
                Opcode::Store(name) => {
                    let val = self.stack.pop().unwrap_or(Value::Nil);
                    if matches!(val, Value::Dotion { .. }) {
                        rt.dotions.insert(name.clone(), val.clone());
                    } else {
                        rt.dotions.remove(&name);
                    }
                    if let Value::Str(ref s) = val {
                        if s.starts_with("__closure_") {
                            if let Some(chunk) = rt.fn_chunks.get(s).cloned() {
                                rt.fn_chunks.insert(name.clone(), chunk);
                            }
                        }
                    }
                    self.env.set(&name, val);
                }
                Opcode::Inc(name) => {
                    let cur = self.env.get(&name).unwrap_or(Value::Number(0));
                    match cur {
                        Value::Number(n) => self.env.set(&name, Value::Number(n + 1)),
                        other => {
                            return Err(type_error(&format!(
                                "Cannot increment '{}': expected num, got {}",
                                name,
                                other.type_name()
                            )));
                        }
                    }
                }
                Opcode::Dec(name) => {
                    let cur = self.env.get(&name).unwrap_or(Value::Number(0));
                    match cur {
                        Value::Number(n) => self.env.set(&name, Value::Number(n - 1)),
                        other => {
                            return Err(type_error(&format!(
                                "Cannot decrement '{}': expected num, got {}",
                                name,
                                other.type_name()
                            )));
                        }
                    }
                }

                // ── ARITHMETIC ────────────────────────────────────────
                Opcode::Add => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(match (l, r) {
                        (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                        (Value::Float(a), Value::Number(b)) => Value::Float(a + b as f64),
                        (Value::Number(a), Value::Float(b)) => Value::Float(a as f64 + b),
                        (a, b) => Value::Str(format!("{}{}", a.display(), b.display())),
                    });
                }
                Opcode::Sub => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(match (l, r) {
                        (Value::Number(a), Value::Number(b)) => Value::Number(a - b),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
                        (Value::Float(a), Value::Number(b)) => Value::Float(a - b as f64),
                        (Value::Number(a), Value::Float(b)) => Value::Float(a as f64 - b),
                        (l, r) => {
                            return Err(type_error(&format!(
                                "Cannot subtract {} from {}",
                                r.type_name(),
                                l.type_name()
                            )));
                        }
                    });
                }
                Opcode::Mul => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(match (l, r) {
                        (Value::Number(a), Value::Number(b)) => Value::Number(a * b),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a * b),
                        (Value::Float(a), Value::Number(b)) => Value::Float(a * b as f64),
                        (Value::Number(a), Value::Float(b)) => Value::Float(a as f64 * b),
                        (l, r) => {
                            return Err(type_error(&format!(
                                "Cannot multiply {} and {}",
                                l.type_name(),
                                r.type_name()
                            )));
                        }
                    });
                }
                Opcode::Div => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(match (l, r) {
                        (Value::Number(a), Value::Number(b)) => {
                            if b == 0 {
                                return Err(runtime_error("Division by zero"));
                            }
                            Value::Number(a / b)
                        }
                        (Value::Float(a), Value::Float(b)) => {
                            if b == 0.0 {
                                return Err(runtime_error("Division by zero"));
                            }
                            Value::Float(a / b)
                        }
                        (Value::Float(a), Value::Number(b)) => {
                            if b == 0 {
                                return Err(runtime_error("Division by zero"));
                            }
                            Value::Float(a / b as f64)
                        }
                        (Value::Number(a), Value::Float(b)) => {
                            if b == 0.0 {
                                return Err(runtime_error("Division by zero"));
                            }
                            Value::Float(a as f64 / b)
                        }
                        (l, r) => {
                            return Err(type_error(&format!(
                                "Cannot divide {} by {}",
                                l.type_name(),
                                r.type_name()
                            )));
                        }
                    });
                }
                Opcode::Mod => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(match (l, r) {
                        (Value::Number(a), Value::Number(b)) => {
                            if b == 0 {
                                return Err(runtime_error("Modulo by zero"));
                            }
                            Value::Number(a % b)
                        }
                        (Value::Float(a), Value::Float(b)) => {
                            if b == 0.0 {
                                return Err(runtime_error("Modulo by zero"));
                            }
                            Value::Float(a % b)
                        }
                        (Value::Float(a), Value::Number(b)) => {
                            if b == 0 {
                                return Err(runtime_error("Modulo by zero"));
                            }
                            Value::Float(a % b as f64)
                        }
                        (Value::Number(a), Value::Float(b)) => {
                            if b == 0.0 {
                                return Err(runtime_error("Modulo by zero"));
                            }
                            Value::Float(a as f64 % b)
                        }
                        (l, r) => {
                            return Err(type_error(&format!(
                                "Cannot apply %% to {} and {}",
                                l.type_name(),
                                r.type_name()
                            )));
                        }
                    });
                }
                Opcode::Lt2 => {
                    // pop (b, a) where b is top; result: a < b
                    let b = self.stack.pop()?;
                    let a = self.stack.pop()?;
                    self.stack.push(Value::Bool(match (a, b) {
                        (Value::Number(x), Value::Number(y)) => x < y,
                        (Value::Float(x), Value::Float(y)) => x < y,
                        (Value::Number(x), Value::Float(y)) => (x as f64) < y,
                        (Value::Float(x), Value::Number(y)) => x < y as f64,
                        (l, r) => {
                            return Err(type_error(&format!(
                                "Cannot compare {} < {}",
                                l.type_name(),
                                r.type_name()
                            )));
                        }
                    }));
                }
                Opcode::Concat => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack
                        .push(Value::Str(format!("{}{}", l.display(), r.display())));
                }

                // ── COMPARISON ────────────────────────────────────────
                Opcode::Eq => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(Value::Bool(l.equals(&r)));
                }
                Opcode::Ne => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(Value::Bool(!l.equals(&r)));
                }
                Opcode::Lt => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(Value::Bool(match (l, r) {
                        (Value::Number(a), Value::Number(b)) => a < b,
                        (Value::Float(a), Value::Float(b)) => a < b,
                        (Value::Float(a), Value::Number(b)) => a < b as f64,
                        (Value::Number(a), Value::Float(b)) => (a as f64) < b,
                        (l, r) => {
                            return Err(type_error(&format!(
                                "Cannot compare {} < {}",
                                l.type_name(),
                                r.type_name()
                            )));
                        }
                    }));
                }
                Opcode::Gt => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(Value::Bool(match (l, r) {
                        (Value::Number(a), Value::Number(b)) => a > b,
                        (Value::Float(a), Value::Float(b)) => a > b,
                        (Value::Float(a), Value::Number(b)) => a > b as f64,
                        (Value::Number(a), Value::Float(b)) => (a as f64) > b,
                        (l, r) => {
                            return Err(type_error(&format!(
                                "Cannot compare {} > {}",
                                l.type_name(),
                                r.type_name()
                            )));
                        }
                    }));
                }
                Opcode::Le => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(Value::Bool(match (l, r) {
                        (Value::Number(a), Value::Number(b)) => a <= b,
                        (Value::Float(a), Value::Float(b)) => a <= b,
                        (Value::Float(a), Value::Number(b)) => a <= b as f64,
                        (Value::Number(a), Value::Float(b)) => (a as f64) <= b,
                        (l, r) => {
                            return Err(type_error(&format!(
                                "Cannot compare {} <= {}",
                                l.type_name(),
                                r.type_name()
                            )));
                        }
                    }));
                }
                Opcode::Ge => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(Value::Bool(match (l, r) {
                        (Value::Number(a), Value::Number(b)) => a >= b,
                        (Value::Float(a), Value::Float(b)) => a >= b,
                        (Value::Float(a), Value::Number(b)) => a >= b as f64,
                        (Value::Number(a), Value::Float(b)) => (a as f64) >= b,
                        (l, r) => {
                            return Err(type_error(&format!(
                                "Cannot compare {} >= {}",
                                l.type_name(),
                                r.type_name()
                            )));
                        }
                    }));
                }

                // ── LOGICAL ───────────────────────────────────────────
                Opcode::And => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(Value::Bool(l.is_truthy() && r.is_truthy()));
                }
                Opcode::Or => {
                    let r = self.stack.pop()?;
                    let l = self.stack.pop()?;
                    self.stack.push(Value::Bool(l.is_truthy() || r.is_truthy()));
                }
                Opcode::Not => {
                    let v = self.stack.pop()?;
                    self.stack.push(Value::Bool(!v.is_truthy()));
                }

                // ── CONTROL FLOW ──────────────────────────────────────
                Opcode::Jump(target) => {
                    ip = target;
                }
                Opcode::JumpIfFalse(target) => {
                    if !self.stack.pop()?.is_truthy() {
                        ip = target;
                    }
                }
                Opcode::JumpIfTrue(target) => {
                    if self.stack.pop()?.is_truthy() {
                        ip = target;
                    }
                }

                // ── I/O ───────────────────────────────────────────────
                Opcode::Print => {
                    let v = self.stack.pop()?;
                    println!("{}", v.display());
                }
                Opcode::Input => {
                    use std::io::{self, Write};
                    let mut s = String::new();
                    print!("> ");
                    io::stdout().flush().unwrap();
                    io::stdin().read_line(&mut s).unwrap();
                    let t = s.trim();
                    self.stack.push(if let Ok(n) = t.parse::<i64>() {
                        Value::Number(n)
                    } else {
                        Value::Str(t.to_string())
                    });
                }

                // ── COLLECTIONS ───────────────────────────────────────
                Opcode::MakeArray(n) => {
                    let els = self.stack.pop_n(n)?;
                    self.stack.push(Value::Array(els));
                }
                Opcode::IndexGet => {
                    let idx = self.stack.pop()?;
                    let obj = self.stack.pop()?;
                    let result = match obj {
                        Value::Array(arr) => {
                            if let Value::Number(i) = idx {
                                arr.get(i as usize).cloned().unwrap_or(Value::Nil)
                            } else {
                                return Err(type_error("Array index must be a number"));
                            }
                        }
                        Value::Map(map) => map.get(&idx.display()).cloned().unwrap_or(Value::Nil),
                        Value::Str(s) => {
                            if let Value::Number(i) = idx {
                                s.chars()
                                    .nth(i as usize)
                                    .map(|c| Value::Str(c.to_string()))
                                    .unwrap_or(Value::Nil)
                            } else {
                                return Err(type_error("String index must be a number"));
                            }
                        }
                        other => {
                            return Err(type_error(&format!(
                                "{} is not indexable",
                                other.type_name()
                            )));
                        }
                    };
                    self.stack.push(result);
                }
                Opcode::IndexSet => {
                    let name_val = self.stack.pop()?;
                    let idx = self.stack.pop()?;
                    let new_val = self.stack.pop()?;
                    let name = name_val.display();
                    let key_str = idx.display();
                    let obj = self
                        .env
                        .get(&name)
                        .ok_or_else(|| scope_error(&format!("'{}' is not defined", name)))?;
                    let updated = match obj {
                        Value::Array(mut arr) => {
                            if let Value::Number(i) = idx {
                                let i = i as usize;
                                if i >= arr.len() {
                                    arr.resize(i + 1, Value::Nil);
                                }
                                arr[i] = new_val;
                            } else {
                                return Err(type_error("Array index must be a number"));
                            }
                            Value::Array(arr)
                        }
                        Value::Map(mut map) => {
                            map.insert(key_str, new_val);
                            Value::Map(map)
                        }
                        Value::Struct {
                            name: sn,
                            mut fields,
                        } => {
                            fields.insert(key_str, new_val);
                            Value::Struct { name: sn, fields }
                        }
                        Value::Dotion {
                            id,
                            name: dn,
                            mut fields,
                            methods,
                            handlers,
                            mailbox,
                            brain,
                            tags,
                            tick_priority,
                        } => {
                            fields.insert(key_str, new_val);
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
                            }
                        }
                        other => {
                            return Err(type_error(&format!(
                                "{} is not indexable",
                                other.type_name()
                            )));
                        }
                    };
                    if matches!(updated, Value::Dotion { .. }) {
                        rt.dotions.insert(name.clone(), updated.clone());
                    }
                    self.env.set(&name, updated);
                }
                Opcode::MakeMap(n) => {
                    let mut map = HashMap::new();
                    // pairs were pushed key, value, key, value... pop in reverse
                    let mut flat = Vec::new();
                    for _ in 0..n * 2 {
                        flat.insert(0, self.stack.pop()?);
                    }
                    for chunk in flat.chunks(2) {
                        if let [k, v] = chunk {
                            map.insert(k.display(), v.clone());
                        }
                    }
                    self.stack.push(Value::Map(map));
                }
                Opcode::FieldGet(field) => {
                    let obj = self.stack.pop()?;
                    let val = match &obj {
                        Value::Struct { fields, .. } | Value::Dotion { fields, .. } => {
                            fields.get(&field).cloned().unwrap_or(Value::Nil)
                        }
                        Value::Map(m) => m.get(&field).cloned().unwrap_or(Value::Nil),
                        other => {
                            return Err(type_error(&format!(
                                "Cannot access .{} on {}",
                                field,
                                other.type_name()
                            )));
                        }
                    };
                    self.stack.push(val);
                }
                Opcode::FieldSet(field) => {
                    let name_val = self.stack.pop()?;
                    let new_val = self.stack.pop()?;
                    let name = name_val.display();
                    let obj = self
                        .env
                        .get(&name)
                        .ok_or_else(|| scope_error(&format!("'{}' not defined", name)))?;
                    let updated = match obj {
                        Value::Struct {
                            name: sn,
                            mut fields,
                        } => {
                            fields.insert(field, new_val);
                            Value::Struct { name: sn, fields }
                        }
                        Value::Map(mut m) => {
                            m.insert(field, new_val);
                            Value::Map(m)
                        }
                        Value::Dotion {
                            id,
                            name: dn,
                            mut fields,
                            methods,
                            handlers,
                            mailbox,
                            brain,
                            tags,
                            tick_priority,
                        } => {
                            fields.insert(field, new_val);
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
                            }
                        }
                        other => {
                            return Err(type_error(&format!(
                                "Cannot set field on {}",
                                other.type_name()
                            )));
                        }
                    };
                    // Update registry if it's a dotion
                    if matches!(updated, Value::Dotion { .. }) {
                        rt.dotions.insert(name.clone(), updated.clone());
                    }
                    self.env.set(&name, updated);
                }
                Opcode::MakeStruct(name, n) => {
                    // Stack has n pairs of (name_str, value) pushed interleaved
                    let pairs = self.stack.pop_n(n * 2)?;
                    let mut fields = std::collections::HashMap::new();
                    for chunk in pairs.chunks(2) {
                        if let [Value::Str(k), v] = chunk {
                            fields.insert(k.clone(), v.clone());
                        }
                    }
                    self.stack.push(Value::Struct { name, fields });
                }
                Opcode::MakeEnum(enum_name, variant) => {
                    self.stack.push(Value::EnumVariant { enum_name, variant });
                }

                // ── FUNCTIONS — delegated to Runtime ──────────────────
                Opcode::CallBuiltin(name, argc) => {
                    let args = self.stack.pop_n(argc)?;
                    let result = rt.call_builtin(&name, args, self)?;
                    self.stack.push(result);
                }
                Opcode::Call(name, argc) => {
                    let args = self.stack.pop_n(argc)?;
                    let result = rt.call_function(self, &name, args)?;
                    self.stack.push(result);
                }
                Opcode::Return => break,
                Opcode::MakeFunction(_) => {
                    // Functions are pre-registered before execution starts
                }

                // ── DOP — delegated to Runtime/Scheduler ──────────────
                Opcode::LoadSelf => {
                    let v = self
                        .env
                        .get("__self__")
                        .ok_or_else(|| runtime_error("'self' used outside dotion context"))?;
                    self.stack.push(v);
                }
                Opcode::SendMsg => {
                    let arg = self.stack.pop()?;
                    let msg_val = self.stack.pop()?;
                    let dotion = self.stack.pop()?;
                    rt.send_message(self, dotion, msg_val.display(), arg)?;
                }
                Opcode::CallMethod(name, argc) => {
                    let args = self.stack.pop_n(argc)?;
                    let dotion = self.stack.pop()?;
                    let result = rt.call_method(self, dotion, &name, args)?;
                    self.stack.push(result);
                }
                Opcode::TickStep => {
                    // Signal the DOP scheduler — handled by Runtime
                    rt.tick_step(self)?;
                }
            }
        }

        Ok(())
    }

    /// Print the disassembly of a Chunk (debug tool).
    pub fn disassemble(chunk: &Chunk) {
        let cyan = "\x1b[36m";
        let gray = "\x1b[90m";
        let reset = "\x1b[0m";
        let bold = "\x1b[1m";
        let green = "\x1b[32m";
        println!(
            "\n{}{}=== Bytecode: {} ==={}",
            bold, cyan, chunk.name, reset
        );
        println!(
            "{}  {:>4}  {:<24} {}{}",
            gray, "IDX", "OPCODE", "OPERANDS", reset
        );
        println!("{}  {}{}", gray, "─".repeat(48), reset);
        for (i, op) in chunk.instructions.iter().enumerate() {
            let (name, operands) = match op {
                Opcode::Push(v) => ("PUSH", format!("{:?}", v)),
                Opcode::Pop => ("POP", String::new()),
                Opcode::Dup => ("DUP", String::new()),
                Opcode::Load(n) => ("LOAD", n.clone()),
                Opcode::Store(n) => ("STORE", n.clone()),
                Opcode::Inc(n) => ("INC", n.clone()),
                Opcode::Dec(n) => ("DEC", n.clone()),
                Opcode::Add => ("ADD", String::new()),
                Opcode::Sub => ("SUB", String::new()),
                Opcode::Mul => ("MUL", String::new()),
                Opcode::Div => ("DIV", String::new()),
                Opcode::Mod => ("MOD", String::new()),
                Opcode::Eq => ("EQ", String::new()),
                Opcode::Ne => ("NE", String::new()),
                Opcode::Lt => ("LT", String::new()),
                Opcode::Gt => ("GT", String::new()),
                Opcode::Le => ("LE", String::new()),
                Opcode::Ge => ("GE", String::new()),
                Opcode::And => ("AND", String::new()),
                Opcode::Or => ("OR", String::new()),
                Opcode::Not => ("NOT", String::new()),
                Opcode::Lt2 => ("LT2", String::new()),
                Opcode::Concat => ("CONCAT", String::new()),
                Opcode::Jump(t) => ("JUMP", format!("→ {}", t)),
                Opcode::JumpIfFalse(t) => ("JUMP_FALSE", format!("→ {}", t)),
                Opcode::JumpIfTrue(t) => ("JUMP_TRUE", format!("→ {}", t)),
                Opcode::Print => ("PRINT", String::new()),
                Opcode::Input => ("INPUT", String::new()),
                Opcode::Call(n, a) => ("CALL", format!("{} ({} args)", n, a)),
                Opcode::CallBuiltin(n, a) => ("CALL_BUILTIN", format!("{} ({} args)", n, a)),
                Opcode::Return => ("RETURN", String::new()),
                Opcode::MakeArray(n) => ("MAKE_ARRAY", format!("{} elements", n)),
                Opcode::MakeMap(n) => ("MAKE_MAP", format!("{} pairs", n)),
                Opcode::IndexGet => ("INDEX_GET", String::new()),
                Opcode::IndexSet => ("INDEX_SET", String::new()),
                Opcode::FieldGet(f) => ("FIELD_GET", f.clone()),
                Opcode::FieldSet(f) => ("FIELD_SET", f.clone()),
                Opcode::MakeStruct(n, c) => ("MAKE_STRUCT", format!("{} ({} fields)", n, c)),
                Opcode::MakeEnum(e, v) => ("MAKE_ENUM", format!("{}::{}", e, v)),
                Opcode::PushScope => ("PUSH_SCOPE", String::new()),
                Opcode::PopScope => ("POP_SCOPE", String::new()),
                Opcode::Halt => ("HALT", String::new()),
                Opcode::Nop => ("NOP", String::new()),
                Opcode::TryCatch(t) => ("TRY_CATCH", format!("catch@{}", t)),
                Opcode::TryEnd(t) => ("TRY_END", format!("after@{}", t)),
                Opcode::CatchBind(v) => ("CATCH_BIND", v.clone()),
                Opcode::MakeFunction(n) => ("MAKE_FN", n.clone()),
                Opcode::CallMethod(n, a) => ("CALL_METHOD", format!("{} ({} args)", n, a)),
                Opcode::SendMsg => ("SEND_MSG", String::new()),
                Opcode::LoadSelf => ("LOAD_SELF", String::new()),
                Opcode::TickStep => ("TICK_STEP", String::new()),
            };
            println!(
                "  {}{:>4}{}  {}{:<24}{}  {}{}{}",
                gray, i, reset, green, name, reset, gray, operands, reset
            );
        }
        println!("{}  {}{}", gray, "─".repeat(48), reset);
    }
}

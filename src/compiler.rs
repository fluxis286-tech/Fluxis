// FLUXIS v9.0 — compiler.rs
// Walks the AST and emits bytecode instructions into a Chunk

use crate::ast::{Expr, MatchPattern, Statement};
use crate::error::FluxisError;
use crate::error::runtime_error;
use crate::vm::value::Value;
use crate::vm::{Chunk, Opcode};
use std::collections::HashMap;

pub struct Compiler {
    pub functions: HashMap<String, Chunk>,
    pub struct_types: HashMap<String, Vec<String>>,
    pub enum_types: HashMap<String, Vec<String>>,
    pub dotion_types: std::collections::HashSet<String>,
    pub errors: Vec<crate::error::FluxisError>,
    /// Stack of pending break jump indices per loop level
    loop_breaks: Vec<Vec<usize>>,
    /// Stack of pending continue jump indices per loop level
    loop_continues: Vec<Vec<usize>>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            struct_types: HashMap::new(),
            enum_types: HashMap::new(),
            dotion_types: std::collections::HashSet::new(),
            errors: Vec::new(),
            loop_breaks: Vec::new(),
            loop_continues: Vec::new(),
        }
    }

    /// Compile a full program. Returns main chunk + collects all errors.
    /// If fatal errors exist, Err contains all of them joined.
    pub fn compile(&mut self, program: Vec<Statement>) -> Result<Chunk, crate::error::FluxisError> {
        // Pass 1: register all top-level definitions so forward references work
        for stmt in &program {
            match stmt {
                Statement::StructDef { name, fields } => {
                    self.struct_types.insert(name.clone(), fields.clone());
                }
                Statement::EnumDef { name, variants } => {
                    self.enum_types.insert(name.clone(), variants.clone());
                }
                Statement::DotionDef { name, .. } => {
                    self.dotion_types.insert(name.clone());
                }
                _ => {}
            }
        }

        // Pass 2: compile
        let mut chunk = Chunk::new("main");
        for stmt in program {
            if let Err(e) = self.compile_stmt(&stmt, &mut chunk) {
                self.errors.push(e);
                // Continue compiling to collect more errors
            }
        }
        chunk.emit(Opcode::Halt);

        if !self.errors.is_empty() {
            // Return first error as the main error — caller can read self.errors for all
            return Err(self.errors[0].clone());
        }
        Ok(chunk)
    }

    fn compile_stmt(&mut self, stmt: &Statement, chunk: &mut Chunk) -> Result<(), FluxisError> {
        match stmt {
            Statement::Print { value } => {
                self.compile_expr(value, chunk)?;
                chunk.emit(Opcode::Print);
            }

            Statement::Assignment {
                name,
                type_annotation: _,
                value,
            } => {
                if name == "__discard__" {
                    self.compile_expr(value, chunk)?;
                    chunk.emit(Opcode::Pop);
                } else {
                    self.compile_expr(value, chunk)?;
                    chunk.emit(Opcode::Store(name.clone()));
                }
            }

            Statement::Increment { name } => {
                chunk.emit(Opcode::Inc(name.clone()));
            }

            Statement::Decrement { name } => {
                chunk.emit(Opcode::Dec(name.clone()));
            }

            Statement::Return { value } => {
                self.compile_expr(value, chunk)?;
                chunk.emit(Opcode::Return);
            }

            Statement::Break => {
                let idx = chunk.emit(Opcode::Jump(usize::MAX));
                if let Some(breaks) = self.loop_breaks.last_mut() {
                    breaks.push(idx);
                }
            }

            Statement::Continue => {
                let idx = chunk.emit(Opcode::Jump(usize::MAX));
                if let Some(continues) = self.loop_continues.last_mut() {
                    continues.push(idx);
                }
            }

            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                // Compile condition
                self.compile_expr(condition, chunk)?;

                // JumpIfFalse to else block (placeholder)
                let jf_idx = chunk.emit(Opcode::JumpIfFalse(0));

                // then block
                chunk.emit(Opcode::PushScope);
                for s in then_branch {
                    self.compile_stmt(s, chunk)?;
                }
                chunk.emit(Opcode::PopScope);

                // Jump past else block (placeholder)
                let jmp_idx = chunk.emit(Opcode::Jump(0));

                // Patch JumpIfFalse to here (start of else)
                chunk.patch_jump(jf_idx);

                // else block
                if !else_branch.is_empty() {
                    chunk.emit(Opcode::PushScope);
                    for s in else_branch {
                        self.compile_stmt(s, chunk)?;
                    }
                    chunk.emit(Opcode::PopScope);
                }

                // Patch Jump past else to here
                chunk.patch_jump(jmp_idx);
            }

            Statement::While { condition, body } => {
                // Record loop start position
                let loop_start = chunk.current_pos();

                // Compile condition
                self.compile_expr(condition, chunk)?;

                // JumpIfFalse to exit (placeholder)
                let exit_jump = chunk.emit(Opcode::JumpIfFalse(0));

                // Body
                chunk.emit(Opcode::PushScope);
                self.loop_breaks.push(Vec::new());
                self.loop_continues.push(Vec::new());

                for s in body {
                    self.compile_stmt(s, chunk)?;
                }

                chunk.emit(Opcode::PopScope);
                chunk.emit(Opcode::Jump(loop_start));
                chunk.patch_jump(exit_jump);
                let exit_pos = chunk.current_pos();

                let break_jumps = self.loop_breaks.pop().unwrap_or_default();
                let continue_jumps = self.loop_continues.pop().unwrap_or_default();
                for idx in break_jumps {
                    if let Opcode::Jump(ref mut t) = chunk.instructions[idx] {
                        *t = exit_pos;
                    }
                }
                for idx in continue_jumps {
                    if let Opcode::Jump(ref mut t) = chunk.instructions[idx] {
                        *t = loop_start;
                    }
                }
            }

            Statement::For {
                init,
                condition,
                update,
                body,
            } => {
                chunk.emit(Opcode::PushScope);
                self.compile_stmt(init, chunk)?;

                let loop_start = chunk.current_pos();
                self.compile_expr(condition, chunk)?;
                let exit_jump = chunk.emit(Opcode::JumpIfFalse(0));

                chunk.emit(Opcode::PushScope);
                self.loop_breaks.push(Vec::new());
                self.loop_continues.push(Vec::new());
                for s in body {
                    self.compile_stmt(s, chunk)?;
                }
                chunk.emit(Opcode::PopScope);

                let update_pos = chunk.current_pos();
                self.compile_stmt(update, chunk)?;
                chunk.emit(Opcode::Jump(loop_start));
                chunk.patch_jump(exit_jump);
                let exit_pos = chunk.current_pos();
                chunk.emit(Opcode::PopScope);

                let break_jumps = self.loop_breaks.pop().unwrap_or_default();
                let continue_jumps = self.loop_continues.pop().unwrap_or_default();
                for idx in break_jumps {
                    if let Opcode::Jump(ref mut t) = chunk.instructions[idx] {
                        *t = exit_pos;
                    }
                }
                for idx in continue_jumps {
                    if let Opcode::Jump(ref mut t) = chunk.instructions[idx] {
                        *t = update_pos;
                    }
                }
            }

            Statement::DoWhile { body, condition } => {
                let loop_start = chunk.current_pos();
                chunk.emit(Opcode::PushScope);
                self.loop_breaks.push(Vec::new());
                self.loop_continues.push(Vec::new());
                for s in body {
                    self.compile_stmt(s, chunk)?;
                }
                chunk.emit(Opcode::PopScope);
                self.compile_expr(condition, chunk)?;
                chunk.emit(Opcode::JumpIfTrue(loop_start));
                let exit_pos = chunk.current_pos();
                let break_jumps = self.loop_breaks.pop().unwrap_or_default();
                let continue_jumps = self.loop_continues.pop().unwrap_or_default();
                for idx in break_jumps {
                    if let Opcode::Jump(ref mut t) = chunk.instructions[idx] {
                        *t = exit_pos;
                    }
                }
                for idx in continue_jumps {
                    if let Opcode::Jump(ref mut t) = chunk.instructions[idx] {
                        *t = loop_start;
                    }
                }
            }

            // ── COMPOUND ASSIGN: x += val..  ─────────────────────────
            Statement::CompoundAssign { name, op, value } => {
                // Desugar: x += v  →  LOAD x, compile v, OP, STORE x
                chunk.emit(Opcode::Load(name.clone()));
                self.compile_expr(value, chunk)?;
                let instr = match op.as_str() {
                    "+" => Opcode::Add,
                    "-" => Opcode::Sub,
                    "*" => Opcode::Mul,
                    "/" => Opcode::Div,
                    "%" => Opcode::Mod,
                    _ => return Err(runtime_error(&format!("Unknown compound op '{}'", op))),
                };
                chunk.emit(instr);
                chunk.emit(Opcode::Store(name.clone()));
            }

            // ── FOR-IN ────────────────────────────────────────────────
            Statement::ForIn {
                var,
                iterable,
                body,
            } => {
                // Compile iterable, store in hidden __iter__ var
                self.compile_expr(iterable, chunk)?;
                let iter_var = format!("__iter_{}__", chunk.current_pos());
                let idx_var = format!("__idx_{}__", chunk.current_pos());
                chunk.emit(Opcode::Store(iter_var.clone()));
                // idx = 0
                chunk.emit(Opcode::Push(Value::Number(0)));
                chunk.emit(Opcode::Store(idx_var.clone()));

                let loop_start = chunk.current_pos();
                // Condition: idx < len(iter)
                chunk.emit(Opcode::Load(idx_var.clone()));
                chunk.emit(Opcode::Load(iter_var.clone()));
                chunk.emit(Opcode::CallBuiltin("len".to_string(), 1));
                chunk.emit(Opcode::Lt2); // stack: [idx, len] → idx < len
                let exit_jump = chunk.emit(Opcode::JumpIfFalse(0));

                // var = iter[idx]  (no scope push — item lives in current scope)
                chunk.emit(Opcode::Load(iter_var.clone()));
                chunk.emit(Opcode::Load(idx_var.clone()));
                chunk.emit(Opcode::IndexGet);
                chunk.emit(Opcode::Store(var.clone()));

                self.loop_breaks.push(Vec::new());
                self.loop_continues.push(Vec::new());
                for s in body {
                    self.compile_stmt(s, chunk)?;
                }

                let update_pos = chunk.current_pos();
                chunk.emit(Opcode::Inc(idx_var.clone()));
                chunk.emit(Opcode::Jump(loop_start));
                chunk.patch_jump(exit_jump);
                let exit_pos = chunk.current_pos();

                let break_jumps = self.loop_breaks.pop().unwrap_or_default();
                let continue_jumps = self.loop_continues.pop().unwrap_or_default();
                for idx in break_jumps {
                    if let Opcode::Jump(ref mut t) = chunk.instructions[idx] {
                        *t = exit_pos;
                    }
                }
                for idx in continue_jumps {
                    if let Opcode::Jump(ref mut t) = chunk.instructions[idx] {
                        *t = update_pos;
                    }
                }
            }

            // ── MATCH ─────────────────────────────────────────────────
            Statement::Match { value, arms } => {
                self.compile_expr(value, chunk)?;
                // Store matched value in hidden var
                let match_var = format!("__match_{}__", chunk.current_pos());
                chunk.emit(Opcode::Store(match_var.clone()));

                let mut end_jumps = Vec::new();

                for arm in arms {
                    match &arm.pattern {
                        MatchPattern::Wildcard => {
                            // Always matches — compile body, jump to end
                            chunk.emit(Opcode::PushScope);
                            for s in &arm.body {
                                self.compile_stmt(s, chunk)?;
                            }
                            chunk.emit(Opcode::PopScope);
                            let j = chunk.emit(Opcode::Jump(0));
                            end_jumps.push(j);
                        }
                        MatchPattern::Literal(lit_expr) => {
                            // Load matched, push literal, Eq, JumpIfFalse skip
                            chunk.emit(Opcode::Load(match_var.clone()));
                            self.compile_expr(lit_expr, chunk)?;
                            chunk.emit(Opcode::Eq);
                            let skip = chunk.emit(Opcode::JumpIfFalse(0));
                            chunk.emit(Opcode::PushScope);
                            for s in &arm.body {
                                self.compile_stmt(s, chunk)?;
                            }
                            chunk.emit(Opcode::PopScope);
                            let j = chunk.emit(Opcode::Jump(0));
                            end_jumps.push(j);
                            chunk.patch_jump(skip);
                        }
                        MatchPattern::EnumVariant { enum_name, variant } => {
                            chunk.emit(Opcode::Load(match_var.clone()));
                            chunk.emit(Opcode::MakeEnum(enum_name.clone(), variant.clone()));
                            chunk.emit(Opcode::Eq);
                            let skip = chunk.emit(Opcode::JumpIfFalse(0));
                            chunk.emit(Opcode::PushScope);
                            for s in &arm.body {
                                self.compile_stmt(s, chunk)?;
                            }
                            chunk.emit(Opcode::PopScope);
                            let j = chunk.emit(Opcode::Jump(0));
                            end_jumps.push(j);
                            chunk.patch_jump(skip);
                        }
                        MatchPattern::Struct {
                            type_name,
                            bindings,
                        } => {
                            // match val against struct type name, bind fields
                            chunk.emit(Opcode::Load(match_var.clone()));
                            chunk.emit(Opcode::Push(Value::Str(type_name.clone())));
                            chunk.emit(Opcode::CallBuiltin("__match_struct__".to_string(), 2));
                            let skip = chunk.emit(Opcode::JumpIfFalse(0));
                            chunk.emit(Opcode::PushScope);
                            // Bind each field as a local variable
                            for binding in bindings {
                                chunk.emit(Opcode::Load(match_var.clone()));
                                chunk.emit(Opcode::FieldGet(binding.clone()));
                                chunk.emit(Opcode::Store(binding.clone()));
                            }
                            for s in &arm.body {
                                self.compile_stmt(s, chunk)?;
                            }
                            chunk.emit(Opcode::PopScope);
                            let j = chunk.emit(Opcode::Jump(0));
                            end_jumps.push(j);
                            chunk.patch_jump(skip);
                        }
                    }
                }
                // Patch all end-jumps to here
                for j in end_jumps {
                    chunk.patch_jump(j);
                }
            }

            Statement::FunctionDef {
                name,
                params,
                return_type: _,
                body,
            } => {
                let mut fn_chunk = Chunk::new(&name);
                // Bind parameters in reverse (last pushed = first bound)
                for param in params.iter().rev() {
                    // variadic and regular params both just pop from stack
                    // The difference is at call site — variadic collects extra args
                    fn_chunk.emit(Opcode::Store(param.name.clone()));
                }
                // Bind default values for params that got nil
                for param in params.iter() {
                    if let Some(ref default_expr) = param.default {
                        // if param == nil, use default
                        fn_chunk.emit(Opcode::Load(param.name.clone()));
                        fn_chunk.emit(Opcode::Push(Value::Nil));
                        fn_chunk.emit(Opcode::Eq);
                        let skip = fn_chunk.emit(Opcode::JumpIfFalse(0));
                        self.compile_expr(default_expr, &mut fn_chunk)?;
                        fn_chunk.emit(Opcode::Store(param.name.clone()));
                        fn_chunk.patch_jump(skip);
                    }
                }
                for s in body {
                    self.compile_stmt(s, &mut fn_chunk)?;
                }
                fn_chunk.emit(Opcode::Push(Value::Nil));
                fn_chunk.emit(Opcode::Return);
                self.functions.insert(name.clone(), fn_chunk);
            }

            Statement::StructDef { name, fields } => {
                // Already registered in pass 1 — no bytecode needed
                // Validate no duplicate field names
                let mut seen = std::collections::HashSet::new();
                for f in fields {
                    if !seen.insert(f.clone()) {
                        self.errors.push(crate::error::compile_error(
                            &format!("Struct '{}' has duplicate field '{}'", name, f),
                            0,
                            0,
                        ));
                    }
                }
            }

            Statement::EnumDef { name, variants } => {
                // Already registered in pass 1 — no bytecode needed
                let mut seen = std::collections::HashSet::new();
                for v in variants {
                    if !seen.insert(v.clone()) {
                        self.errors.push(crate::error::compile_error(
                            &format!("Enum '{}' has duplicate variant '{}'", name, v),
                            0,
                            0,
                        ));
                    }
                }
            }

            Statement::Import { module } => {
                // Emit a runtime import call — Runtime handles file loading
                chunk.emit(Opcode::Push(Value::Str(module.clone())));
                chunk.emit(Opcode::CallBuiltin("__import__".to_string(), 1));
            }

            Statement::DotionDef {
                name,
                fields,
                methods,
                handlers,
                brain,
                extends,
                tags,
                tick_priority,
            } => {
                // Register the dotion type definition as a special function chunk.
                // When instantiated, Runtime looks up "__doptype__Name" and builds a Value::Dotion.
                // We store metadata as a Push of a serialized definition — Runtime deserializes it.
                let mut def_chunk = Chunk::new(&format!("__doptype__{}", name));
                // Push field default values with their names interleaved
                for (fname, fexpr) in fields {
                    def_chunk.emit(Opcode::Push(Value::Str(fname.to_string())));
                    self.compile_expr(&fexpr.clone(), &mut def_chunk)?;
                }
                def_chunk.emit(Opcode::Push(Value::Number(fields.len() as i64)));
                def_chunk.emit(Opcode::Return);
                self.functions
                    .insert(format!("__doptype__{}", name), def_chunk);

                // Also store methods, handlers, brain, tags, tick_priority in runtime metadata
                // by encoding them as a special registration chunk
                let mut reg_chunk = Chunk::new(&format!("__dopreg__{}", name));
                reg_chunk.emit(Opcode::Push(Value::Str(name.clone())));
                reg_chunk.emit(Opcode::Push(Value::Str(
                    extends.clone().unwrap_or_default(),
                )));
                reg_chunk.emit(Opcode::Push(Value::Str(brain.clone().unwrap_or_default())));
                reg_chunk.emit(Opcode::Push(Value::Array(
                    tags.iter().map(|t| Value::Str(t.clone())).collect(),
                )));
                reg_chunk.emit(Opcode::Push(Value::Number(*tick_priority)));
                reg_chunk.emit(Opcode::Return);
                self.functions
                    .insert(format!("__dopreg__{}", name), reg_chunk);

                // Compile each method as a standalone function "__dopmethod__TypeName__methodname"
                for method in methods {
                    let fn_name = format!("__dopmethod__{}_{}", name, method.name);
                    let mut mchunk = Chunk::new(&fn_name);
                    // params arrive on stack in order, bind in reverse
                    for p in method.params.iter().rev() {
                        mchunk.emit(Opcode::Store(p.clone()));
                    }
                    for s in method.body.clone() {
                        self.compile_stmt(&s, &mut mchunk)?;
                    }
                    mchunk.emit(Opcode::Push(Value::Nil));
                    mchunk.emit(Opcode::Return);
                    self.functions.insert(fn_name, mchunk);
                }

                // Compile each handler as "__dophandler__TypeName__msgname"
                for handler in handlers {
                    let fn_name = format!("__dophandler__{}_{}", name, handler.msg);
                    let mut hchunk = Chunk::new(&fn_name);
                    if let Some(ref param) = handler.param {
                        hchunk.emit(Opcode::Store(param.clone()));
                    }
                    for s in handler.body.clone() {
                        self.compile_stmt(&s, &mut hchunk)?;
                    }
                    hchunk.emit(Opcode::Push(Value::Nil));
                    hchunk.emit(Opcode::Return);
                    self.functions.insert(fn_name, hchunk);
                }
            }

            Statement::ActorDef { name, methods } => {
                for method in methods {
                    let fn_name = format!("__actomethod__{}_{}", name, method.name);
                    let mut mchunk = Chunk::new(&fn_name);
                    for p in method.params.iter().rev() {
                        mchunk.emit(Opcode::Store(p.clone()));
                    }
                    for s in method.body.clone() {
                        self.compile_stmt(&s, &mut mchunk)?;
                    }
                    mchunk.emit(Opcode::Push(Value::Nil));
                    mchunk.emit(Opcode::Return);
                    self.functions.insert(fn_name, mchunk);
                }
            }

            Statement::TryCatch {
                try_body,
                catch_var,
                catch_body,
            } => {
                // TryCatch(catch_addr) — if any error in try block, jump to catch
                let try_op = chunk.emit(Opcode::TryCatch(0)); // patched below

                // Try body
                for s in try_body {
                    self.compile_stmt(s, chunk)?;
                }

                // TryEnd(after_catch) — success path: skip catch block
                let end_op = chunk.emit(Opcode::TryEnd(0)); // patched below

                // Catch block
                let catch_start = chunk.current_pos();
                chunk.emit(Opcode::CatchBind(catch_var.clone()));
                for s in catch_body {
                    self.compile_stmt(s, chunk)?;
                }

                let after_catch = chunk.current_pos();

                // Patch TryCatch → catch_start
                if let Opcode::TryCatch(ref mut t) = chunk.instructions[try_op] {
                    *t = catch_start;
                }
                // Patch TryEnd → after_catch
                if let Opcode::TryEnd(ref mut t) = chunk.instructions[end_op] {
                    *t = after_catch;
                }
            }

            Statement::TickBlock { body } => {
                // Store the tick block body as a special function "__tick_block__"
                let mut tchunk = Chunk::new("__tick_block__");
                for s in body {
                    self.compile_stmt(&s, &mut tchunk)?;
                }
                tchunk.emit(Opcode::Push(Value::Nil));
                tchunk.emit(Opcode::Return);
                self.functions.insert("__tick_block__".to_string(), tchunk);
            }

            Statement::TickRun { count } => {
                self.compile_expr(count, chunk)?;
                chunk.emit(Opcode::CallBuiltin("__tick_run__".to_string(), 1));
            }

            Statement::SelfFieldAssign { field, value } => {
                self.compile_expr(value, chunk)?;
                chunk.emit(Opcode::CallBuiltin(format!("__self_set__{}", field), 1));
            }

            Statement::SelfCompoundAssign { field, op, value } => {
                // Load current self.field, apply op, store back
                chunk.emit(Opcode::CallBuiltin(format!("__self_get__{}", field), 0));
                self.compile_expr(value, chunk)?;
                match op.as_str() {
                    "+" => {
                        chunk.emit(Opcode::Add);
                    }
                    "-" => {
                        chunk.emit(Opcode::Sub);
                    }
                    "*" => {
                        chunk.emit(Opcode::Mul);
                    }
                    "/" => {
                        chunk.emit(Opcode::Div);
                    }
                    "%" => {
                        chunk.emit(Opcode::Mod);
                    }
                    _ => {}
                };
                chunk.emit(Opcode::CallBuiltin(format!("__self_set__{}", field), 1));
            }

            Statement::MethodCallStmt {
                object,
                method,
                args,
            } => {
                for a in args {
                    self.compile_expr(&a.clone(), chunk)?;
                }
                chunk.emit(Opcode::Push(Value::Str(object.clone())));
                chunk.emit(Opcode::CallBuiltin(
                    format!("__dopcall__{}", method),
                    args.len() + 1,
                ));
            }

            Statement::IndexAssign {
                object,
                index,
                value,
            } => {
                self.compile_expr(value, chunk)?;
                self.compile_expr(index, chunk)?;
                chunk.emit(Opcode::Push(Value::Str(object.clone())));
                chunk.emit(Opcode::IndexSet);
            }

            Statement::FieldAssign {
                object,
                field,
                value,
            } => {
                self.compile_expr(value, chunk)?;
                chunk.emit(Opcode::Push(Value::Str(object.clone())));
                chunk.emit(Opcode::FieldSet(field.clone()));
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr, chunk: &mut Chunk) -> Result<(), FluxisError> {
        match expr {
            Expr::Number(n) => {
                chunk.emit(Opcode::Push(Value::Number(*n)));
            }
            Expr::String(s) => {
                chunk.emit(Opcode::Push(Value::Str(s.clone())));
            }
            Expr::Bool(b) => {
                chunk.emit(Opcode::Push(Value::Bool(*b)));
            }
            Expr::Identifier(n) => {
                chunk.emit(Opcode::Load(n.clone()));
            }
            Expr::Input => {
                chunk.emit(Opcode::Input);
            }

            Expr::Array(elements) => {
                for e in elements {
                    self.compile_expr(e, chunk)?;
                }
                chunk.emit(Opcode::MakeArray(elements.len()));
            }

            Expr::Map(pairs) => {
                for (k, v) in pairs {
                    self.compile_expr(k, chunk)?;
                    self.compile_expr(v, chunk)?;
                }
                chunk.emit(Opcode::MakeMap(pairs.len()));
            }

            Expr::Index { object, index } => {
                self.compile_expr(object, chunk)?;
                self.compile_expr(index, chunk)?;
                chunk.emit(Opcode::IndexGet);
            }

            Expr::Field { object, field } => {
                self.compile_expr(object, chunk)?;
                chunk.emit(Opcode::FieldGet(field.clone()));
            }

            Expr::StructInit { name, fields } => {
                // If this is a dotion type, instantiate it
                if self.dotion_types.contains(name)
                    || self.functions.contains_key(&format!("__doptype__{}", name))
                {
                    for (fname, fv) in fields {
                        chunk.emit(Opcode::Push(Value::Str(fname.clone())));
                        self.compile_expr(fv, chunk)?;
                    }
                    chunk.emit(Opcode::Push(Value::Number(fields.len() as i64)));
                    chunk.emit(Opcode::Push(Value::Str(name.clone())));
                    chunk.emit(Opcode::CallBuiltin(
                        "__dotion_new__".to_string(),
                        fields.len() * 2 + 2,
                    ));
                } else {
                    // Regular struct — validate fields if type is known
                    if let Some(known_fields) = self.struct_types.get(name).cloned() {
                        for (fname, _) in fields.iter() {
                            if !known_fields.contains(fname) {
                                self.errors.push(crate::error::compile_error(
                                    &format!(
                                        "Struct '{}' has no field '{}'. Known fields: {}",
                                        name,
                                        fname,
                                        known_fields.join(", ")
                                    ),
                                    0,
                                    0,
                                ));
                            }
                        }
                    }
                    for (fname, fv) in fields {
                        chunk.emit(Opcode::Push(Value::Str(fname.clone())));
                        self.compile_expr(fv, chunk)?;
                    }
                    chunk.emit(Opcode::MakeStruct(name.clone(), fields.len()));
                }
            }

            Expr::EnumVariant { enum_name, variant } => {
                chunk.emit(Opcode::MakeEnum(enum_name.clone(), variant.clone()));
            }
            Expr::Nil => {
                chunk.emit(Opcode::Push(Value::Nil));
            }
            Expr::Float(f) => {
                chunk.emit(Opcode::Push(crate::vm::Value::Float(*f)));
            }
            Expr::DotionLit { .. } => {
                chunk.emit(Opcode::Push(crate::vm::Value::Nil));
            }

            // Closure: fn(params) { body } — stored as a named function chunk
            Expr::Closure { params, body } => {
                let fn_name = format!("__closure_{}_{}", chunk.name, chunk.current_pos());
                let mut cl_chunk = Chunk::new(&fn_name);
                for param in params.iter().rev() {
                    cl_chunk.emit(Opcode::Store(param.name.clone()));
                }
                for param in params.iter() {
                    if let Some(ref default_expr) = param.default {
                        cl_chunk.emit(Opcode::Load(param.name.clone()));
                        cl_chunk.emit(Opcode::Push(Value::Nil));
                        cl_chunk.emit(Opcode::Eq);
                        let skip = cl_chunk.emit(Opcode::JumpIfFalse(0));
                        self.compile_expr(default_expr, &mut cl_chunk)?;
                        cl_chunk.emit(Opcode::Store(param.name.clone()));
                        cl_chunk.patch_jump(skip);
                    }
                }
                for s in body {
                    self.compile_stmt(&s, &mut cl_chunk)?;
                }
                cl_chunk.emit(Opcode::Push(Value::Nil));
                cl_chunk.emit(Opcode::Return);
                self.functions.insert(fn_name.clone(), cl_chunk);
                // Push closure name as a string value so it can be called
                chunk.emit(Opcode::Push(Value::Str(fn_name)));
            }

            // CallExpr: call a closure stored in a variable
            Expr::CallExpr { callee, args } => {
                for a in args {
                    self.compile_expr(a, chunk)?;
                }
                self.compile_expr(callee, chunk)?;
                chunk.emit(Opcode::CallBuiltin(
                    "__call_closure__".to_string(),
                    args.len() + 1,
                ));
            }

            // String interpolation: "Hello {name}!"
            Expr::InterpolatedStr(segments) => {
                // Build result by concatenating segments
                let mut first = true;
                for (literal, expr_opt) in segments {
                    if !literal.is_empty() {
                        chunk.emit(Opcode::Push(Value::Str(literal.clone())));
                        if !first {
                            chunk.emit(Opcode::Add);
                        }
                        first = false;
                    }
                    if let Some(expr) = expr_opt {
                        self.compile_expr(expr, chunk)?;
                        chunk.emit(Opcode::CallBuiltin("to_str".to_string(), 1));
                        if !first {
                            chunk.emit(Opcode::Add);
                        }
                        first = false;
                    }
                }
                if first {
                    chunk.emit(Opcode::Push(Value::Str(String::new())));
                }
            }

            // Range: start..end or start..end..step
            Expr::Range { start, end, step } => {
                self.compile_expr(start, chunk)?;
                self.compile_expr(end, chunk)?;
                if let Some(s) = step {
                    self.compile_expr(s, chunk)?;
                    chunk.emit(Opcode::CallBuiltin("range".to_string(), 3));
                } else {
                    chunk.emit(Opcode::CallBuiltin("range".to_string(), 2));
                }
            }

            // Optional chain: obj?.field
            Expr::OptionalChain { object, field } => {
                self.compile_expr(object, chunk)?;
                chunk.emit(Opcode::Push(Value::Str(field.clone())));
                chunk.emit(Opcode::CallBuiltin("__optional_chain__".to_string(), 2));
            }

            // Null coalesce: left ?? right
            Expr::NullCoalesce { left, right } => {
                self.compile_expr(left, chunk)?;
                chunk.emit(Opcode::Dup);
                chunk.emit(Opcode::Push(Value::Nil));
                chunk.emit(Opcode::Eq);
                let skip = chunk.emit(Opcode::JumpIfFalse(0));
                chunk.emit(Opcode::Pop); // discard the nil
                self.compile_expr(right, chunk)?;
                chunk.patch_jump(skip);
            }

            // in operator: val in collection
            Expr::In {
                value,
                collection,
                negated,
            } => {
                self.compile_expr(value, chunk)?;
                self.compile_expr(collection, chunk)?;
                chunk.emit(Opcode::CallBuiltin("__in__".to_string(), 2));
                if *negated {
                    chunk.emit(Opcode::Not);
                }
            }

            // Await: just evaluate the expression (async is Future work)
            Expr::Await(inner) => {
                self.compile_expr(inner, chunk)?;
            }
            Expr::Self_ => {
                // Load __self__ from env — set by Runtime during method/handler calls
                chunk.emit(Opcode::Load("__self__".to_string()));
            }
            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                // Push args, then object identifier name, call via builtin dispatcher
                for a in args {
                    self.compile_expr(a, chunk)?;
                }
                if let Expr::Identifier(obj_name) = object.as_ref() {
                    chunk.emit(Opcode::Push(Value::Str(obj_name.clone())));
                    chunk.emit(Opcode::CallBuiltin(
                        format!("__dopcall__{}", method),
                        args.len() + 1,
                    ));
                } else {
                    chunk.emit(Opcode::Push(crate::vm::Value::Nil));
                }
            }

            Expr::Call { name, args } => {
                for a in args {
                    self.compile_expr(a, chunk)?;
                }
                if Self::is_builtin(&name) {
                    chunk.emit(Opcode::CallBuiltin(name.clone(), args.len()));
                } else {
                    chunk.emit(Opcode::Call(name.clone(), args.len()));
                }
            }

            Expr::Unary { op, expr } => {
                if op == "!" {
                    self.compile_expr(expr, chunk)?;
                    chunk.emit(Opcode::Not);
                } else if op == "-" {
                    // compile as 0 - expr
                    chunk.emit(Opcode::Push(Value::Number(0)));
                    self.compile_expr(expr, chunk)?;
                    chunk.emit(Opcode::Sub);
                }
            }

            Expr::Binary { left, op, right } => {
                // Short-circuit: && and || need special handling
                match op.as_str() {
                    "&&" => {
                        self.compile_expr(left, chunk)?;
                        chunk.emit(Opcode::Dup);
                        let jf = chunk.emit(Opcode::JumpIfFalse(0));
                        chunk.emit(Opcode::Pop);
                        self.compile_expr(right, chunk)?;
                        chunk.patch_jump(jf);
                    }
                    "||" => {
                        self.compile_expr(left, chunk)?;
                        chunk.emit(Opcode::Dup);
                        let jt = chunk.emit(Opcode::JumpIfTrue(0));
                        chunk.emit(Opcode::Pop);
                        self.compile_expr(right, chunk)?;
                        chunk.patch_jump(jt);
                    }
                    "+" => {
                        self.compile_expr(left, chunk)?;
                        self.compile_expr(right, chunk)?;
                        chunk.emit(Opcode::Add);
                    }
                    _ => {
                        self.compile_expr(left, chunk)?;
                        self.compile_expr(right, chunk)?;
                        let instr = match op.as_str() {
                            "-" => Opcode::Sub,
                            "*" => Opcode::Mul,
                            "/" => Opcode::Div,
                            "==" => Opcode::Eq,
                            "!=" => Opcode::Ne,
                            "<" => Opcode::Lt,
                            ">" => Opcode::Gt,
                            "<=" => Opcode::Le,
                            ">=" => Opcode::Ge,
                            "%" => Opcode::Mod,
                            _ => return Err(runtime_error(&format!("Unknown op: {}", op))),
                        };
                        chunk.emit(instr);
                    }
                }
            }
        }
        Ok(())
    }

    /// Returns true if a function name should be emitted as CallBuiltin
    /// rather than Call (user-defined function lookup).
    fn is_builtin(name: &str) -> bool {
        matches!(
            name,
            "len"
                | "push"
                | "pop"
                | "keys"
                | "has"
                | "del"
                | "type_of"
                | "to_str"
                | "to_num"
                | "to_float"
                | "tick_count"
                | "send"
                | "broadcast"
                | "broadcast_to"
                | "format"
                | "range"
                | "sort_arr"
                | "sort_desc"
                | "remove"
                | "insert"
                | "slice"
                | "flatten"
                | "reverse"
                | "zip"
                | "assert"
                | "map_fn"
                | "filter_fn"
                | "reduce_fn"
                | "any_fn"
                | "all_fn"
                | "dotion_list"
                | "dotion_count"
                | "dotion_where"
                | "dotion_where_fn"
                | "clone"
                | "send_self"
                | "is_num"
                | "is_float"
                | "is_str"
                | "is_bool"
                | "is_array"
                | "is_map"
                | "is_nil"
                | "__dotion_new__"
                | "__tick_run__"
                | "__import__"
                | "__call_closure__"
                | "__optional_chain__"
                | "__in__"
                | "__collect_variadic__"
                | "__match_struct__"
                | "__try_start__"
                | "__try_end__"
                | "__catch_start__"
                | "__catch_end__"
        ) || name.starts_with("__self_set__")
            || name.starts_with("__self_get__")
            || name.starts_with("__dopcall__")
            || crate::stdlib::is_math_fn(name)
            || crate::stdlib::is_string_fn(name)
            || crate::stdlib::is_io_fn(name)
            || crate::stdlib::is_ml_fn(name)
            || crate::stdlib::is_ai_fn(name)
            || crate::stdlib::is_gfx_fn(name)
    }
}

// FLUXIS — interpreter.rs (v10)
// Modules: math, string, io, ai (LLM), ml (machine learning), gfx (2D graphics)
use std::collections::HashMap;
#[allow(unused_imports)]
use crate::ast::{Expr, Statement, TypeAnnotation, Handler, DotionMethod, MatchPattern};
use crate::dop::{new_id, DotionTypeDef, ActorTypeDef, TickEngine};
use crate::error::{FluxisError, runtime_error, type_error, scope_error, arity_error};
use crate::stdlib;

// ── VALUE ─────────────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
pub enum Value {
    Number(i64), Float(f64), Str(String), Bool(bool), Nil,
    Array(Vec<Value>), Map(HashMap<String, Value>),
    Struct { name: String, fields: HashMap<String, Value> },
    EnumVariant { enum_name: String, variant: String },
    Dotion {
        id: u64, name: String,
        fields: HashMap<String, Value>,
        methods: Vec<DotionMethod>,
        handlers: Vec<Handler>,
        mailbox: Vec<(String, Value)>,
        brain: Option<String>,
        tags: Vec<String>,
        tick_priority: i64,
    },
}

// ── STORED DEFS ───────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
struct FunctionDef { params: Vec<(String, Option<TypeAnnotation>)>, return_type: Option<TypeAnnotation>, body: Vec<Statement> }
#[derive(Clone, Debug)]
struct StructDef { fields: Vec<String> }
#[derive(Clone, Debug)]
struct EnumDef   { variants: Vec<String> }

// ── CONTROL FLOW ─────────────────────────────────────────────────────────
enum ControlFlow { None, Return(Value), Break, Continue, Error(FluxisError) }

// ── SCOPE ─────────────────────────────────────────────────────────────────
pub struct Scope { pub stack: Vec<HashMap<String, Value>> }
impl Scope {
    fn new() -> Self { Self { stack: vec![HashMap::new()] } }
    fn push(&mut self) { self.stack.push(HashMap::new()); }
    fn pop(&mut self)  { if self.stack.len()>1 { self.stack.pop(); } }
    pub fn get(&self, n: &str) -> Option<Value> {
        for f in self.stack.iter().rev() { if let Some(v)=f.get(n){return Some(v.clone());} }
        None
    }
    fn set(&mut self, n: &str, v: Value) {
        for f in self.stack.iter_mut().rev() { if f.contains_key(n){f.insert(n.to_string(),v);return;} }
        if let Some(f)=self.stack.last_mut(){f.insert(n.to_string(),v);}
    }
    fn define(&mut self, n: &str, v: Value) {
        if let Some(f)=self.stack.last_mut(){f.insert(n.to_string(),v);}
    }
    fn get_mut(&mut self, n: &str) -> Option<&mut Value> {
        for f in self.stack.iter_mut().rev(){if f.contains_key(n){return f.get_mut(n);}}
        None
    }
    pub fn all_names(&self) -> Vec<String> {
        let mut ns=Vec::new();
        for f in &self.stack{for k in f.keys(){if !ns.contains(k){ns.push(k.clone());}}}
        ns
    }
}

// ── INTERPRETER ───────────────────────────────────────────────────────────
pub struct Interpreter {
    pub scope: Scope,
    functions:    HashMap<String, FunctionDef>,
    dotion_types: HashMap<String, DotionTypeDef>,
    actor_types:  HashMap<String, ActorTypeDef>,
    structs:      HashMap<String, StructDef>,
    enums:        HashMap<String, EnumDef>,
    tick_engine:  TickEngine,
    loaded_mods:  Vec<String>,
    // GFX terminal canvas state
    canvas:       Option<Vec<Vec<char>>>,
    canvas_w:     usize,
    canvas_h:     usize,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            scope:Scope::new(), functions:HashMap::new(),
            dotion_types:HashMap::new(), actor_types:HashMap::new(),
            structs:HashMap::new(), enums:HashMap::new(),
            tick_engine:TickEngine::new(), loaded_mods:Vec::new(),
            canvas:None, canvas_w:0, canvas_h:0,
        }
    }

    pub fn execute(&mut self, program: Vec<Statement>) -> Result<(),FluxisError> {
        for stmt in program {
            if let ControlFlow::Error(e)=self.execute_stmt(stmt){return Err(e);}
        }
        Ok(())
    }

    fn execute_stmt(&mut self, stmt: Statement) -> ControlFlow {
        match stmt {
            Statement::Import{module}=>{
                // .fx file import: import "myfile.fx"..
                if module.ends_with(".fx") {
                    if self.loaded_mods.contains(&module) {
                        return ControlFlow::None; // already loaded, skip
                    }
                    let source = match std::fs::read_to_string(&module) {
                        Ok(s) => s,
                        Err(e) => return ControlFlow::Error(
                            runtime_error(&format!("import \"{}\": {}", module, e))
                                .with_hint("Check the file path is correct relative to where you run fluxis")),
                    };
                    self.loaded_mods.push(module.clone());
                    use crate::lexer::Lexer;
                    use crate::parser::Parser;
                    let tokens = match Lexer::new(&source).lex() {
                        Ok(t) => t,
                        Err(e) => return ControlFlow::Error(e),
                    };
                    let program = match Parser::new(tokens, &source).parse() {
                        Ok(p) => p,
                        Err(e) => return ControlFlow::Error(e),
                    };
                    // Execute all top-level definitions (fn, struct, enum, dotion, actor)
                    // but do NOT run start{} blocks from imported files
                    for stmt in program {
                        match &stmt {
                            Statement::FunctionDef{..}
                            | Statement::StructDef{..}
                            | Statement::EnumDef{..}
                            | Statement::DotionDef{..}
                            | Statement::ActorDef{..}
                            | Statement::Import{..} => {
                                match self.execute_stmt(stmt) {
                                    ControlFlow::Error(e) => return ControlFlow::Error(e),
                                    _ => {}
                                }
                            }
                            _ => {} // skip start{}, expressions, etc.
                        }
                    }
                    return ControlFlow::None;
                }
                // stdlib module import
                if stdlib::load_module(&module).is_some(){
                    if !self.loaded_mods.contains(&module){self.loaded_mods.push(module);}
                }else{
                    return ControlFlow::Error(runtime_error(
                        &format!("Unknown module '{}'. Available: \"math\",\"string\",\"io\",\"ai\",\"ml\",\"gfx\", or a .fx file path",module))
                        .with_hint("Use import \"myfile.fx\"; for local files, or import \"math\"; for stdlib"));
                }
                ControlFlow::None
            }
            Statement::StructDef{name,fields}=>{self.structs.insert(name,StructDef{fields});ControlFlow::None}
            Statement::EnumDef{name,variants}=>{self.enums.insert(name,EnumDef{variants});ControlFlow::None}
            Statement::FunctionDef{name,params,return_type,body}=>{self.functions.insert(name,FunctionDef{params,return_type,body});ControlFlow::None}
            Statement::DotionDef{name,fields,methods,handlers,brain,extends,tags,tick_priority}=>{
                // Inheritance: merge parent's fields/methods/handlers first
                let (mut merged_fields, mut merged_methods, mut merged_handlers) =
                    if let Some(ref parent_name) = extends {
                        if let Some(parent) = self.dotion_types.get(parent_name).cloned() {
                            (parent.fields, parent.methods, parent.handlers)
                        } else {
                            return ControlFlow::Error(runtime_error(
                                &format!("dotion '{}' extends '{}' but '{}' is not defined",
                                    name, parent_name, parent_name))
                                .with_hint(&format!("Define '{}' before '{}'", parent_name, name)));
                        }
                    } else {
                        (Vec::new(), Vec::new(), Vec::new())
                    };
                // Child fields override parent fields with same name
                for (fname, fexpr) in fields {
                    merged_fields.retain(|(k,_)| k != &fname);
                    merged_fields.push((fname, fexpr));
                }
                // Child methods override parent methods with same name
                for method in methods {
                    merged_methods.retain(|m: &DotionMethod| m.name != method.name);
                    merged_methods.push(method);
                }
                // Child handlers override parent handlers for same message
                for handler in handlers {
                    merged_handlers.retain(|h: &Handler| h.msg != handler.msg);
                    merged_handlers.push(handler);
                }
                self.dotion_types.insert(name, DotionTypeDef{
                    fields: merged_fields,
                    methods: merged_methods,
                    handlers: merged_handlers,
                    brain,
                    extends,
                    tags,
                    tick_priority,
                });
                ControlFlow::None
            }
            Statement::ActorDef{name,methods}=>{self.actor_types.insert(name,ActorTypeDef{methods});ControlFlow::None}
            Statement::TickBlock{body}=>{self.tick_engine.set_block(body);ControlFlow::None}
            Statement::TickRun{count}=>{
                let n=match self.eval_expr(count){
                    Ok(Value::Number(n))=>n,
                    Ok(other)=>return ControlFlow::Error(type_error(&format!("tick() expects num, got {}",self.type_name(&other)))),
                    Err(e)=>return ControlFlow::Error(e),
                };
                let tick_body=match self.tick_engine.tick_block.clone(){
                    Some(b)=>b,
                    None=>return ControlFlow::Error(runtime_error("tick(n) called but no tick block defined").with_hint("Define: tick { ... }")),
                };
                for _ in 0..n {
                    // Sort dotion names by tick_priority before each phase
                    let mut names=self.scope.all_names();
                    names.sort_by_key(|n|match self.scope.get(n){
                        Some(Value::Dotion{tick_priority,..})=>tick_priority,
                        _=>0,
                    });
                    if let Err(e)=self.process_mailboxes_ordered(&names){return ControlFlow::Error(e);}
                    if let Err(e)=self.run_actor_brains(){return ControlFlow::Error(e);}
                    // Flush any messages queued by actor brains (e.g. send_self) in same tick
                    let mut names2=self.scope.all_names();
                    names2.sort_by_key(|n|match self.scope.get(n){
                        Some(Value::Dotion{tick_priority,..})=>tick_priority,
                        _=>0,
                    });
                    if let Err(e)=self.process_mailboxes_ordered(&names2){return ControlFlow::Error(e);}
                    self.scope.push();
                    for s in tick_body.clone(){match self.execute_stmt(s){ControlFlow::None=>{}other=>{self.scope.pop();return other;}}}
                    self.scope.pop();
                    self.tick_engine.advance();
                }
                ControlFlow::None
            }
            Statement::Return{value}=>match self.eval_expr(value){Ok(v)=>ControlFlow::Return(v),Err(e)=>ControlFlow::Error(e)}
            Statement::Break=>ControlFlow::Break,
            Statement::Continue=>ControlFlow::Continue,
            Statement::Increment{name}=>{
                let c=self.scope.get(&name).unwrap_or(Value::Number(0));
                match c{Value::Number(n)=>{self.scope.set(&name,Value::Number(n+1));ControlFlow::None}
                other=>ControlFlow::Error(type_error(&format!("Cannot increment '{}': expected num, got {}",name,self.type_name(&other))))}
            }
            Statement::Decrement{name}=>{
                let c=self.scope.get(&name).unwrap_or(Value::Number(0));
                match c{Value::Number(n)=>{self.scope.set(&name,Value::Number(n-1));ControlFlow::None}
                other=>ControlFlow::Error(type_error(&format!("Cannot decrement '{}': expected num, got {}",name,self.type_name(&other))))}
            }
            Statement::CompoundAssign{name,op,value}=>{
                let rhs=match self.eval_expr(value){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                let lhs=match self.scope.get(&name){
                    Some(v)=>v,
                    None=>return ControlFlow::Error(scope_error(&format!("'{}' is not defined",name)).with_hint(&format!("Declare it first: {} = <value>;",name))),
                };
                let result=match self.apply_op(&lhs,&op,&rhs){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                self.scope.set(&name,result);
                ControlFlow::None
            }
            Statement::SelfCompoundAssign{field,op,value}=>{
                let rhs=match self.eval_expr(value){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                let lhs=match self.scope.get("__self__"){
                    Some(Value::Dotion{ref fields,..})=>fields.get(&field).cloned().unwrap_or(Value::Nil),
                    _=>return ControlFlow::Error(runtime_error("'self' used outside of a dotion method or handler")),
                };
                let result=match self.apply_op(&lhs,&op,&rhs){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                match self.scope.get_mut("__self__"){
                    Some(Value::Dotion{fields,..})=>{fields.insert(field,result);}
                    _=>return ControlFlow::Error(runtime_error("'self' used outside of a dotion method or handler")),
                }
                ControlFlow::None
            }
            Statement::ForIn{var,iterable,body}=>{
                let iter_val=match self.eval_expr(iterable){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                let items:Vec<Value>=match iter_val{
                    Value::Array(a)=>a,
                    Value::Str(s)=>s.chars().map(|c|Value::Str(c.to_string())).collect(),
                    Value::Map(m)=>m.into_keys().map(Value::Str).collect(),
                    other=>return ControlFlow::Error(type_error(&format!("Cannot iterate over {}",self.type_name(&other))).with_hint("for-in works on arrays, strings, and maps")),
                };
                'forin:for item in items {
                    self.scope.push();
                    self.scope.define(&var,item);
                    let mut brk=false;
                    for s in body.clone(){
                        match self.execute_stmt(s){
                            ControlFlow::None=>{}
                            ControlFlow::Break=>{brk=true;break;}
                            ControlFlow::Continue=>{break;}
                            other=>{self.scope.pop();return other;}
                        }
                    }
                    self.scope.pop();
                    if brk{break 'forin;}
                }
                ControlFlow::None
            }
            Statement::Match{value,arms}=>{
                let matched=match self.eval_expr(value){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                for arm in arms {
                    let does_match=match &arm.pattern{
                        MatchPattern::Wildcard=>true,
                        MatchPattern::Literal(lit_expr)=>{
                            let lit=match self.eval_expr(lit_expr.clone()){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                            self.values_equal(&matched,&lit)
                        }
                        MatchPattern::EnumVariant{enum_name,variant}=>{
                            match &matched{
                                Value::EnumVariant{enum_name:en,variant:v}=>en==enum_name&&v==variant,
                                _=>false,
                            }
                        }
                    };
                    if does_match {
                        self.scope.push();
                        for s in arm.body{
                            match self.execute_stmt(s){
                                ControlFlow::None=>{}
                                other=>{self.scope.pop();return other;}
                            }
                        }
                        self.scope.pop();
                        break;
                    }
                }
                ControlFlow::None
            }
                        Statement::Print{value}=>match self.eval_expr(value){Ok(v)=>{println!("{}",self.val_to_string(&v));ControlFlow::None}Err(e)=>ControlFlow::Error(e)}
            Statement::Assignment{name,type_annotation,value}=>{
                match self.eval_expr(value){
                    Err(e)=>ControlFlow::Error(e),
                    Ok(result)=>{
                        if let Some(ref ta)=type_annotation{if let Err(e)=self.check_type(&result,ta,&name){return ControlFlow::Error(e);}}
                        if name!="__discard__"{self.scope.set(&name,result);}
                        ControlFlow::None
                    }
                }
            }
            Statement::SelfFieldAssign{field,value}=>{
                // Detect self.field[idx] = val desugaring:
                // value = Binary{ left: Binary{ left: Field(self,field), op: "__idx_set__", right: idx }, op: "__val__", right: new_val }
                if let Expr::Binary{left:outer_left, op:outer_op, right:new_val_expr} = &value {
                    if outer_op == "__val__" {
                        if let Expr::Binary{left:_inner_left, op:inner_op, right:idx_expr} = outer_left.as_ref() {
                            if inner_op == "__idx_set__" {
                                let idx = match self.eval_expr(*idx_expr.clone()){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                                let new_val = match self.eval_expr(*new_val_expr.clone()){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                                // Compute map key BEFORE mutable borrow to satisfy borrow checker
                                let map_key = self.val_to_string(&idx);
                                match self.scope.get_mut("__self__"){
                                    Some(Value::Dotion{fields,..})=>{
                                        match fields.get_mut(&field){
                                            Some(Value::Array(arr))=>{
                                                if let Value::Number(i)=idx{
                                                    let i=i as usize;
                                                    if i>=arr.len(){arr.resize(i+1,Value::Nil);}
                                                    arr[i]=new_val;
                                                } else {
                                                    return ControlFlow::Error(type_error("Array index must be a number"));
                                                }
                                            }
                                            Some(Value::Map(m))=>{m.insert(map_key,new_val);}
                                            _=>return ControlFlow::Error(type_error(&format!("self.{} is not indexable",field))),
                                        }
                                    }
                                    _=>return ControlFlow::Error(runtime_error("'self' used outside of a dotion method or handler")),
                                }
                                return ControlFlow::None;
                            }
                        }
                    }
                }
                let new_val=match self.eval_expr(value){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                match self.scope.get_mut("__self__"){
                    Some(Value::Dotion{fields,..})=>{fields.insert(field,new_val);}
                    _=>return ControlFlow::Error(runtime_error("'self' used outside of a dotion method or handler")),
                }
                ControlFlow::None
            }
            Statement::MethodCallStmt{object,method,args}=>{
                let mut av=Vec::new();
                for a in args{match self.eval_expr(a){Ok(v)=>av.push(v),Err(e)=>return ControlFlow::Error(e)}}
                match self.scope.get(&object){
                    Some(Value::Dotion{..})=>{match self.call_dotion_method(&object,&method,av){Ok(_)=>{}Err(e)=>return ControlFlow::Error(e)}}
                    None=>return ControlFlow::Error(scope_error(&format!("'{}' is not defined",object))),
                    _=>return ControlFlow::Error(type_error(&format!("'{}' is not a dotion",object))),
                }
                ControlFlow::None
            }
            Statement::IndexAssign{object,index,value}=>{
                let idx=match self.eval_expr(index){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                let nv=match self.eval_expr(value){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                let ks=self.val_to_string(&idx);
                match self.scope.get_mut(&object){
                    Some(Value::Array(arr))=>{
                        if let Value::Number(i)=idx{let i=i as usize;if i>=arr.len(){arr.resize(i+1,Value::Nil);}arr[i]=nv;}
                        else{return ControlFlow::Error(type_error("Array index must be a number"));}
                    }
                    Some(Value::Map(m))=>{m.insert(ks,nv);}
                    Some(Value::Struct{fields,..})|Some(Value::Dotion{fields,..})=>{fields.insert(ks,nv);}
                    None=>return ControlFlow::Error(scope_error(&format!("'{}' is not defined",object))),
                    _=>return ControlFlow::Error(type_error(&format!("'{}' is not indexable",object))),
                }
                ControlFlow::None
            }
            Statement::FieldAssign{object,field,value}=>{
                let nv=match self.eval_expr(value){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                match self.scope.get_mut(&object){
                    Some(Value::Struct{fields,..})|Some(Value::Dotion{fields,..})=>{fields.insert(field,nv);}
                    Some(Value::Map(m))=>{m.insert(field,nv);}
                    None=>return ControlFlow::Error(scope_error(&format!("'{}' is not defined",object))),
                    _=>return ControlFlow::Error(type_error(&format!("'{}' does not support field assignment",object))),
                }
                ControlFlow::None
            }
            Statement::If{condition,then_branch,else_branch}=>{
                let c=match self.eval_expr(condition){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                let b=if self.is_truthy(&c){then_branch}else{else_branch};
                self.scope.push();
                for s in b{match self.execute_stmt(s){ControlFlow::None=>{}other=>{self.scope.pop();return other;}}}
                self.scope.pop();ControlFlow::None
            }
            Statement::While{condition,body}=>{
                loop{
                    let c=match self.eval_expr(condition.clone()){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                    if !self.is_truthy(&c){break;}
                    self.scope.push();let mut brk=false;
                    'wl:for s in body.clone(){match self.execute_stmt(s){ControlFlow::None=>{}ControlFlow::Break=>{brk=true;break 'wl;}ControlFlow::Continue=>{break 'wl;}other=>{self.scope.pop();return other;}}}
                    self.scope.pop();if brk{break;}
                }
                ControlFlow::None
            }
            Statement::For{init,condition,update,body}=>{
                self.scope.push();
                match self.execute_stmt(*init){ControlFlow::None=>{}other=>{self.scope.pop();return other;}}
                loop{
                    let c=match self.eval_expr(condition.clone()){Ok(v)=>v,Err(e)=>{self.scope.pop();return ControlFlow::Error(e);}};
                    if !self.is_truthy(&c){break;}
                    self.scope.push();let mut brk=false;
                    'fl:for s in body.clone(){match self.execute_stmt(s){ControlFlow::None=>{}ControlFlow::Break=>{brk=true;break 'fl;}ControlFlow::Continue=>{break 'fl;}other=>{self.scope.pop();self.scope.pop();return other;}}}
                    self.scope.pop();if brk{break;}
                    match self.execute_stmt(*update.clone()){ControlFlow::None=>{}other=>{self.scope.pop();return other;}}
                }
                self.scope.pop();ControlFlow::None
            }
            Statement::DoWhile{body,condition}=>{
                loop{
                    self.scope.push();let mut brk=false;
                    'dl:for s in body.clone(){match self.execute_stmt(s){ControlFlow::None=>{}ControlFlow::Break=>{brk=true;break 'dl;}ControlFlow::Continue=>{break 'dl;}other=>{self.scope.pop();return other;}}}
                    self.scope.pop();if brk{break;}
                    let c=match self.eval_expr(condition.clone()){Ok(v)=>v,Err(e)=>return ControlFlow::Error(e)};
                    if !self.is_truthy(&c){break;}
                }
                ControlFlow::None
            }
        }
    }

    // ── DOP PHASES ────────────────────────────────────────────────────
    fn process_mailboxes(&mut self) -> Result<(),FluxisError> {
        let names = self.scope.all_names();
        self.process_mailboxes_ordered(&names)
    }

    fn process_mailboxes_ordered(&mut self, names: &[String]) -> Result<(),FluxisError> {
        for vn in names {
            let val=match self.scope.get(vn){Some(v)=>v,None=>continue};
            let(did,dname,fields,methods,handlers,mailbox,brain,tags,tick_priority)=match val{
                Value::Dotion{id,name,fields,methods,handlers,mailbox,brain,tags,tick_priority} if !mailbox.is_empty()=>
                    (id,name,fields,methods,handlers,mailbox,brain,tags,tick_priority),
                _=>continue,
            };
            let mut upd=fields;
            let mut should_destroy=false;
            for(msg_name,arg) in mailbox {
                for h in &handlers {
                    if h.msg==msg_name {
                        self.scope.push();
                        for(k,v) in &upd{self.scope.define(k,v.clone());}
                        self.scope.define("__self__",Value::Dotion{id:did,name:dname.clone(),fields:upd.clone(),methods:methods.clone(),handlers:handlers.clone(),mailbox:Vec::new(),brain:brain.clone(),tags:tags.clone(),tick_priority});
                        if let Some(ref p)=h.param{self.scope.define(p,arg.clone());}
                        let mut err:Option<FluxisError>=None;
                        for s in h.body.clone(){match self.execute_stmt(s){ControlFlow::None|ControlFlow::Return(_)=>{}ControlFlow::Error(e)=>{err=Some(e);break;}_=>{}}}
                        let top=self.scope.stack.last().cloned().unwrap_or_default();
                        let self_val=self.scope.get("__self__");
                        self.scope.pop();
                        if let Some(e)=err{return Err(e);}
                        for(k,v) in top{if upd.contains_key(&k)&&k!="__self__"{upd.insert(k,v);}}
                        if let Some(Value::Dotion{fields:sf,..})=self_val{for(k,v) in sf{if upd.contains_key(&k){upd.insert(k,v);}}}
                        // Check if this message was "destroy" — trigger lifecycle
                        if msg_name == "destroy" { should_destroy = true; }
                        break;
                    }
                }
            }
            self.scope.set(vn,Value::Dotion{id:did,name:dname.clone(),fields:upd,methods:methods.clone(),handlers:handlers.clone(),mailbox:Vec::new(),brain:brain.clone(),tags:tags.clone(),tick_priority});
            // Auto-trigger on "destroy" only when health hits 0 (NOT when destroy was explicitly sent)
            // If should_destroy is already true, the handler already ran via the normal mailbox loop above
            if !should_destroy {
                let dead = match self.scope.get(vn) {
                    Some(Value::Dotion{ref fields,..}) => match fields.get("health") {
                        Some(Value::Number(h)) => *h <= 0,
                        Some(Value::Float(h))  => *h <= 0.0,
                        _ => false,
                    },
                    _ => false,
                };
                if dead && handlers.iter().any(|h| h.msg == "destroy") {
                    // Run on "destroy" handler with __self__ properly set
                    if let Some(dh) = handlers.iter().find(|h| h.msg == "destroy").cloned() {
                        if let Some(Value::Dotion{id:did2,name:dn2,fields:cf,methods:ms2,handlers:hs2,mailbox:mb2,brain:br2,tags:tg2,tick_priority:tp2})=self.scope.get(vn) {
                            self.scope.push();
                            for(k,v) in &cf{self.scope.define(k,v.clone());}
                            self.scope.define("__self__",Value::Dotion{id:did2,name:dn2.clone(),fields:cf.clone(),methods:ms2.clone(),handlers:hs2.clone(),mailbox:Vec::new(),brain:br2.clone(),tags:tg2.clone(),tick_priority:tp2});
                            let mut err2:Option<FluxisError>=None;
                            for s in dh.body.clone(){match self.execute_stmt(s){ControlFlow::None|ControlFlow::Return(_)=>{}ControlFlow::Error(e)=>{err2=Some(e);break;}_=>{}}}
                            self.scope.pop();
                            if let Some(e)=err2{return Err(e);}
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn run_actor_brains(&mut self) -> Result<(),FluxisError> {
        let names=self.scope.all_names();
        for vn in names {
            let brain_name=match self.scope.get(&vn){Some(Value::Dotion{brain:Some(b),..})=>b.clone(),_=>continue};
            if let Some(actor)=self.actor_types.get(&brain_name).cloned(){
                if let Some(decide)=actor.methods.iter().find(|m|m.name=="decide").cloned(){
                    let dotion_val=match self.scope.get(&vn){Some(v)=>v,None=>continue};
                    self.scope.push();
                    // Set __self__ so send_self() works inside the brain's decide()
                    self.scope.define("__self__", dotion_val.clone());
                    // Also store the variable name so send_self can find it
                    self.scope.define("__dotion_var__", Value::Str(vn.clone()));
                    if let Some(param)=decide.params.first(){self.scope.define(param,dotion_val);}
                    let mut err:Option<FluxisError>=None;
                    for s in decide.body{match self.execute_stmt(s){ControlFlow::None|ControlFlow::Return(_)=>{}ControlFlow::Error(e)=>{err=Some(e);break;}_=>{}}}
                    self.scope.pop();
                    if let Some(e)=err{return Err(e);}
                }
            }
        }
        Ok(())
    }

    fn call_dotion_method(&mut self, var_name: &str, method_name: &str, args: Vec<Value>) -> Result<Value,FluxisError> {
        let(did,dname,fields,methods,handlers,mailbox,brain,tags,tick_priority)=match self.scope.get(var_name){
            Some(Value::Dotion{id,name,fields,methods,handlers,mailbox,brain,tags,tick_priority})=>(id,name,fields,methods,handlers,mailbox,brain,tags,tick_priority),
            None=>return Err(scope_error(&format!("'{}' is not defined",var_name))),
            _=>return Err(type_error(&format!("'{}' is not a dotion",var_name))),
        };
        let method=methods.iter().find(|m|m.name==method_name).cloned()
            .ok_or_else(||runtime_error(&format!("Dotion '{}' has no method '{}'",dname,method_name))
                .with_hint(&format!("Define it with fn {}() {{ ... }} inside the dotion",method_name)))?;
        if args.len()!=method.params.len(){return Err(arity_error(&format!("{}.{}",dname,method_name),method.params.len(),args.len()));}
        self.scope.push();
        for(p,v) in method.params.iter().zip(args.iter()){self.scope.define(p,v.clone());}
        self.scope.define("__self__",Value::Dotion{id:did,name:dname.clone(),fields:fields.clone(),methods:methods.clone(),handlers:handlers.clone(),mailbox:mailbox.clone(),brain:brain.clone(),tags:tags.clone(),tick_priority});
        for(k,v) in &fields{self.scope.define(k,v.clone());}
        let mut ret=Value::Nil;let mut err:Option<FluxisError>=None;
        for s in method.body{match self.execute_stmt(s){ControlFlow::Return(v)=>{ret=v;break;}ControlFlow::Error(e)=>{err=Some(e);break;}_=>{}}}
        let top=self.scope.stack.last().cloned().unwrap_or_default();
        let self_val=self.scope.get("__self__");
        self.scope.pop();
        if let Some(e)=err{return Err(e);}
        let mut new_fields=fields;
        for(k,v) in top{if new_fields.contains_key(&k)&&k!="__self__"{new_fields.insert(k,v);}}
        if let Some(Value::Dotion{fields:sf,..})=self_val{for(k,v) in sf{if new_fields.contains_key(&k){new_fields.insert(k,v);}}}
        self.scope.set(var_name,Value::Dotion{id:did,name:dname,fields:new_fields,methods,handlers,mailbox,brain,tags,tick_priority});
        Ok(ret)
    }

    // ── CALL FUNCTION ─────────────────────────────────────────────────
    fn call_function(&mut self, name: &str, arg_values: Vec<Value>) -> Result<Value,FluxisError> {
        match name {
            "len"    =>{if arg_values.len()!=1{return Err(arity_error("len",1,arg_values.len()));}return Ok(match &arg_values[0]{Value::Array(a)=>Value::Number(a.len()as i64),Value::Map(m)=>Value::Number(m.len()as i64),Value::Str(s)=>Value::Number(s.len()as i64),Value::Dotion{fields,..}=>Value::Number(fields.len()as i64),other=>return Err(type_error(&format!("len() not supported on {}",self.type_name(other))))});}
            "push"   =>{if arg_values.len()!=2{return Err(arity_error("push",2,arg_values.len()));}if let Value::Array(mut a)=arg_values[0].clone(){a.push(arg_values[1].clone());return Ok(Value::Array(a));}return Err(type_error("push() requires array"));}
            "pop"    =>{if arg_values.len()!=1{return Err(arity_error("pop",1,arg_values.len()));}if let Value::Array(mut a)=arg_values[0].clone(){a.pop();return Ok(Value::Array(a));}return Err(type_error("pop() requires array"));}
             "keys"   =>{if arg_values.len()!=1{return Err(arity_error("keys",1,arg_values.len()));}return Ok(match &arg_values[0]{Value::Map(m)=>Value::Array(m.keys().map(|k|Value::Str(k.clone())).collect()),Value::Struct{fields,..}|Value::Dotion{fields,..}=>Value::Array(fields.keys().map(|k|Value::Str(k.clone())).collect()),other=>return Err(type_error(&format!("keys() not supported on {}",self.type_name(other))))});}
            "has"    =>{if arg_values.len()!=2{return Err(arity_error("has",2,arg_values.len()));}let k=self.val_to_string(&arg_values[1]);return Ok(match &arg_values[0]{Value::Map(m)=>Value::Bool(m.contains_key(&k)),Value::Struct{fields,..}|Value::Dotion{fields,..}=>Value::Bool(fields.contains_key(&k)),other=>return Err(type_error(&format!("has() not supported on {}",self.type_name(other))))});}
            "del"    =>{if arg_values.len()!=2{return Err(arity_error("del",2,arg_values.len()));}let k=self.val_to_string(&arg_values[1]);if let Value::Map(mut m)=arg_values[0].clone(){m.remove(&k);return Ok(Value::Map(m));}return Err(type_error("del() requires map"));}
            "type_of"=>{if arg_values.len()!=1{return Err(arity_error("type_of",1,arg_values.len()));}return Ok(Value::Str(self.type_name(&arg_values[0]).to_string()));}
            "to_str" =>{if arg_values.len()!=1{return Err(arity_error("to_str",1,arg_values.len()));}return Ok(Value::Str(self.val_to_string(&arg_values[0])));}
            "to_float"=>{if arg_values.len()!=1{return Err(arity_error("to_float",1,arg_values.len()));}return Ok(match &arg_values[0]{Value::Number(n)=>Value::Float(*n as f64),Value::Float(f)=>Value::Float(*f),Value::Str(s)=>s.trim().parse::<f64>().map(Value::Float).unwrap_or(Value::Float(0.0)),Value::Bool(b)=>Value::Float(if *b{1.0}else{0.0}),_=>Value::Float(0.0)});}
            "to_num" =>{if arg_values.len()!=1{return Err(arity_error("to_num",1,arg_values.len()));}return Ok(match &arg_values[0]{Value::Str(s)=>s.trim().parse::<f64>().map(|f|if f.fract()==0.0{Value::Number(f as i64)}else{Value::Float(f)}).unwrap_or(Value::Number(0)),Value::Number(n)=>Value::Number(*n),Value::Float(f)=>Value::Number(*f as i64),Value::Bool(b)=>Value::Number(if *b{1}else{0}),_=>Value::Number(0)});}
            "tick_count"=>{return Ok(Value::Number(self.tick_engine.tick_count as i64));}
            "format"=>{
                if arg_values.is_empty(){return Err(arity_error("format",1,0));}
                let template=match &arg_values[0]{Value::Str(s)=>s.clone(),other=>return Err(type_error(&format!("format() first arg must be string, got {}",self.type_name(other))))};
                let extra=&arg_values[1..];
                let mut result=String::new();
                let chars:Vec<char>=template.chars().collect();
                let mut i=0usize;let mut arg_idx=0usize;
                while i<chars.len(){
                    if chars[i]=='{'&&i+1<chars.len(){
                        if chars[i+1]=='}'{
                            if arg_idx<extra.len(){result.push_str(&self.val_to_string(&extra[arg_idx]));arg_idx+=1;}
                            i+=2;
                        }else{
                            let mut name=String::new();let mut j=i+1;
                            while j<chars.len()&&chars[j]!='}'&&chars[j]!='{'{name.push(chars[j]);j+=1;}
                            if j<chars.len()&&chars[j]=='}'&&!name.is_empty(){
                                if let Some(v)=self.scope.get(&name){result.push_str(&self.val_to_string(&v));}
                                else if let Ok(idx)=name.parse::<usize>(){if idx<extra.len(){result.push_str(&self.val_to_string(&extra[idx]));}}
                                else{result.push('{');result.push_str(&name);result.push('}');}
                                i=j+1;
                            }else{result.push(chars[i]);i+=1;}
                        }
                    }else{result.push(chars[i]);i+=1;}
                }
                return Ok(Value::Str(result));
            }
            "range"=>{
                if arg_values.len()<2||arg_values.len()>3{return Err(runtime_error("range() takes 2 or 3 args: range(start,end) or range(start,end,step)").with_hint("Example: range(0,10) or range(0,10,2)"));}
                let start=match &arg_values[0]{Value::Number(n)=>*n,Value::Float(f)=>*f as i64,other=>return Err(type_error(&format!("range() start must be num, got {}",self.type_name(other))))};
                let end=match &arg_values[1]{Value::Number(n)=>*n,Value::Float(f)=>*f as i64,other=>return Err(type_error(&format!("range() end must be num, got {}",self.type_name(other))))};
                let step=if arg_values.len()==3{match &arg_values[2]{Value::Number(n)=>*n,Value::Float(f)=>*f as i64,other=>return Err(type_error(&format!("range() step must be num, got {}",self.type_name(other))))}}else{if start<=end{1}else{-1}};
                if step==0{return Err(runtime_error("range() step cannot be zero"));}
                let mut arr=Vec::new();let mut cur=start;
                if step>0{while cur<end{arr.push(Value::Number(cur));cur+=step;}}
                else{while cur>end{arr.push(Value::Number(cur));cur+=step;}}
                return Ok(Value::Array(arr));
            }
            "sort_arr"=>{
                if arg_values.len()!=1{return Err(arity_error("sort_arr",1,arg_values.len()));}
                let mut arr=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("sort_arr() requires array, got {}",self.type_name(&other))))};
                arr.sort_by(|a,b|match(a,b){
                    (Value::Number(x),Value::Number(y))=>x.cmp(y),
                    (Value::Float(x),Value::Float(y))=>x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Number(x),Value::Float(y))=>(*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Float(x),Value::Number(y))=>x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal),
                    (a,b)=>self.val_to_string(a).cmp(&self.val_to_string(b)),
                });
                return Ok(Value::Array(arr));
            }
            "sort_desc"=>{
                if arg_values.len()!=1{return Err(arity_error("sort_desc",1,arg_values.len()));}
                let mut arr=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("sort_desc() requires array, got {}",self.type_name(&other))))};
                arr.sort_by(|a,b|match(a,b){
                    (Value::Number(x),Value::Number(y))=>y.cmp(x),
                    (Value::Float(x),Value::Float(y))=>y.partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Number(x),Value::Float(y))=>(*y).partial_cmp(&(*x as f64)).unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Float(x),Value::Number(y))=>(*y as f64).partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal),
                    (a,b)=>self.val_to_string(b).cmp(&self.val_to_string(a)),
                });
                return Ok(Value::Array(arr));
            }
            "map_fn"=>{
                if arg_values.len()!=2{return Err(arity_error("map_fn",2,arg_values.len()));}
                let arr=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("map_fn() first arg must be array, got {}",self.type_name(&other))))};
                let fn_name=match arg_values[1].clone(){Value::Str(s)=>s,other=>return Err(type_error(&format!("map_fn() second arg must be function name string, got {}",self.type_name(&other))))};
                let mut result=Vec::new();
                for item in arr{match self.call_function(&fn_name,vec![item]){Ok(v)=>result.push(v),Err(e)=>return Err(e)}}
                return Ok(Value::Array(result));
            }
            "filter_fn"=>{
                if arg_values.len()!=2{return Err(arity_error("filter_fn",2,arg_values.len()));}
                let arr=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("filter_fn() first arg must be array, got {}",self.type_name(&other))))};
                let fn_name=match arg_values[1].clone(){Value::Str(s)=>s,other=>return Err(type_error(&format!("filter_fn() second arg must be function name string, got {}",self.type_name(&other))))};
                let mut result=Vec::new();
                for item in arr{match self.call_function(&fn_name,vec![item.clone()]){Ok(v)=>if self.is_truthy(&v){result.push(item);},Err(e)=>return Err(e)}}
                return Ok(Value::Array(result));
            }
            "reduce_fn"=>{
                if arg_values.len()!=3{return Err(arity_error("reduce_fn",3,arg_values.len()));}
                let arr=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("reduce_fn() first arg must be array, got {}",self.type_name(&other))))};
                let fn_name=match arg_values[1].clone(){Value::Str(s)=>s,other=>return Err(type_error(&format!("reduce_fn() second arg must be function name string, got {}",self.type_name(&other))))};
                let mut acc=arg_values[2].clone();
                for item in arr{match self.call_function(&fn_name,vec![acc,item]){Ok(v)=>acc=v,Err(e)=>return Err(e)}}
                return Ok(acc);
            }
            "any_fn"=>{
                if arg_values.len()!=2{return Err(arity_error("any_fn",2,arg_values.len()));}
                let arr=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("any_fn() first arg must be array, got {}",self.type_name(&other))))};
                let fn_name=match arg_values[1].clone(){Value::Str(s)=>s,other=>return Err(type_error(&format!("any_fn() second arg must be function name string, got {}",self.type_name(&other))))};
                for item in arr{match self.call_function(&fn_name,vec![item]){Ok(v)=>if self.is_truthy(&v){return Ok(Value::Bool(true));},Err(e)=>return Err(e)}}
                return Ok(Value::Bool(false));
            }
            "all_fn"=>{
                if arg_values.len()!=2{return Err(arity_error("all_fn",2,arg_values.len()));}
                let arr=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("all_fn() first arg must be array, got {}",self.type_name(&other))))};
                let fn_name=match arg_values[1].clone(){Value::Str(s)=>s,other=>return Err(type_error(&format!("all_fn() second arg must be function name string, got {}",self.type_name(&other))))};
                for item in arr{match self.call_function(&fn_name,vec![item]){Ok(v)=>if !self.is_truthy(&v){return Ok(Value::Bool(false));},Err(e)=>return Err(e)}}
                return Ok(Value::Bool(true));
            }
            "remove"=>{
                if arg_values.len()!=2{return Err(arity_error("remove",2,arg_values.len()));}
                let mut arr=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("remove() first arg must be array, got {}",self.type_name(&other))))};
                let idx=match &arg_values[1]{Value::Number(n)=>*n as usize,other=>return Err(type_error(&format!("remove() index must be num, got {}",self.type_name(other))))};
                if idx>=arr.len(){return Err(runtime_error(&format!("remove() index {} out of bounds (len={})",idx,arr.len())));}
                arr.remove(idx);
                return Ok(Value::Array(arr));
            }
            "insert"=>{
                if arg_values.len()!=3{return Err(arity_error("insert",3,arg_values.len()));}
                let mut arr=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("insert() first arg must be array, got {}",self.type_name(&other))))};
                let idx=match &arg_values[1]{Value::Number(n)=>(*n as usize).min(arr.len()),other=>return Err(type_error(&format!("insert() index must be num, got {}",self.type_name(other))))};
                arr.insert(idx,arg_values[2].clone());
                return Ok(Value::Array(arr));
            }
            "slice"=>{
                if arg_values.len()<2||arg_values.len()>3{return Err(runtime_error("slice() takes 2 or 3 args: slice(arr,start) or slice(arr,start,end)"));}
                let arr=match arg_values[0].clone(){Value::Array(a)=>a,Value::Str(s)=>s.chars().map(|c|Value::Str(c.to_string())).collect(),other=>return Err(type_error(&format!("slice() first arg must be array or string, got {}",self.type_name(&other))))};
                let start=match &arg_values[1]{Value::Number(n)=>(*n).max(0) as usize,other=>return Err(type_error(&format!("slice() start must be num, got {}",self.type_name(other))))};
                let end=if arg_values.len()==3{match &arg_values[2]{Value::Number(n)=>(*n).max(0) as usize,other=>return Err(type_error(&format!("slice() end must be num, got {}",self.type_name(other))))}}else{arr.len()};
                let end=end.min(arr.len());let start=start.min(end);
                return Ok(Value::Array(arr[start..end].to_vec()));
            }
            "flatten"=>{
                if arg_values.len()!=1{return Err(arity_error("flatten",1,arg_values.len()));}
                let arr=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("flatten() requires array, got {}",self.type_name(&other))))};
                let mut result=Vec::new();
                for item in arr{match item{Value::Array(inner)=>result.extend(inner),other=>result.push(other)}}
                return Ok(Value::Array(result));
            }
            "reverse"=>{
                if arg_values.len()!=1{return Err(arity_error("reverse",1,arg_values.len()));}
                match arg_values[0].clone(){
                    Value::Array(mut a)=>{a.reverse();return Ok(Value::Array(a));}
                    Value::Str(s)=>return Ok(Value::Str(s.chars().rev().collect())),
                    other=>return Err(type_error(&format!("reverse() requires array or string, got {}",self.type_name(&other)))),
                }
            }
            "zip"=>{
                if arg_values.len()!=2{return Err(arity_error("zip",2,arg_values.len()));}
                let a=match arg_values[0].clone(){Value::Array(a)=>a,other=>return Err(type_error(&format!("zip() first arg must be array, got {}",self.type_name(&other))))};
                let b=match arg_values[1].clone(){Value::Array(b)=>b,other=>return Err(type_error(&format!("zip() second arg must be array, got {}",self.type_name(&other))))};
                let result=a.iter().zip(b.iter()).map(|(x,y)|Value::Array(vec![x.clone(),y.clone()])).collect();
                return Ok(Value::Array(result));
            }
            "dotion_list"=>{
                let names=self.scope.all_names();
                let mut dotions:Vec<Value>=names.iter().filter_map(|n|self.scope.get(n)).filter(|v|matches!(v,Value::Dotion{..})).collect();
                // Sort by tick_priority
                dotions.sort_by_key(|d|match d{Value::Dotion{tick_priority,..}=>*tick_priority,_=>0});
                return Ok(Value::Array(dotions));
            }
            // ── send_self: queue a message to the current dotion from inside a handler/method ──
            "send_self"=>{
                if arg_values.len()<1||arg_values.len()>2{return Err(runtime_error("send_self(\"msg\") or send_self(\"msg\", val)"));}
                let msg=match &arg_values[0]{Value::Str(s)=>s.clone(),other=>return Err(type_error(&format!("send_self() msg must be string, got {}",self.type_name(other))))};
                let arg=if arg_values.len()==2{arg_values[1].clone()}else{Value::Nil};
                // Get the current dotion's ID from __self__
                let self_id=match self.scope.get("__self__"){
                    Some(Value::Dotion{id,..})=>id,
                    _=>return Err(runtime_error("send_self() used outside of a dotion method, handler, or actor brain")),
                };
                // First try __dotion_var__ hint (set by actor brain runner)
                let hint_var=match self.scope.get("__dotion_var__"){
                    Some(Value::Str(s))=>Some(s),
                    _=>None,
                };
                if let Some(ref vn)=hint_var {
                    if let Some(Value::Dotion{id,name,fields,methods,handlers,mut mailbox,brain,tags,tick_priority})=self.scope.get(vn){
                        if id==self_id {
                            mailbox.push((msg.clone(),arg.clone()));
                            self.scope.set(vn,Value::Dotion{id,name,fields,methods,handlers,mailbox,brain,tags,tick_priority});
                            return Ok(Value::Nil);
                        }
                    }
                }
                // Fallback: search all scope for matching ID
                let names=self.scope.all_names();
                for vn in &names{
                    if let Some(Value::Dotion{id,name,fields,methods,handlers,mut mailbox,brain,tags,tick_priority})=self.scope.get(vn){
                        if id==self_id{
                            mailbox.push((msg.clone(),arg.clone()));
                            self.scope.set(vn,Value::Dotion{id,name,fields,methods,handlers,mailbox,brain,tags,tick_priority});
                            return Ok(Value::Nil);
                        }
                    }
                }
                return Err(runtime_error("send_self() could not find current dotion in scope"));
            }
            // ── clone: create an independent copy of a dotion with a new ID ──────────────────
            "clone"=>{
                if arg_values.len()!=1{return Err(arity_error("clone",1,arg_values.len()));}
                match arg_values[0].clone(){
                    Value::Dotion{name,fields,methods,handlers,brain,tags,tick_priority,..}=>
                        return Ok(Value::Dotion{id:new_id(),name,fields,methods,handlers,mailbox:Vec::new(),brain,tags,tick_priority}),
                    other=>return Err(type_error(&format!("clone() requires a dotion, got {}",self.type_name(&other)))),
                }
            }
            // ── dotion_where: filter dotions by field value ──────────────────────────────────
            "dotion_where"=>{
                if arg_values.len()!=2{return Err(arity_error("dotion_where",2,arg_values.len()));}
                let field=match &arg_values[0]{Value::Str(s)=>s.clone(),other=>return Err(type_error(&format!("dotion_where() field must be string, got {}",self.type_name(other))))};
                let target_val=arg_values[1].clone();
                let target_str=self.val_to_string(&target_val);
                let names=self.scope.all_names();
                let mut result=Vec::new();
                for n in &names{
                    if let Some(Value::Dotion{ref fields,..})=self.scope.get(n){
                        if let Some(v)=fields.get(&field){
                            if self.val_to_string(v)==target_str{
                                result.push(self.scope.get(n).unwrap());
                            }
                        }
                    }
                }
                return Ok(Value::Array(result));
            }
            // ── dotion_where_fn: filter dotions by calling a method on each ──────────────────
            "dotion_where_fn"=>{
                if arg_values.len()!=1{return Err(arity_error("dotion_where_fn",1,arg_values.len()));}
                let method=match &arg_values[0]{Value::Str(s)=>s.clone(),other=>return Err(type_error(&format!("dotion_where_fn() method must be string, got {}",self.type_name(other))))};
                let names=self.scope.all_names();
                let dotion_names:Vec<String>=names.iter().filter(|n|matches!(self.scope.get(n),Some(Value::Dotion{..}))).cloned().collect();
                let mut result=Vec::new();
                for vn in &dotion_names{
                    match self.call_dotion_method(vn,&method,vec![]){
                        Ok(Value::Bool(true))=>result.push(self.scope.get(vn).unwrap()),
                        Ok(_)=>{}
                        Err(e)=>return Err(e),
                    }
                }
                return Ok(Value::Array(result));
            }
            // ── dotion_count: count dotions in scope ──────────────────────────────────────────
            "dotion_count"=>{
                let names=self.scope.all_names();
                let count=names.iter().filter(|n|matches!(self.scope.get(n),Some(Value::Dotion{..}))).count();
                return Ok(Value::Number(count as i64));
            }
            // ── broadcast_to: send to dotions matching a tag ──────────────────────────────────
            "broadcast_to"=>{
                if arg_values.len()<2||arg_values.len()>3{return Err(runtime_error("broadcast_to(\"tag\", \"msg\") or broadcast_to(\"tag\", \"msg\", val)"));}
                let tag=match &arg_values[0]{Value::Str(s)=>s.clone(),other=>return Err(type_error(&format!("broadcast_to() tag must be string, got {}",self.type_name(other))))};
                let msg=match &arg_values[1]{Value::Str(s)=>s.clone(),other=>return Err(type_error(&format!("broadcast_to() msg must be string, got {}",self.type_name(other))))};
                let arg=if arg_values.len()==3{arg_values[2].clone()}else{Value::Nil};
                let names=self.scope.all_names();
                for vn in &names{
                    if let Some(Value::Dotion{id,name,fields,methods,handlers,mut mailbox,brain,tags,tick_priority})=self.scope.get(vn){
                        if tags.contains(&tag){
                            mailbox.push((msg.clone(),arg.clone()));
                            self.scope.set(vn,Value::Dotion{id,name,fields,methods,handlers,mailbox,brain,tags,tick_priority});
                        }
                    }
                }
                return Ok(Value::Nil);
            }
            // ── dotion_to_str: serialize a dotion's fields to a string ───────────────────────
            "dotion_to_str"=>{
                if arg_values.len()!=1{return Err(arity_error("dotion_to_str",1,arg_values.len()));}
                match &arg_values[0]{
                    Value::Dotion{name,fields,tags,tick_priority,..}=>{
                        let mut parts=Vec::new();
                        parts.push(format!("__name__:{}", name));
                        parts.push(format!("__tags__:{}", tags.join(",")));
                        parts.push(format!("__priority__:{}", tick_priority));
                        for(k,v) in fields{
                            let vs=self.val_to_string(v);
                            parts.push(format!("{}:{}", k, vs));
                        }
                        return Ok(Value::Str(parts.join("|")));
                    }
                    other=>return Err(type_error(&format!("dotion_to_str() requires dotion, got {}",self.type_name(other)))),
                }
            }
            // ── dotion_from_str: restore dotion fields from serialized string ─────────────────
            "dotion_from_str"=>{
                if arg_values.len()<1||arg_values.len()>2{return Err(runtime_error("dotion_from_str(str) or dotion_from_str(str, dotion)"));}
                let s=match &arg_values[0]{Value::Str(s)=>s.clone(),other=>return Err(type_error(&format!("dotion_from_str() first arg must be string, got {}",self.type_name(other))))};
                // If second arg given, merge into that dotion — otherwise create plain map
                let mut fields:HashMap<String,Value>=HashMap::new();
                let mut restored_name=String::from("dotion");
                let mut restored_tags:Vec<String>=Vec::new();
                let mut restored_priority:i64=0;
                for part in s.split('|'){
                    let mut kv=part.splitn(2,':');
                    let k=kv.next().unwrap_or("").to_string();
                    let v=kv.next().unwrap_or("").to_string();
                    match k.as_str(){
                        "__name__" => restored_name=v,
                        "__tags__" => restored_tags=if v.is_empty(){Vec::new()}else{v.split(',').map(|s|s.to_string()).collect()},
                        "__priority__" => restored_priority=v.parse::<i64>().unwrap_or(0),
                        _ => { fields.insert(k, Value::Str(v)); }
                    }
                }
                if arg_values.len()==2 {
                    // Merge into existing dotion
                    match arg_values[1].clone(){
                        Value::Dotion{id,name:dname2,fields:mut df,methods,handlers,mailbox,brain,tags,tick_priority}=>{
                            for(k,v) in fields{df.insert(k,v);}
                            return Ok(Value::Dotion{id,name:dname2,fields:df,methods,handlers,mailbox,brain,tags,tick_priority});
                        }
                        other=>return Err(type_error(&format!("dotion_from_str() second arg must be dotion, got {}",self.type_name(&other)))),
                    }
                } else {
                    // Create new bare dotion (no methods/handlers — use with existing type)
                    return Ok(Value::Dotion{id:new_id(),name:restored_name,fields,methods:Vec::new(),handlers:Vec::new(),mailbox:Vec::new(),brain:None,tags:restored_tags,tick_priority:restored_priority});
                }
            }
            // ── Type-check builtins ───────────────────────────────────
            "is_num"  =>{if arg_values.len()!=1{return Err(arity_error("is_num",1,arg_values.len()));}return Ok(Value::Bool(matches!(&arg_values[0],Value::Number(_))));}
            "is_float"=>{if arg_values.len()!=1{return Err(arity_error("is_float",1,arg_values.len()));}return Ok(Value::Bool(matches!(&arg_values[0],Value::Float(_))));}
            "is_str"  =>{if arg_values.len()!=1{return Err(arity_error("is_str",1,arg_values.len()));}return Ok(Value::Bool(matches!(&arg_values[0],Value::Str(_))));}
            "is_bool" =>{if arg_values.len()!=1{return Err(arity_error("is_bool",1,arg_values.len()));}return Ok(Value::Bool(matches!(&arg_values[0],Value::Bool(_))));}
            "is_array"=>{if arg_values.len()!=1{return Err(arity_error("is_array",1,arg_values.len()));}return Ok(Value::Bool(matches!(&arg_values[0],Value::Array(_))));}
            "is_map"  =>{if arg_values.len()!=1{return Err(arity_error("is_map",1,arg_values.len()));}return Ok(Value::Bool(matches!(&arg_values[0],Value::Map(_))));}
            "is_nil"  =>{if arg_values.len()!=1{return Err(arity_error("is_nil",1,arg_values.len()));}return Ok(Value::Bool(matches!(&arg_values[0],Value::Nil)));}
            // ── Assert ───────────────────────────────────────────────
            "assert" =>{
                if arg_values.is_empty(){return Err(arity_error("assert",1,0));}
                let ok=match &arg_values[0]{
                    Value::Bool(b)=>*b,
                    Value::Nil=>false,
                    Value::Number(n)=>*n!=0,
                    _=>true,
                };
                if !ok {
                    let msg=if arg_values.len()>=2{
                        self.val_to_string(&arg_values[1])
                    } else {
                        "Assertion failed".to_string()
                    };
                    return Err(runtime_error(&msg).with_hint("assert(condition, \"message\")"));
                }
                return Ok(Value::Nil);
            }
            "send"=>{
                if arg_values.len()<2||arg_values.len()>3{return Err(arity_error("send",3,arg_values.len()));}
                let msg=match &arg_values[1]{Value::Str(s)=>s.clone(),other=>return Err(type_error(&format!("send() msg must be string, got {}",self.type_name(other))))};
                let arg=if arg_values.len()==3{arg_values[2].clone()}else{Value::Nil};
                let tid=match &arg_values[0]{Value::Dotion{id,..}=>*id,other=>return Err(type_error(&format!("send() first arg must be dotion, got {}",self.type_name(other))))};
                let names=self.scope.all_names();
                for vn in &names{
                    if let Some(Value::Dotion{id,name,fields,methods,handlers,mut mailbox,brain,tags,tick_priority})=self.scope.get(vn){
                        if id==tid{mailbox.push((msg.clone(),arg.clone()));self.scope.set(vn,Value::Dotion{id,name,fields,methods,handlers,mailbox,brain,tags,tick_priority});return Ok(Value::Nil);}
                    }
                }
                return Err(runtime_error("send() could not find target dotion in scope").with_hint("Make sure the dotion variable is in scope when calling send()"));
            }
            "broadcast"=>{
                if arg_values.len()<1||arg_values.len()>2{return Err(arity_error("broadcast",2,arg_values.len()));}
                let msg=match &arg_values[0]{Value::Str(s)=>s.clone(),other=>return Err(type_error(&format!("broadcast() msg must be string, got {}",self.type_name(other))))};
                let arg=if arg_values.len()==2{arg_values[1].clone()}else{Value::Nil};
                let names=self.scope.all_names();
                for vn in &names{if let Some(Value::Dotion{id,name,fields,methods,handlers,mut mailbox,brain,tags,tick_priority})=self.scope.get(vn){mailbox.push((msg.clone(),arg.clone()));self.scope.set(vn,Value::Dotion{id,name,fields,methods,handlers,mailbox,brain,tags,tick_priority});}}
                return Ok(Value::Nil);
            }
            _=>{}
        }
        if stdlib::is_math_fn(name)   {return self.call_math(name,&arg_values);}
        if stdlib::is_string_fn(name) {return self.call_string(name,&arg_values);}
        if stdlib::is_io_fn(name)     {return self.call_io(name,&arg_values);}
        if stdlib::is_ai_fn(name)     {return self.call_ai(name,&arg_values);}
        if stdlib::is_ml_fn(name)     {return self.call_ml(name,&arg_values);}
        if stdlib::is_gfx_fn(name)    {return self.call_gfx(name,&arg_values);}
        if let Some(type_name)=name.strip_prefix("__dotion__"){
            if let Some(def)=self.dotion_types.get(type_name).cloned(){
                let mut fields=HashMap::new();
                for(fn_,fexpr) in &def.fields{match self.eval_expr(fexpr.clone()){Ok(v)=>{fields.insert(fn_.clone(),v);}Err(e)=>return Err(e)}}
                return Ok(Value::Dotion{id:new_id(),name:type_name.to_string(),fields,methods:def.methods,handlers:def.handlers,mailbox:Vec::new(),brain:def.brain,tags:def.tags.clone(),tick_priority:def.tick_priority});
            }
        }
        let func=self.functions.get(name).cloned().ok_or_else(||runtime_error(&format!("Undefined function '{}'",name)).with_hint(&format!("Define it with: fn {}(...) {{ ... }}",name)))?;
        if arg_values.len()!=func.params.len(){return Err(arity_error(name,func.params.len(),arg_values.len()));}
        for((pn,pt),v) in func.params.iter().zip(arg_values.iter()){if let Some(ta)=pt{self.check_type(v,ta,pn)?;}}
        let rt=func.return_type.clone();
        let saved=std::mem::replace(&mut self.scope,Scope::new());
        for((pn,_),v) in func.params.iter().zip(arg_values.into_iter()){self.scope.define(pn,v);}
        let mut ret=Value::Nil;let mut err:Option<FluxisError>=None;
        for s in func.body{match self.execute_stmt(s){ControlFlow::Return(v)=>{ret=v;break;}ControlFlow::Error(e)=>{err=Some(e);break;}_=>{}}}
        self.scope=saved;if let Some(e)=err{return Err(e);}
        if let Some(ref rt)=rt{if !matches!(rt,TypeAnnotation::Any){if let Err(_)=self.check_type(&ret,rt,name){return Err(type_error(&format!("Function '{}' must return '{}'",name,rt.name())));}}}
        Ok(ret)
    }

    // ── EVAL EXPR ─────────────────────────────────────────────────────
    fn eval_expr(&mut self, expr: Expr) -> Result<Value,FluxisError> {
        match expr {
            Expr::Number(n)=>Ok(Value::Number(n)),
            Expr::Float(f)=>Ok(Value::Float(f)),
            Expr::String(s)=>Ok(Value::Str(s)),
            Expr::Bool(b)=>Ok(Value::Bool(b)),
            Expr::Nil=>Ok(Value::Nil),
            Expr::Identifier(n)=>self.scope.get(&n).ok_or_else(||scope_error(&format!("'{}' is not defined",n)).with_hint(&format!("Declare it first: {} = <value>;",n))),
            Expr::Self_=>self.scope.get("__self__").ok_or_else(||runtime_error("'self' used outside of a dotion method or handler")),
            Expr::Input=>{use std::io::{self,Write};let mut s=String::new();print!("> ");io::stdout().flush().unwrap();io::stdin().read_line(&mut s).unwrap();let t=s.trim();Ok(if let Ok(n)=t.parse::<i64>(){Value::Number(n)}else{Value::Str(t.to_string())})}
            Expr::DotionLit{fields,methods,handlers}=>{let mut fm=HashMap::new();for(k,v) in fields{fm.insert(k,self.eval_expr(v)?);}Ok(Value::Dotion{id:new_id(),name:"dotion".into(),fields:fm,methods,handlers,mailbox:Vec::new(),brain:None,tags:Vec::new(),tick_priority:0})}
            Expr::Array(els)=>{let mut v=Vec::new();for e in els{v.push(self.eval_expr(e)?);}Ok(Value::Array(v))}
            Expr::Map(pairs)=>{let mut m=HashMap::new();for(k,v) in pairs{let kv=self.eval_expr(k)?;let key=self.val_to_string(&kv);m.insert(key,self.eval_expr(v)?);}Ok(Value::Map(m))}
            Expr::Index{object,index}=>{
                let obj=self.eval_expr(*object)?;let idx=self.eval_expr(*index)?;
                match obj{
                    Value::Array(a)=>{if let Value::Number(i)=idx{Ok(a.get(i as usize).cloned().unwrap_or(Value::Nil))}else{Err(type_error("Array index must be a number"))}}
                    Value::Map(m)=>Ok(m.get(&self.val_to_string(&idx)).cloned().unwrap_or(Value::Nil)),
                    Value::Str(s)=>{if let Value::Number(i)=idx{Ok(s.chars().nth(i as usize).map(|c|Value::Str(c.to_string())).unwrap_or(Value::Nil))}else{Err(type_error("String index must be a number"))}}
                    other=>Err(type_error(&format!("{} is not indexable",self.type_name(&other)))),
                }
            }
            Expr::Field{object,field}=>{
                let obj=self.eval_expr(*object)?;
                match obj{
                    Value::Struct{ref fields,..}|Value::Dotion{ref fields,..}=>Ok(fields.get(&field).cloned().unwrap_or(Value::Nil)),
                    Value::Map(ref m)=>Ok(m.get(&field).cloned().unwrap_or(Value::Nil)),
                    other=>Err(type_error(&format!("Cannot access .{} on {}",field,self.type_name(&other)))),
                }
            }
            Expr::MethodCall{object,method,args}=>{
                if let Expr::Identifier(ref vn)=*object{
                    let vn=vn.clone();let mut av=Vec::new();for a in args{av.push(self.eval_expr(a)?);}
                    self.call_dotion_method(&vn,&method,av)
                }else{
                    let obj=self.eval_expr(*object)?;
                    match obj{
                        Value::Dotion{ref methods,..}=>{
                            let m=methods.iter().find(|m|m.name==method).cloned()
                                .ok_or_else(||runtime_error(&format!("No method '{}'",method)))?;
                            let mut av=Vec::new();for a in args{av.push(self.eval_expr(a)?);}
                            self.scope.push();self.scope.define("__self__",obj.clone());
                            for(p,v) in m.params.iter().zip(av.iter()){self.scope.define(p,v.clone());}
                            let mut ret=Value::Nil;
                            for s in m.body{match self.execute_stmt(s){ControlFlow::Return(v)=>{ret=v;break;}_=>{}}}
                            self.scope.pop();Ok(ret)
                        }
                        other=>Err(type_error(&format!("Cannot call method on {}",self.type_name(&other)))),
                    }
                }
            }
            Expr::StructInit{name,fields}=>{
                if let Some(def)=self.structs.get(&name).cloned(){for r in &def.fields{if !fields.iter().any(|(f,_)|f==r){return Err(runtime_error(&format!("Missing field '{}' in {} init",r,name)));}}}
                let mut fm=HashMap::new();for(fn_,fv) in fields{fm.insert(fn_,self.eval_expr(fv)?);}
                Ok(Value::Struct{name,fields:fm})
            }
            Expr::EnumVariant{enum_name,variant}=>{
                if let Some(def)=self.enums.get(&enum_name).cloned(){if !def.variants.contains(&variant){return Err(runtime_error(&format!("'{}' is not a variant of '{}'",variant,enum_name)));}}
                Ok(Value::EnumVariant{enum_name,variant})
            }
            Expr::Call{name,args}=>{let mut av=Vec::new();for a in args{av.push(self.eval_expr(a)?);}self.call_function(&name,av)}
            Expr::Unary{op,expr}=>{
                let v=self.eval_expr(*expr)?;
                match op.as_str(){
                    "!"=>Ok(Value::Bool(!self.is_truthy(&v))),
                    "-"=>match v{Value::Number(n)=>Ok(Value::Number(-n)),Value::Float(f)=>Ok(Value::Float(-f)),other=>Err(type_error(&format!("Unary minus requires num or float, got {}",self.type_name(&other))))},
                    _=>Err(runtime_error(&format!("Unknown unary '{}'",op)))
                }
            }
            Expr::Binary{left,op,right}=>{
                let l=self.eval_expr(*left)?;let r=self.eval_expr(*right)?;
                match op.as_str(){
                    "+"=>Ok(match(&l,&r){
                        (Value::Number(a),Value::Number(b))=>Value::Number(a+b),
                        (Value::Float(a),Value::Float(b))=>Value::Float(a+b),
                        (Value::Float(a),Value::Number(b))=>Value::Float(a+(*b as f64)),
                        (Value::Number(a),Value::Float(b))=>Value::Float((*a as f64)+b),
                        _=>Value::Str(format!("{}{}",self.val_to_string(&l),self.val_to_string(&r)))
                    }),
                    "-"=>match(l,r){
                        (Value::Number(a),Value::Number(b))=>Ok(Value::Number(a-b)),
                        (Value::Float(a),Value::Float(b))=>Ok(Value::Float(a-b)),
                        (Value::Float(a),Value::Number(b))=>Ok(Value::Float(a-(b as f64))),
                        (Value::Number(a),Value::Float(b))=>Ok(Value::Float((a as f64)-b)),
                        (l,r)=>Err(type_error(&format!("Cannot subtract {} from {}",self.type_name(&r),self.type_name(&l))))
                    },
                    "*"=>match(l,r){
                        (Value::Number(a),Value::Number(b))=>Ok(Value::Number(a*b)),
                        (Value::Float(a),Value::Float(b))=>Ok(Value::Float(a*b)),
                        (Value::Float(a),Value::Number(b))=>Ok(Value::Float(a*(b as f64))),
                        (Value::Number(a),Value::Float(b))=>Ok(Value::Float((a as f64)*b)),
                        (l,r)=>Err(type_error(&format!("Cannot multiply {} and {}",self.type_name(&l),self.type_name(&r))))
                    },
                    "/"=>match(l,r){
                        (Value::Number(a),Value::Number(b))=>{if b==0{Err(runtime_error("Division by zero"))}else{Ok(Value::Number(a/b))}}
                        (Value::Float(a),Value::Float(b))=>{if b==0.0{Err(runtime_error("Division by zero"))}else{Ok(Value::Float(a/b))}}
                        (Value::Float(a),Value::Number(b))=>{if b==0{Err(runtime_error("Division by zero"))}else{Ok(Value::Float(a/(b as f64)))}}
                        (Value::Number(a),Value::Float(b))=>{if b==0.0{Err(runtime_error("Division by zero"))}else{Ok(Value::Float((a as f64)/b))}}
                        (l,r)=>Err(type_error(&format!("Cannot divide {} and {}",self.type_name(&l),self.type_name(&r))))
                    },
                    "%"=>match(l,r){
                        (Value::Number(a),Value::Number(b))=>{if b==0{Err(runtime_error("Modulo by zero"))}else{Ok(Value::Number(a%b))}}
                        (Value::Float(a),Value::Float(b))=>{if b==0.0{Err(runtime_error("Modulo by zero"))}else{Ok(Value::Float(a%b))}}
                        (Value::Float(a),Value::Number(b))=>{if b==0{Err(runtime_error("Modulo by zero"))}else{Ok(Value::Float(a%(b as f64)))}}
                        (Value::Number(a),Value::Float(b))=>{if b==0.0{Err(runtime_error("Modulo by zero"))}else{Ok(Value::Float((a as f64)%b))}}
                        (l,r)=>Err(type_error(&format!("Cannot apply %% to {} and {}",self.type_name(&l),self.type_name(&r))))
                    },
                    "=="=>Ok(Value::Bool(self.val_to_string(&l)==self.val_to_string(&r))),
                    "!="=>Ok(Value::Bool(self.val_to_string(&l)!=self.val_to_string(&r))),
                    ">"=>match(l,r){(Value::Number(a),Value::Number(b))=>Ok(Value::Bool(a>b)),(Value::Float(a),Value::Float(b))=>Ok(Value::Bool(a>b)),(Value::Float(a),Value::Number(b))=>Ok(Value::Bool(a>(b as f64))),(Value::Number(a),Value::Float(b))=>Ok(Value::Bool((a as f64)>b)),(l,r)=>Err(type_error(&format!("Cannot compare {} > {}",self.type_name(&l),self.type_name(&r))))},
                    "<"=>match(l,r){(Value::Number(a),Value::Number(b))=>Ok(Value::Bool(a<b)),(Value::Float(a),Value::Float(b))=>Ok(Value::Bool(a<b)),(Value::Float(a),Value::Number(b))=>Ok(Value::Bool(a<(b as f64))),(Value::Number(a),Value::Float(b))=>Ok(Value::Bool((a as f64)<b)),(l,r)=>Err(type_error(&format!("Cannot compare {} < {}",self.type_name(&l),self.type_name(&r))))},
                    ">="=>match(l,r){(Value::Number(a),Value::Number(b))=>Ok(Value::Bool(a>=b)),(Value::Float(a),Value::Float(b))=>Ok(Value::Bool(a>=b)),(Value::Float(a),Value::Number(b))=>Ok(Value::Bool(a>=(b as f64))),(Value::Number(a),Value::Float(b))=>Ok(Value::Bool((a as f64)>=b)),(l,r)=>Err(type_error(&format!("Cannot compare {} >= {}",self.type_name(&l),self.type_name(&r))))},
                    "<="=>match(l,r){(Value::Number(a),Value::Number(b))=>Ok(Value::Bool(a<=b)),(Value::Float(a),Value::Float(b))=>Ok(Value::Bool(a<=b)),(Value::Float(a),Value::Number(b))=>Ok(Value::Bool(a<=(b as f64))),(Value::Number(a),Value::Float(b))=>Ok(Value::Bool((a as f64)<=b)),(l,r)=>Err(type_error(&format!("Cannot compare {} <= {}",self.type_name(&l),self.type_name(&r))))},
                    "&&"=>Ok(Value::Bool(self.is_truthy(&l)&&self.is_truthy(&r))),
                    "||"=>Ok(Value::Bool(self.is_truthy(&l)||self.is_truthy(&r))),
                    _=>Err(runtime_error(&format!("Unknown operator '{}'",op))),
                }
            }
        }
    }

    // ── MATH ──────────────────────────────────────────────────────────
    fn call_math(&self, name: &str, args: &[Value]) -> Result<Value,FluxisError> {
        let n1=|f:&str,args:&[Value]|->Result<i64,FluxisError>{if args.len()!=1{return Err(arity_error(f,1,args.len()));}match &args[0]{Value::Number(n)=>Ok(*n),_=>Err(type_error(&format!("{} expects num",f)))}};
        match name {
            "abs"       =>{Ok(match &args[0]{Value::Number(n)=>Value::Number(n.abs()),Value::Float(f)=>Value::Float(f.abs()),_=>return Err(type_error("abs() expects num or float"))})}
            "sign"      =>{let n=n1("sign",args)?;Ok(Value::Number(if n>0{1}else if n<0{-1}else{0}))}
            "sqrt"      =>{let n=match &args[0]{Value::Number(n)=>*n as f64,Value::Float(f)=>*f,_=>return Err(type_error("sqrt() expects num or float"))};if n<0.0{return Err(runtime_error("sqrt() of negative number").with_hint("Use abs() first if needed"));}Ok(Value::Float(n.sqrt()))}
            "floor"=>{Ok(match &args[0]{Value::Float(f)=>Value::Number(f.floor() as i64),Value::Number(n)=>Value::Number(*n),_=>return Err(type_error("floor() expects num or float"))})}
            "ceil" =>{Ok(match &args[0]{Value::Float(f)=>Value::Number(f.ceil() as i64),Value::Number(n)=>Value::Number(*n),_=>return Err(type_error("ceil() expects num or float"))})}
            "max"       =>{if args.len()!=2{return Err(arity_error("max",2,args.len()));}match(&args[0],&args[1]){(Value::Number(a),Value::Number(b))=>Ok(Value::Number(*a.max(b))),(Value::Float(a),Value::Float(b))=>Ok(Value::Float(a.max(*b))),(Value::Float(a),Value::Number(b))=>Ok(Value::Float(a.max(*b as f64))),(Value::Number(a),Value::Float(b))=>Ok(Value::Float((*a as f64).max(*b))),_=>Err(type_error("max() requires numbers or floats"))}}
            "min"       =>{if args.len()!=2{return Err(arity_error("min",2,args.len()));}match(&args[0],&args[1]){(Value::Number(a),Value::Number(b))=>Ok(Value::Number(*a.min(b))),(Value::Float(a),Value::Float(b))=>Ok(Value::Float(a.min(*b))),(Value::Float(a),Value::Number(b))=>Ok(Value::Float(a.min(*b as f64))),(Value::Number(a),Value::Float(b))=>Ok(Value::Float((*a as f64).min(*b))),_=>Err(type_error("min() requires numbers or floats"))}}
            "pow"       =>{if args.len()!=2{return Err(arity_error("pow",2,args.len()));}match(&args[0],&args[1]){(Value::Number(b),Value::Number(e))=>{if *e<0{return Err(runtime_error("pow() negative exponent"));}Ok(Value::Number(b.pow(*e as u32)))},(Value::Float(b),Value::Number(e))=>Ok(Value::Float(b.powi(*e as i32))),(Value::Float(b),Value::Float(e))=>Ok(Value::Float(b.powf(*e))),(Value::Number(b),Value::Float(e))=>Ok(Value::Float((*b as f64).powf(*e))),_=>Err(type_error("pow() requires numbers or floats"))}}
                        "clamp"     =>{if args.len()!=3{return Err(arity_error("clamp",3,args.len()));}match(&args[0],&args[1],&args[2]){(Value::Number(v),Value::Number(lo),Value::Number(hi))=>Ok(Value::Number((*v).clamp(*lo,*hi))),_=>Err(type_error("clamp() requires three numbers"))}}
            "rand"      =>{
                if args.len()!=2{return Err(arity_error("rand",2,args.len()));}
                match(&args[0],&args[1]){
                    (Value::Number(lo),Value::Number(hi))=>{
                        if lo>hi{return Err(runtime_error("rand() min must be <= max"));}
                        use std::time::{SystemTime,UNIX_EPOCH};
                        let s=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
                        let r=(s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407))>>17;
                        Ok(Value::Number(lo+(r%((hi-lo+1)as u64))as i64))
                    }
                    _=>Err(type_error("rand() requires two numbers"))
                }
            }
            "rand_float"=>{
                use std::time::{SystemTime,UNIX_EPOCH};
                let s=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
                let r=(s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407))>>33;
                Ok(Value::Float((r%1000000) as f64 / 1000000.0))
            }
            "log" =>{
                let f=match &args[0]{Value::Number(n)=>*n as f64,Value::Float(f)=>*f,_=>return Err(type_error("log() expects num or float"))};
                if f<=0.0{return Err(runtime_error("log() requires positive number").with_hint("log(x) is undefined for x <= 0"));}
                Ok(Value::Float(f.ln()))
            }
            "log2" =>{
                let f=match &args[0]{Value::Number(n)=>*n as f64,Value::Float(f)=>*f,_=>return Err(type_error("log2() expects num or float"))};
                if f<=0.0{return Err(runtime_error("log2() requires positive number"));}
                Ok(Value::Float(f.log2()))
            }
            "log10" =>{
                let f=match &args[0]{Value::Number(n)=>*n as f64,Value::Float(f)=>*f,_=>return Err(type_error("log10() expects num or float"))};
                if f<=0.0{return Err(runtime_error("log10() requires positive number"));}
                Ok(Value::Float(f.log10()))
            }
            "sin" =>{
                let f=match &args[0]{Value::Number(n)=>*n as f64,Value::Float(f)=>*f,_=>return Err(type_error("sin() expects num or float"))};
                Ok(Value::Float(f.sin()))
            }
            "cos" =>{
                let f=match &args[0]{Value::Number(n)=>*n as f64,Value::Float(f)=>*f,_=>return Err(type_error("cos() expects num or float"))};
                Ok(Value::Float(f.cos()))
            }
            "tan" =>{
                let f=match &args[0]{Value::Number(n)=>*n as f64,Value::Float(f)=>*f,_=>return Err(type_error("tan() expects num or float"))};
                Ok(Value::Float(f.tan()))
            }
            "pi" => Ok(Value::Float(std::f64::consts::PI)),
            _=>Err(runtime_error(&format!("Unknown math function '{}'",name))),
        }
    }

    // ── STRING ────────────────────────────────────────────────────────
    fn call_string(&self, name: &str, args: &[Value]) -> Result<Value,FluxisError> {
        let rs=|f:&str,args:&[Value],i:usize|->Result<String,FluxisError>{if args.len()<=i{return Err(arity_error(f,i+1,args.len()));}match &args[i]{Value::Str(s)=>Ok(s.clone()),_=>Err(type_error(&format!("{} arg {} must be string",f,i+1)))}};
        match name {
            "upper"      =>Ok(Value::Str(rs("upper",args,0)?.to_uppercase())),
            "lower"      =>Ok(Value::Str(rs("lower",args,0)?.to_lowercase())),
            "trim"       =>Ok(Value::Str(rs("trim",args,0)?.trim().to_string())),
            "str_len"    =>Ok(Value::Number(rs("str_len",args,0)?.len()as i64)),
            "contains"   =>{let s=rs("contains",args,0)?;let p=rs("contains",args,1)?;Ok(Value::Bool(s.contains(&*p)))}
            "starts_with"=>{let s=rs("starts_with",args,0)?;let p=rs("starts_with",args,1)?;Ok(Value::Bool(s.starts_with(&*p)))}
            "ends_with"  =>{let s=rs("ends_with",args,0)?;let p=rs("ends_with",args,1)?;Ok(Value::Bool(s.ends_with(&*p)))}
            "replace"    =>{let s=rs("replace",args,0)?;let f=rs("replace",args,1)?;let t=rs("replace",args,2)?;Ok(Value::Str(s.replace(&*f,&t)))}
            "split"      =>{let s=rs("split",args,0)?;let sep=rs("split",args,1)?;Ok(Value::Array(s.split(&*sep).map(|p|Value::Str(p.to_string())).collect()))}
            "join"       =>{if args.len()!=2{return Err(arity_error("join",2,args.len()));}let sep=rs("join",args,1)?;match &args[0]{Value::Array(a)=>Ok(Value::Str(a.iter().map(|v|self.val_to_string(v)).collect::<Vec<_>>().join(&sep))),_=>Err(type_error("join() first arg must be array"))}}
            "repeat"     =>{let s=rs("repeat",args,0)?;match &args.get(1){Some(Value::Number(n))=>Ok(Value::Str(s.repeat(*n as usize))),_=>Err(type_error("repeat() second arg must be num"))}}
            "char_at"    =>{let s=rs("char_at",args,0)?;match args.get(1){Some(Value::Number(i))=>Ok(s.chars().nth(*i as usize).map(|c|Value::Str(c.to_string())).unwrap_or(Value::Nil)),_=>Err(type_error("char_at() second arg must be num"))}}
            "pad_left"   =>{
                let s=rs("pad_left",args,0)?;
                let width=match args.get(1){Some(Value::Number(n))=>*n as usize,_=>return Err(type_error("pad_left() second arg must be num"))};
                let pad_char=match args.get(2){Some(Value::Str(c))=>c.chars().next().unwrap_or(' '),_=>' '};
                if s.len()>=width{return Ok(Value::Str(s));}
                let padding:String=std::iter::repeat(pad_char).take(width-s.len()).collect();
                Ok(Value::Str(format!("{}{}",padding,s)))
            }
            "pad_right"  =>{
                let s=rs("pad_right",args,0)?;
                let width=match args.get(1){Some(Value::Number(n))=>*n as usize,_=>return Err(type_error("pad_right() second arg must be num"))};
                let pad_char=match args.get(2){Some(Value::Str(c))=>c.chars().next().unwrap_or(' '),_=>' '};
                if s.len()>=width{return Ok(Value::Str(s));}
                let padding:String=std::iter::repeat(pad_char).take(width-s.len()).collect();
                Ok(Value::Str(format!("{}{}",s,padding)))
            }
            "parse_int"  =>{
                let s=rs("parse_int",args,0)?;
                match s.trim().parse::<i64>(){
                    Ok(n)=>Ok(Value::Number(n)),
                    Err(_)=>Ok(Value::Nil),
                }
            }
            "parse_float"=>{
                let s=rs("parse_float",args,0)?;
                match s.trim().parse::<f64>(){
                    Ok(f)=>Ok(Value::Float(f)),
                    Err(_)=>Ok(Value::Nil),
                }
            }
            _=>Err(runtime_error(&format!("Unknown string function '{}'",name))),
        }
    }

    // ── IO ────────────────────────────────────────────────────────────
    fn call_io(&self, name: &str, args: &[Value]) -> Result<Value,FluxisError> {
        match name {
            "read_line" =>{
                use std::io::{self,Write};
                let mut s=String::new();
                io::stdout().flush().ok();
                io::stdin().read_line(&mut s).ok();
                Ok(Value::Str(s.trim_end_matches('\n').trim_end_matches('\r').to_string()))
            }
            "print_err" =>{
                if args.len()!=1{return Err(arity_error("print_err",1,args.len()));}
                eprintln!("{}",self.val_to_string(&args[0]));
                Ok(Value::Nil)
            }
            "read_file" =>{
                if args.len()!=1{return Err(arity_error("read_file",1,args.len()));}
                let path=match &args[0]{Value::Str(s)=>s.clone(),_=>return Err(type_error("read_file() expects string path"))};
                match std::fs::read_to_string(&path){
                    Ok(content)=>Ok(Value::Str(content)),
                    Err(e)=>Err(runtime_error(&format!("read_file(\"{}\") failed: {}",path,e))
                        .with_hint("Check that the file exists and you have read permission")),
                }
            }
            "write_file"=>{
                if args.len()!=2{return Err(arity_error("write_file",2,args.len()));}
                let path=match &args[0]{Value::Str(s)=>s.clone(),_=>return Err(type_error("write_file() first arg must be string path"))};
                let content=self.val_to_string(&args[1]);
                match std::fs::write(&path,content){
                    Ok(_)=>Ok(Value::Nil),
                    Err(e)=>Err(runtime_error(&format!("write_file(\"{}\") failed: {}",path,e))),
                }
            }
            "append_file"=>{
                if args.len()!=2{return Err(arity_error("append_file",2,args.len()));}
                let path=match &args[0]{Value::Str(s)=>s.clone(),_=>return Err(type_error("append_file() first arg must be string path"))};
                let content=self.val_to_string(&args[1]);
                use std::io::Write;
                match std::fs::OpenOptions::new().create(true).append(true).open(&path){
                    Ok(mut f)=>{ f.write_all(content.as_bytes()).ok(); Ok(Value::Nil) }
                    Err(e)=>Err(runtime_error(&format!("append_file(\"{}\") failed: {}",path,e))),
                }
            }
            "file_exists"=>{
                if args.len()!=1{return Err(arity_error("file_exists",1,args.len()));}
                let path=match &args[0]{Value::Str(s)=>s.clone(),_=>return Err(type_error("file_exists() expects string path"))};
                Ok(Value::Bool(std::path::Path::new(&path).exists()))
            }
            "exit" =>{
                let code=match args.first(){
                    Some(Value::Number(n))=>*n as i32,
                    None=>0,
                    _=>return Err(type_error("exit() expects optional num exit code")),
                };
                std::process::exit(code);
            }
            "sleep" =>{
                if args.len()!=1{return Err(arity_error("sleep",1,args.len()));}
                let ms=match &args[0]{
                    Value::Number(n)=>*n as u64,
                    Value::Float(f)=>*f as u64,
                    _=>return Err(type_error("sleep() expects num (milliseconds)")),
                };
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(Value::Nil)
            }
            "time_now"  =>{
                use std::time::{SystemTime,UNIX_EPOCH};
                let ms=SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                Ok(Value::Number(ms))
            }
            _=>Err(runtime_error(&format!("Unknown io function '{}'",name))),
        }
    }

    // ── AI / LLM ──────────────────────────────────────────────────────
    // Uses curl subprocess — works on Termux without extra crates.
    fn call_ai(&mut self, name: &str, args: &[Value]) -> Result<Value,FluxisError> {
        use std::process::Command;
        let get_key=||std::env::var("FLUXIS_AI_KEY").unwrap_or_default();
        let no_key=||runtime_error("No AI key set. Use: ai_set_key(\"sk-ant-...\")..")
            .with_hint("Get a key at console.anthropic.com");
        let extract_text=|resp:&str|->String{
            if let Some(start)=resp.find("\"text\":\""){
                let after=&resp[start+8..];
                if let Some(end)=after.find("\"}"){
                    return after[..end].replace("\\n","\n").replace("\\\"","\"").replace("\\\\","\\");
                }
            }
            resp.to_string()
        };
        let call_api=|body:&str,key:&str|->Result<String,FluxisError>{
            let out=Command::new("curl")
                .args(["-s","-X","POST",
                    "https://api.anthropic.com/v1/messages",
                    "-H","content-type: application/json",
                    "-H",&format!("x-api-key: {}",key),
                    "-H","anthropic-version: 2023-06-01",
                    "-d",body])
                .output()
                .map_err(|e|runtime_error(&format!("curl failed: {}. Install with: pkg install curl",e)))?;
            let resp=String::from_utf8_lossy(&out.stdout).to_string();
            if resp.contains("\"error\"")&&resp.contains("\"type\""){
                return Err(runtime_error(&format!("AI API error: check your key and internet connection. Response: {}",&resp[..resp.len().min(200)])));
            }
            Ok(resp)
        };
        match name {
            "ai_set_key"=>{
                if args.len()!=1{return Err(arity_error("ai_set_key",1,args.len()));}
                unsafe { std::env::set_var("FLUXIS_AI_KEY", self.val_to_string(&args[0])); }
                Ok(Value::Str("API key set".to_string()))
            }
            "ai_ask"=>{
                if args.len()!=1{return Err(arity_error("ai_ask",1,args.len()));}
                let key=get_key();if key.is_empty(){return Err(no_key());}
                let prompt=self.val_to_string(&args[0]).replace('"', "'").replace('\\', " ");
                let body=format!(r#"{{"model":"claude-haiku-4-5-20251001","max_tokens":1024,"messages":[{{"role":"user","content":"{}"}}]}}"#,prompt);
                let resp=call_api(&body,&key)?;
                Ok(Value::Str(extract_text(&resp)))
            }
            "ai_model"=>{
                if args.len()!=2{return Err(arity_error("ai_model",2,args.len()));}
                let key=get_key();if key.is_empty(){return Err(no_key());}
                let model_alias=self.val_to_string(&args[0]);
                let model=match model_alias.as_str(){"haiku"=>"claude-haiku-4-5-20251001","sonnet"=>"claude-sonnet-4-6","opus"=>"claude-opus-4-6",other=>other};
                let prompt=self.val_to_string(&args[1]).replace('"', "'").replace('\\', " ");
                let body=format!(r#"{{"model":"{}","max_tokens":2048,"messages":[{{"role":"user","content":"{}"}}]}}"#,model,prompt);
                let resp=call_api(&body,&key)?;
                Ok(Value::Str(extract_text(&resp)))
            }
            "ai_chat"=>{
                if args.len()!=2{return Err(arity_error("ai_chat",2,args.len()));}
                let key=get_key();if key.is_empty(){return Err(no_key());}
                let new_msg=self.val_to_string(&args[1]).replace('"', "'");
                let mut messages=String::new();
                let roles=["user","assistant"];
                if let Value::Array(hist)=&args[0]{
                    for(i,v) in hist.iter().enumerate(){
                        if !messages.is_empty(){messages.push(',');}
                        messages.push_str(&format!(r#"{{"role":"{}","content":"{}"}}"#,roles[i%2],self.val_to_string(v).replace('"', "'")));
                    }
                }
                if !messages.is_empty(){messages.push(',');}
                messages.push_str(&format!(r#"{{"role":"user","content":"{}"}}"#,new_msg));
                let body=format!(r#"{{"model":"claude-haiku-4-5-20251001","max_tokens":1024,"messages":[{}]}}"#,messages);
                let resp=call_api(&body,&key)?;
                Ok(Value::Str(extract_text(&resp)))
            }
            _=>Err(runtime_error(&format!("Unknown ai function '{}'",name))),
        }
    }

        // ── ML / AI MATH LIBRARY ──────────────────────────────────────────
    // ALL VALUES ARE REAL f64 FLOATS — no ×1000 scaling.
    // Matrices = Value::Array of Value::Array of Value::Float
    fn call_ml(&mut self, name: &str, args: &[Value]) -> Result<Value, FluxisError> {
        // ── Helpers ────────────────────────────────────────────────────────
        // Extract f64 from any numeric Value (Number or Float)
        let fv = |v: &Value| -> f64 {
            match v { Value::Float(f) => *f, Value::Number(n) => *n as f64, _ => 0.0 }
        };
        // Flatten any Value into Vec<f64>  (handles Number AND Float)
        let flat = |v: &Value| -> Vec<f64> {
            match v {
                Value::Array(a) => a.iter().map(|x| match x {
                    Value::Float(f) => *f,
                    Value::Number(n) => *n as f64,
                    _ => 0.0
                }).collect(),
                Value::Float(f) => vec![*f],
                Value::Number(n) => vec![*n as f64],
                _ => vec![]
            }
        };
        // Extract 2-D matrix as Vec<Vec<f64>>
        let mat = |v: &Value| -> Vec<Vec<f64>> {
            match v {
                Value::Array(rows) => rows.iter().map(|r| match r {
                    Value::Array(cells) => cells.iter().map(|c| match c {
                        Value::Float(f) => *f, Value::Number(n) => *n as f64, _ => 0.0
                    }).collect(),
                    _ => vec![]
                }).collect(),
                _ => vec![]
            }
        };
        // Wrap Vec<f64> as Value::Array of Value::Float
        let arr = |v: Vec<f64>| -> Value {
            Value::Array(v.into_iter().map(Value::Float).collect())
        };
        // Wrap Vec<Vec<f64>> as 2-D matrix
        let mat2d = |m: Vec<Vec<f64>>| -> Value {
            Value::Array(m.into_iter().map(|r|
                Value::Array(r.into_iter().map(Value::Float).collect())
            ).collect())
        };
        // Sigmoid on a single f64
        let sig = |x: f64| -> f64 { 1.0 / (1.0 + (-x).exp()) };

        match name {
            // ── Matrix creation ───────────────────────────────────────
            "ml_zeros" => {
                if args.len()!=2{return Err(arity_error("ml_zeros",2,args.len()));}
                let (r,c) = (fv(&args[0]) as usize, fv(&args[1]) as usize);
                Ok(mat2d(vec![vec![0.0;c];r]))
            }
            "ml_ones" => {
                if args.len()!=2{return Err(arity_error("ml_ones",2,args.len()));}
                let (r,c) = (fv(&args[0]) as usize, fv(&args[1]) as usize);
                Ok(mat2d(vec![vec![1.0;c];r]))
            }
            "ml_identity" => {
                if args.len()!=1{return Err(arity_error("ml_identity",1,args.len()));}
                let n = fv(&args[0]) as usize;
                let mut m = vec![vec![0.0;n];n];
                for i in 0..n { m[i][i] = 1.0; }
                Ok(mat2d(m))
            }
            "ml_new" => {
                if args.len()!=3{return Err(arity_error("ml_new",3,args.len()));}
                let (r,c,v) = (fv(&args[0]) as usize, fv(&args[1]) as usize, fv(&args[2]));
                Ok(mat2d(vec![vec![v;c];r]))
            }
            // ── Matrix access ─────────────────────────────────────────
            "ml_get" => {
                if args.len()!=3{return Err(arity_error("ml_get",3,args.len()));}
                let m = mat(&args[0]);
                let (r,c) = (fv(&args[1]) as usize, fv(&args[2]) as usize);
                Ok(Value::Float(m.get(r).and_then(|row|row.get(c)).copied().unwrap_or(0.0)))
            }
            "ml_set" => {
                if args.len()!=4{return Err(arity_error("ml_set",4,args.len()));}
                let mut m = mat(&args[0]);
                let (r,c,v) = (fv(&args[1]) as usize, fv(&args[2]) as usize, fv(&args[3]));
                if r<m.len() && c<m[r].len() { m[r][c] = v; }
                Ok(mat2d(m))
            }
            "ml_shape" => {
                if args.len()!=1{return Err(arity_error("ml_shape",1,args.len()));}
                let m = mat(&args[0]);
                let r = m.len() as f64;
                let c = m.first().map(|r|r.len()).unwrap_or(0) as f64;
                Ok(Value::Array(vec![Value::Float(r), Value::Float(c)]))
            }
            // ── Matrix arithmetic ─────────────────────────────────────
            "ml_add" => {
                if args.len()!=2{return Err(arity_error("ml_add",2,args.len()));}
                let (a,b) = (mat(&args[0]), mat(&args[1]));
                Ok(mat2d(a.iter().zip(b.iter()).map(|(ar,br)|
                    ar.iter().zip(br.iter()).map(|(x,y)| x+y).collect()
                ).collect()))
            }
            "ml_sub" => {
                if args.len()!=2{return Err(arity_error("ml_sub",2,args.len()));}
                let (a,b) = (mat(&args[0]), mat(&args[1]));
                Ok(mat2d(a.iter().zip(b.iter()).map(|(ar,br)|
                    ar.iter().zip(br.iter()).map(|(x,y)| x-y).collect()
                ).collect()))
            }
            "ml_mul" => {
                if args.len()!=2{return Err(arity_error("ml_mul",2,args.len()));}
                let (a,b) = (mat(&args[0]), mat(&args[1]));
                Ok(mat2d(a.iter().zip(b.iter()).map(|(ar,br)|
                    ar.iter().zip(br.iter()).map(|(x,y)| x*y).collect()
                ).collect()))
            }
            "ml_scale" => {
                if args.len()!=2{return Err(arity_error("ml_scale",2,args.len()));}
                let (m,s) = (mat(&args[0]), fv(&args[1]));
                Ok(mat2d(m.iter().map(|r| r.iter().map(|x| x*s).collect()).collect()))
            }
            "ml_matmul" => {
                if args.len()!=2{return Err(arity_error("ml_matmul",2,args.len()));}
                let (a,b) = (mat(&args[0]), mat(&args[1]));
                let (ra,ca) = (a.len(), a.first().map(|r|r.len()).unwrap_or(0));
                let cb = b.first().map(|r|r.len()).unwrap_or(0);
                if ca != b.len() { return Err(runtime_error(&format!("ml_matmul: shape mismatch {}x{} vs {}x{}", ra,ca,b.len(),cb)).with_hint("cols of A must equal rows of B")); }
                let mut res = vec![vec![0.0f64; cb]; ra];
                for i in 0..ra { for j in 0..cb { for k in 0..ca { res[i][j] += a[i][k]*b[k][j]; } } }
                Ok(mat2d(res))
            }
            "ml_transpose" => {
                if args.len()!=1{return Err(arity_error("ml_transpose",1,args.len()));}
                let m = mat(&args[0]);
                let (rows,cols) = (m.len(), m.first().map(|r|r.len()).unwrap_or(0));
                let mut t = vec![vec![0.0f64; rows]; cols];
                for i in 0..rows { for j in 0..cols { t[j][i] = m[i][j]; } }
                Ok(mat2d(t))
            }
            "ml_dot" => {
                if args.len()!=2{return Err(arity_error("ml_dot",2,args.len()));}
                let (a,b) = (flat(&args[0]), flat(&args[1]));
                Ok(Value::Float(a.iter().zip(b.iter()).map(|(x,y)| x*y).sum()))
            }
            // ── Activations (all real float) ──────────────────────────
            "sigmoid" => {
                if args.len()!=1{return Err(arity_error("sigmoid",1,args.len()));}
                Ok(Value::Float(sig(fv(&args[0]))))
            }
            "sigmoid_arr" => {
                if args.len()!=1{return Err(arity_error("sigmoid_arr",1,args.len()));}
                Ok(arr(flat(&args[0]).iter().map(|&x| sig(x)).collect()))
            }
            "sigmoid_deriv" => {
                if args.len()!=1{return Err(arity_error("sigmoid_deriv",1,args.len()));}
                let x = fv(&args[0]);
                let s = sig(x);
                Ok(Value::Float(s*(1.0-s)))
            }
            "sigmoid_deriv_from_output" => {
                if args.len()!=1{return Err(arity_error("sigmoid_deriv_from_output",1,args.len()));}
                let s = fv(&args[0]);
                Ok(Value::Float(s*(1.0-s)))
            }
            "sigmoid_deriv_arr" => {
                if args.len()!=1{return Err(arity_error("sigmoid_deriv_arr",1,args.len()));}
                Ok(arr(flat(&args[0]).iter().map(|&s| s*(1.0-s)).collect()))
            }
            "relu" => {
                if args.len()!=1{return Err(arity_error("relu",1,args.len()));}
                Ok(Value::Float(fv(&args[0]).max(0.0)))
            }
            "relu_arr" => {
                if args.len()!=1{return Err(arity_error("relu_arr",1,args.len()));}
                Ok(arr(flat(&args[0]).iter().map(|&x| x.max(0.0)).collect()))
            }
            "leaky_relu" => {
                if args.len()!=2{return Err(arity_error("leaky_relu",2,args.len()));}
                let (x,a) = (fv(&args[0]), fv(&args[1]));
                Ok(Value::Float(if x >= 0.0 { x } else { a*x }))
            }
            "tanh_f" => {
                if args.len()!=1{return Err(arity_error("tanh_f",1,args.len()));}
                Ok(Value::Float(fv(&args[0]).tanh()))
            }
            "softmax" => {
                if args.len()!=1{return Err(arity_error("softmax",1,args.len()));}
                let a = flat(&args[0]);
                let max_v = a.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let exps: Vec<f64> = a.iter().map(|x| (x - max_v).exp()).collect();
                let sum: f64 = exps.iter().sum();
                Ok(arr(exps.iter().map(|&e| e/sum).collect()))
            }
            "linear" => {
                if args.len()!=1{return Err(arity_error("linear",1,args.len()));}
                Ok(args[0].clone())
            }
            // ── Loss functions ────────────────────────────────────────
            "ml_mse" => {
                if args.len()!=2{return Err(arity_error("ml_mse",2,args.len()));}
                let (p,a) = (flat(&args[0]), flat(&args[1]));
                let n = p.len() as f64;
                if n == 0.0 { return Ok(Value::Float(0.0)); }
                Ok(Value::Float(p.iter().zip(a.iter()).map(|(pi,ai)| (pi-ai).powi(2)).sum::<f64>() / n))
            }
            "ml_mae" => {
                if args.len()!=2{return Err(arity_error("ml_mae",2,args.len()));}
                let (p,a) = (flat(&args[0]), flat(&args[1]));
                let n = p.len() as f64;
                if n == 0.0 { return Ok(Value::Float(0.0)); }
                Ok(Value::Float(p.iter().zip(a.iter()).map(|(pi,ai)| (pi-ai).abs()).sum::<f64>() / n))
            }
            "ml_cross_entropy" => {
                if args.len()!=2{return Err(arity_error("ml_cross_entropy",2,args.len()));}
                let (p,a) = (flat(&args[0]), flat(&args[1]));
                let n = p.len() as f64;
                if n == 0.0 { return Ok(Value::Float(0.0)); }
                let loss: f64 = p.iter().zip(a.iter()).map(|(&pi,&ai)| {
                    let p_clamped = pi.max(1e-9).min(1.0 - 1e-9);
                    -ai * p_clamped.ln() - (1.0-ai) * (1.0-p_clamped).ln()
                }).sum();
                Ok(Value::Float(loss / n))
            }
            // ── Statistics ────────────────────────────────────────────
            "ml_mean" => {
                if args.len()!=1{return Err(arity_error("ml_mean",1,args.len()));}
                let a = flat(&args[0]);
                let n = a.len() as f64;
                Ok(Value::Float(if n==0.0 {0.0} else {a.iter().sum::<f64>()/n}))
            }
            "ml_sum" => {
                if args.len()!=1{return Err(arity_error("ml_sum",1,args.len()));}
                Ok(Value::Float(flat(&args[0]).iter().sum()))
            }
            "ml_max_val" => {
                if args.len()!=1{return Err(arity_error("ml_max_val",1,args.len()));}
                let a = flat(&args[0]);
                Ok(Value::Float(a.iter().cloned().fold(f64::NEG_INFINITY, f64::max)))
            }
            "ml_min" => {
                if args.len()!=1{return Err(arity_error("ml_min",1,args.len()));}
                let a = flat(&args[0]);
                Ok(Value::Float(a.iter().cloned().fold(f64::INFINITY, f64::min)))
            }
            "ml_std" => {
                if args.len()!=1{return Err(arity_error("ml_std",1,args.len()));}
                                let a = flat(&args[0]);
                let n = a.len() as f64;
                if n==0.0 { return Ok(Value::Float(0.0)); }
                let mean = a.iter().sum::<f64>() / n;
                let var  = a.iter().map(|x| (x-mean).powi(2)).sum::<f64>() / n;
                Ok(Value::Float(var.sqrt()))
            }
            "ml_normalize" => {
                if args.len()!=1{return Err(arity_error("ml_normalize",1,args.len()));}
                let a = flat(&args[0]);
                let mn = a.iter().cloned().fold(f64::INFINITY, f64::min);
                let mx = a.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let range = mx - mn;
                if range == 0.0 { return Ok(arr(vec![0.0; a.len()])); }
                Ok(arr(a.iter().map(|x| (x-mn)/range).collect()))
            }
            "ml_flatten" => {
                if args.len()!=1{return Err(arity_error("ml_flatten",1,args.len()));}
                Ok(arr(mat(&args[0]).into_iter().flatten().collect()))
            }
            // ── Neural network ────────────────────────────────────────
            // ml_layer_forward(inputs: [f64], weights: [[f64]], biases: [f64]) → [f64]
            "ml_layer_forward" => {
                if args.len()!=3{return Err(arity_error("ml_layer_forward",3,args.len()));}
                let inputs  = flat(&args[0]);
                let weights = mat(&args[1]);
                let biases  = flat(&args[2]);
                let outputs: Vec<f64> = (0..weights.len()).map(|i| {
                    weights[i].iter().zip(inputs.iter()).map(|(wi,xi)| wi*xi).sum::<f64>()
                    + biases.get(i).copied().unwrap_or(0.0)
                }).collect();
                Ok(arr(outputs))
            }
            // ml_grad_desc_step(weights, grads, lr) → new weights
            "ml_grad_desc_step" => {
                if args.len()!=3{return Err(arity_error("ml_grad_desc_step",3,args.len()));}
                let (w,g,lr) = (mat(&args[0]), mat(&args[1]), fv(&args[2]));
                Ok(mat2d(w.iter().zip(g.iter()).map(|(wr,gr)|
                    wr.iter().zip(gr.iter()).map(|(wi,gi)| wi - lr*gi).collect()
                ).collect()))
            }
            // ml_random_weights(rows, cols) → Xavier-init float matrix
            "ml_random_weights" => {
                if args.len()!=2{return Err(arity_error("ml_random_weights",2,args.len()));}
                let (r,c) = (fv(&args[0]) as usize, fv(&args[1]) as usize);
                use std::time::{SystemTime, UNIX_EPOCH};
                let mut seed = SystemTime::now().duration_since(UNIX_EPOCH)
                    .unwrap_or_default().subsec_nanos() as u64;
                // Xavier: range [-sqrt(6/(r+c)), +sqrt(6/(r+c))]
                let limit = (6.0f64 / (r + c) as f64).sqrt();
                let mut rng = || {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    let u = (seed >> 11) as f64 / (1u64 << 53) as f64; // uniform [0,1)
                    u * 2.0 * limit - limit
                };
                Ok(mat2d((0..r).map(|_| (0..c).map(|_| rng()).collect()).collect()))
            }
            // ── Backprop helpers ──────────────────────────────────────
            "ml_outer" => {
                if args.len()!=2{return Err(arity_error("ml_outer",2,args.len()));}
                let (a,b) = (flat(&args[0]), flat(&args[1]));
                Ok(mat2d(a.iter().map(|&ai| b.iter().map(|&bi| ai*bi).collect()).collect()))
            }
            "ml_vec_add" => {
                if args.len()!=2{return Err(arity_error("ml_vec_add",2,args.len()));}
                let (a,b) = (flat(&args[0]), flat(&args[1]));
                Ok(arr(a.iter().zip(b.iter()).map(|(x,y)| x+y).collect()))
            }
            "ml_vec_sub" => {
                if args.len()!=2{return Err(arity_error("ml_vec_sub",2,args.len()));}
                let (a,b) = (flat(&args[0]), flat(&args[1]));
                Ok(arr(a.iter().zip(b.iter()).map(|(x,y)| x-y).collect()))
            }
            "ml_vec_mul" => {
                if args.len()!=2{return Err(arity_error("ml_vec_mul",2,args.len()));}
                let (a,b) = (flat(&args[0]), flat(&args[1]));
                Ok(arr(a.iter().zip(b.iter()).map(|(x,y)| x*y).collect()))
            }
            "ml_vec_scale" => {
                if args.len()!=2{return Err(arity_error("ml_vec_scale",2,args.len()));}
                let (a,s) = (flat(&args[0]), fv(&args[1]));
                Ok(arr(a.iter().map(|x| x*s).collect()))
            }
            "ml_mat_T_vec" => {
                if args.len()!=2{return Err(arity_error("ml_mat_T_vec",2,args.len()));}
                let (m,v) = (mat(&args[0]), flat(&args[1]));
                let (rows,cols) = (m.len(), m.first().map(|r|r.len()).unwrap_or(0));
                let mut result = vec![0.0f64; cols];
                for i in 0..rows.min(v.len()) { for j in 0..cols { result[j] += m[i][j]*v[i]; } }
                Ok(arr(result))
            }
            "ml_bias_update" => {
                if args.len()!=3{return Err(arity_error("ml_bias_update",3,args.len()));}
                let (b,g,lr) = (flat(&args[0]), flat(&args[1]), fv(&args[2]));
                Ok(arr(b.iter().zip(g.iter()).map(|(bi,gi)| bi - lr*gi).collect()))
            }
            "ml_vec_to_mat" => {
                if args.len()!=1{return Err(arity_error("ml_vec_to_mat",1,args.len()));}
                Ok(mat2d(vec![flat(&args[0])]))
            }
            "ml_mat_to_vec" => {
                if args.len()!=1{return Err(arity_error("ml_mat_to_vec",1,args.len()));}
                Ok(arr(mat(&args[0]).into_iter().next().unwrap_or_default()))
            }
            // ── Printing ──────────────────────────────────────────────
            "ml_print_mat" => {
                if args.len()!=1{return Err(arity_error("ml_print_mat",1,args.len()));}
                let m = mat(&args[0]);
                println!("Matrix {}×{}:", m.len(), m.first().map(|r|r.len()).unwrap_or(0));
                for row in &m {
                    let s: Vec<String> = row.iter().map(|x| format!("{:8.4}", x)).collect();
                    println!("  [{}]", s.join("  "));
                }
                Ok(Value::Nil)
            }
            _ => Err(runtime_error(&format!("Unknown ml function '{}'", name))),
        }
    }

        // ── 2D GRAPHICS ───────────────────────────────────────────────────
    // Two backends:
    //   Terminal canvas (gfx_canvas, gfx_pixel, gfx_render) — ANSI chars in terminal
    //   PPM image (gfx_image, gfx_set_pixel, gfx_save)       — writes .ppm file
    fn call_gfx(&mut self, name: &str, args: &[Value]) -> Result<Value,FluxisError> {
        let num=|v:&Value|->i64{match v{Value::Number(n)=>*n,_=>0}};
        match name {
            // ── TERMINAL CANVAS ───────────────────────────────────────
            // gfx_canvas(width, height) — create a blank canvas
            "gfx_canvas"=>{
                if args.len()!=2{return Err(arity_error("gfx_canvas",2,args.len()));}
                let(w,h)=(num(&args[0])as usize,num(&args[1])as usize);
                self.canvas=Some(vec![vec![' ';w];h]);
                self.canvas_w=w;self.canvas_h=h;
                Ok(Value::Nil)
            }
            // gfx_clear() — fill canvas with spaces
            "gfx_clear"=>{
                if let Some(ref mut c)=self.canvas{for row in c.iter_mut(){for cell in row.iter_mut(){*cell=' ';}}}
                Ok(Value::Nil)
            }
            // gfx_pixel(x, y, char_str) — draw a character
            "gfx_pixel"=>{
                if args.len()!=3{return Err(arity_error("gfx_pixel",3,args.len()));}
                let(x,y)=(num(&args[0])as usize,num(&args[1])as usize);
                let ch=match &args[2]{Value::Str(s)=>s.chars().next().unwrap_or('#'),Value::Number(n)=>char::from_u32(*n as u32).unwrap_or('#'),_=>'#'};
                if let Some(ref mut c)=self.canvas{if y<c.len()&&x<c[y].len(){c[y][x]=ch;}}
                Ok(Value::Nil)
            }
            // gfx_text(x, y, text_str) — draw text
            "gfx_text"=>{
                if args.len()!=3{return Err(arity_error("gfx_text",3,args.len()));}
                let(x,y)=(num(&args[0])as usize,num(&args[1])as usize);
                let text=match &args[2]{Value::Str(s)=>s.clone(),other=>self.val_to_string(other)};
                if let Some(ref mut c)=self.canvas{
                    if y<c.len(){for(i,ch) in text.chars().enumerate(){if x+i<c[y].len(){c[y][x+i]=ch;}}}
                }
                Ok(Value::Nil)
            }
            // gfx_rect(x, y, w, h, char_str) — draw rectangle outline
            "gfx_rect"=>{
                if args.len()!=5{return Err(arity_error("gfx_rect",5,args.len()));}
                let(x,y,w,h)=(num(&args[0])as usize,num(&args[1])as usize,num(&args[2])as usize,num(&args[3])as usize);
                let ch=match &args[4]{Value::Str(s)=>s.chars().next().unwrap_or('#'),_=>'#'};
                if let Some(ref mut c)=self.canvas{
                    for xi in x..x+w{if y<c.len()&&xi<c[y].len(){c[y][xi]=ch;}if y+h-1<c.len()&&xi<c[y+h-1].len(){c[y+h-1][xi]=ch;}}
                    for yi in y..y+h{if yi<c.len(){if x<c[yi].len(){c[yi][x]=ch;}if x+w-1<c[yi].len(){c[yi][x+w-1]=ch;}}}
                }
                Ok(Value::Nil)
            }
            // gfx_fill_rect(x, y, w, h, char_str) — filled rectangle
            "gfx_fill_rect"=>{
                if args.len()!=5{return Err(arity_error("gfx_fill_rect",5,args.len()));}
                let(x,y,w,h)=(num(&args[0])as usize,num(&args[1])as usize,num(&args[2])as usize,num(&args[3])as usize);
                let ch=match &args[4]{Value::Str(s)=>s.chars().next().unwrap_or('#'),_=>'#'};
                if let Some(ref mut c)=self.canvas{
                    for yi in y..y+h{if yi<c.len(){for xi in x..x+w{if xi<c[yi].len(){c[yi][xi]=ch;}}}}
                }
                Ok(Value::Nil)
            }
            // gfx_line(x1, y1, x2, y2, char_str) — Bresenham's line
            "gfx_line"=>{
                if args.len()!=5{return Err(arity_error("gfx_line",5,args.len()));}
                let(mut x0,mut y0,x1,y1)=(num(&args[0])as i64,num(&args[1])as i64,num(&args[2])as i64,num(&args[3])as i64);
                let ch=match &args[4]{Value::Str(s)=>s.chars().next().unwrap_or('*'),_=>'*'};
                let(dx,dy)=((x1-x0).abs(),(y1-y0).abs());
                let(sx,sy)=(if x0<x1{1}else{-1},if y0<y1{1}else{-1});
                let mut err=dx-dy;
                if let Some(ref mut c)=self.canvas{
                    loop{
                        let(xi,yi)=(x0 as usize,y0 as usize);
                        if yi<c.len()&&xi<c[yi].len(){c[yi][xi]=ch;}
                        if x0==x1&&y0==y1{break;}
                        let e2=2*err;
                        if e2>-dy{err-=dy;x0+=sx;}
                        if e2<dx{err+=dx;y0+=sy;}
                    }
                }
                Ok(Value::Nil)
            }
            // gfx_circle(cx, cy, r, char_str) — midpoint circle
                        "gfx_circle"=>{
                if args.len()!=4{return Err(arity_error("gfx_circle",4,args.len()));}
                let(cx,cy,r)=(num(&args[0]),num(&args[1]),num(&args[2]));
                let ch=match &args[3]{Value::Str(s)=>s.chars().next().unwrap_or('o'),_=>'o'};
                let plot=|c:&mut Vec<Vec<char>>,x:i64,y:i64|{let(xi,yi)=(x as usize,y as usize);if yi<c.len()&&xi<c[yi].len(){c[yi][xi]=ch;}};
                if let Some(ref mut c)=self.canvas{
                    let(mut x,mut y,mut d)=(0i64,r,1-r);
                    while x<=y{
                        plot(c,cx+x,cy+y);plot(c,cx-x,cy+y);plot(c,cx+x,cy-y);plot(c,cx-x,cy-y);
                        plot(c,cx+y,cy+x);plot(c,cx-y,cy+x);plot(c,cx+y,cy-x);plot(c,cx-y,cy-x);
                        if d<0{d+=2*x+3;}else{d+=2*(x-y)+5;y-=1;}
                        x+=1;
                    }
                }
                Ok(Value::Nil)
            }
            // gfx_render() — print canvas to terminal with border
            "gfx_render"=>{
                if let Some(ref c)=self.canvas{
                    let w=c.first().map(|r|r.len()).unwrap_or(0);
                    println!("\x1b[2J\x1b[H"); // clear terminal
                    print!("+");for _ in 0..w{print!("-");}println!("+");
                    for row in c{print!("|");for ch in row{print!("{}",ch);}println!("|");}
                    print!("+");for _ in 0..w{print!("-");}println!("+");
                }
                Ok(Value::Nil)
            }
            // gfx_reset() — clear canvas back to blank
            "gfx_reset"=>{self.canvas=None;self.canvas_w=0;self.canvas_h=0;Ok(Value::Nil)}

            // ── PPM IMAGE BACKEND ─────────────────────────────────────
            // gfx_image(width, height) → image_array (flat array of r,g,b triples stored as [w,h,pixels...])
            "gfx_image"=>{
                if args.len()!=2{return Err(arity_error("gfx_image",2,args.len()));}
                let(w,h)=(num(&args[0]),num(&args[1]));
                // Encode as [width, height, r,g,b, r,g,b, ...] for w*h pixels (black)
                let mut img=vec![Value::Number(w),Value::Number(h)];
                for _ in 0..w*h{img.push(Value::Number(0));img.push(Value::Number(0));img.push(Value::Number(0));}
                Ok(Value::Array(img))
            }
            // gfx_set_pixel(img, x, y, r, g, b) → new img
            "gfx_set_pixel"=>{
                if args.len()!=6{return Err(arity_error("gfx_set_pixel",6,args.len()));}
                if let Value::Array(mut img)=args[0].clone(){
                    let w=num(&img[0]);let h=num(&img[1]);
                    let(x,y,r,g,b)=(num(&args[1]),num(&args[2]),num(&args[3]),num(&args[4]),num(&args[5]));
                    if x>=0&&y>=0&&x<w&&y<h{
                        let idx=(2+(y*w+x)*3)as usize;
                        if idx+2<img.len(){img[idx]=Value::Number(r.clamp(0,255));img[idx+1]=Value::Number(g.clamp(0,255));img[idx+2]=Value::Number(b.clamp(0,255));}
                    }
                    Ok(Value::Array(img))
                }else{Err(type_error("gfx_set_pixel() first arg must be a gfx image"))}
            }
            // gfx_fill(img, r, g, b) → new img filled with color
            "gfx_fill"=>{
                if args.len()!=4{return Err(arity_error("gfx_fill",4,args.len()));}
                if let Value::Array(mut img)=args[0].clone(){
                    let w=num(&img[0]);let h=num(&img[1]);
                    let(r,g,b)=(num(&args[1]).clamp(0,255),num(&args[2]).clamp(0,255),num(&args[3]).clamp(0,255));
                    for i in 0..w*h{let idx=(2+i*3)as usize;if idx+2<img.len(){img[idx]=Value::Number(r);img[idx+1]=Value::Number(g);img[idx+2]=Value::Number(b);}}
                    Ok(Value::Array(img))
                }else{Err(type_error("gfx_fill() first arg must be a gfx image"))}
            }
            // gfx_draw_rect(img, x, y, w, h, r, g, b) → new img
            "gfx_draw_rect"=>{
                if args.len()!=8{return Err(arity_error("gfx_draw_rect",8,args.len()));}
                if let Value::Array(mut img)=args[0].clone(){
                    let iw=num(&img[0]);
                    let(rx,ry,rw,rh)=(num(&args[1]),num(&args[2]),num(&args[3]),num(&args[4]));
                    let(cr,cg,cb)=(num(&args[5]).clamp(0,255),num(&args[6]).clamp(0,255),num(&args[7]).clamp(0,255));
                    for dy in 0..rh{for dx in 0..rw{
                        let(px,py)=(rx+dx,ry+dy);
                        let idx=(2+(py*iw+px)*3)as usize;
                        if idx+2<img.len(){img[idx]=Value::Number(cr);img[idx+1]=Value::Number(cg);img[idx+2]=Value::Number(cb);}
                    }}
                    Ok(Value::Array(img))
                }else{Err(type_error("gfx_draw_rect() first arg must be a gfx image"))}
            }
            // gfx_draw_circle(img, cx, cy, r, cr, cg, cb) → new img
            "gfx_draw_circle"=>{
                if args.len()!=7{return Err(arity_error("gfx_draw_circle",7,args.len()));}
                if let Value::Array(mut img)=args[0].clone(){
                    let iw=num(&img[0]);let ih=num(&img[1]);
                    let(cx,cy,r)=(num(&args[1]),num(&args[2]),num(&args[3]));
                    let(cr,cg,cb)=(num(&args[4]).clamp(0,255),num(&args[5]).clamp(0,255),num(&args[6]).clamp(0,255));
                    for dy in -r..=r{for dx in -r..=r{
                        if dx*dx+dy*dy<=r*r{
                            let(px,py)=(cx+dx,cy+dy);
                            if px>=0&&py>=0&&px<iw&&py<ih{
                                let idx=(2+(py*iw+px)*3)as usize;
                                if idx+2<img.len(){img[idx]=Value::Number(cr);img[idx+1]=Value::Number(cg);img[idx+2]=Value::Number(cb);}
                            }
                        }
                    }}
                    Ok(Value::Array(img))
                }else{Err(type_error("gfx_draw_circle() first arg must be a gfx image"))}
            }
            // gfx_draw_line(img, x1, y1, x2, y2, r, g, b) → new img
            "gfx_draw_line"=>{
                if args.len()!=8{return Err(arity_error("gfx_draw_line",8,args.len()));}
                if let Value::Array(mut img)=args[0].clone(){
                    let iw=num(&img[0]);let ih=num(&img[1]);
                    let(mut x0,mut y0,x1,y1)=(num(&args[1]),num(&args[2]),num(&args[3]),num(&args[4]));
                    let(cr,cg,cb)=(num(&args[5]).clamp(0,255),num(&args[6]).clamp(0,255),num(&args[7]).clamp(0,255));
                    let(dx,dy)=((x1-x0).abs(),(y1-y0).abs());
                    let(sx,sy)=(if x0<x1{1}else{-1},if y0<y1{1}else{-1});
                    let mut err=dx-dy;
                    loop{
                        if x0>=0&&y0>=0&&x0<iw&&y0<ih{
                            let idx=(2+(y0*iw+x0)*3)as usize;
                            if idx+2<img.len(){img[idx]=Value::Number(cr);img[idx+1]=Value::Number(cg);img[idx+2]=Value::Number(cb);}
                        }
                        if x0==x1&&y0==y1{break;}
                        let e2=2*err;
                        if e2>-dy{err-=dy;x0+=sx;}
                        if e2<dx{err+=dx;y0+=sy;}
                    }
                    Ok(Value::Array(img))
                }else{Err(type_error("gfx_draw_line() first arg must be a gfx image"))}
            }
            // gfx_gradient(img, r1,g1,b1, r2,g2,b2, direction) → img with gradient
            // direction: 0=horizontal, 1=vertical
            "gfx_gradient"=>{
                if args.len()!=8{return Err(arity_error("gfx_gradient",8,args.len()));}
                if let Value::Array(mut img)=args[0].clone(){
                    let iw=num(&img[0]);let ih=num(&img[1]);
                    let(r1,g1,b1)=(num(&args[1]),num(&args[2]),num(&args[3]));
                    let(r2,g2,b2)=(num(&args[4]),num(&args[5]),num(&args[6]));
                    let dir=num(&args[7]);
                    for py in 0..ih{for px in 0..iw{
                        let t=if dir==0{if iw<=1{0}else{px*1000/(iw-1)}}else{if ih<=1{0}else{py*1000/(ih-1)}};
                        let(cr,cg,cb)=(r1+(r2-r1)*t/1000,g1+(g2-g1)*t/1000,b1+(b2-b1)*t/1000);
                        let idx=(2+(py*iw+px)*3)as usize;
                        if idx+2<img.len(){img[idx]=Value::Number(cr.clamp(0,255));img[idx+1]=Value::Number(cg.clamp(0,255));img[idx+2]=Value::Number(cb.clamp(0,255));}
                    }}
                    Ok(Value::Array(img))
                }else{Err(type_error("gfx_gradient() first arg must be a gfx image"))}
            }
            // gfx_save(img, filename_str) — save as PPM file
            "gfx_save"=>{
                if args.len()!=2{return Err(arity_error("gfx_save",2,args.len()));}
                if let Value::Array(ref img)=args[0]{
                    let w=match&img[0]{Value::Number(n)=>*n,_=>return Err(type_error("Invalid image"))};
                    let h=match&img[1]{Value::Number(n)=>*n,_=>return Err(type_error("Invalid image"))};
                    let fname=match&args[1]{Value::Str(s)=>s.clone(),_=>return Err(type_error("gfx_save() filename must be string"))};
                    let ppm=format!("P6\n{} {}\n255\n",w,h);
                    let mut bytes:Vec<u8>=ppm.into_bytes();
                    for i in 0..w*h{
                        let idx=(2+i*3)as usize;
                        let r=match img.get(idx){Some(Value::Number(n))=>(*n).clamp(0,255)as u8,_=>0};
                        let g=match img.get(idx+1){Some(Value::Number(n))=>(*n).clamp(0,255)as u8,_=>0};
                        let b=match img.get(idx+2){Some(Value::Number(n))=>(*n).clamp(0,255)as u8,_=>0};
                        bytes.push(r);bytes.push(g);bytes.push(b);
                    }
                    std::fs::write(&fname,&bytes)
                        .map_err(|e|runtime_error(&format!("gfx_save() failed to write '{}': {}",fname,e)))?;
                    println!("Image saved: {} ({}x{} PPM)",fname,w,h);
                    Ok(Value::Str(fname))
                }else{Err(type_error("gfx_save() first arg must be a gfx image"))}
            }
            _=>Err(runtime_error(&format!("Unknown gfx function '{}'",name))),
        }
    }

    // ── HELPERS ───────────────────────────────────────────────────────
        fn apply_op(&self, lhs: &Value, op: &str, rhs: &Value) -> Result<Value,FluxisError> {
        match op {
            "+"=>Ok(match(lhs,rhs){
                (Value::Number(a),Value::Number(b))=>Value::Number(a+b),
                (Value::Float(a),Value::Float(b))=>Value::Float(a+b),
                (Value::Float(a),Value::Number(b))=>Value::Float(a+(*b as f64)),
                (Value::Number(a),Value::Float(b))=>Value::Float((*a as f64)+b),
                _=>Value::Str(format!("{}{}",self.val_to_string(lhs),self.val_to_string(rhs))),
            }),
            "-"=>match(lhs,rhs){
                (Value::Number(a),Value::Number(b))=>Ok(Value::Number(a-b)),
                (Value::Float(a),Value::Float(b))=>Ok(Value::Float(a-b)),
                (Value::Float(a),Value::Number(b))=>Ok(Value::Float(a-(*b as f64))),
                (Value::Number(a),Value::Float(b))=>Ok(Value::Float((*a as f64)-b)),
                (l,r)=>Err(type_error(&format!("Cannot subtract {} from {}",self.type_name(r),self.type_name(l)))),
            },
            "*"=>match(lhs,rhs){
                (Value::Number(a),Value::Number(b))=>Ok(Value::Number(a*b)),
                (Value::Float(a),Value::Float(b))=>Ok(Value::Float(a*b)),
                (Value::Float(a),Value::Number(b))=>Ok(Value::Float(a*(*b as f64))),
                (Value::Number(a),Value::Float(b))=>Ok(Value::Float((*a as f64)*b)),
                (l,r)=>Err(type_error(&format!("Cannot multiply {} and {}",self.type_name(l),self.type_name(r)))),
            },
            "/"=>match(lhs,rhs){
                (Value::Number(a),Value::Number(b))=>{if *b==0{Err(runtime_error("Division by zero"))}else{Ok(Value::Number(a/b))}},
                (Value::Float(a),Value::Float(b))=>{if *b==0.0{Err(runtime_error("Division by zero"))}else{Ok(Value::Float(a/b))}},
                (Value::Float(a),Value::Number(b))=>{if *b==0{Err(runtime_error("Division by zero"))}else{Ok(Value::Float(a/(*b as f64)))}},
                (Value::Number(a),Value::Float(b))=>{if *b==0.0{Err(runtime_error("Division by zero"))}else{Ok(Value::Float((*a as f64)/b))}},
                (l,r)=>Err(type_error(&format!("Cannot divide {} and {}",self.type_name(l),self.type_name(r)))),
            },
            "%"=>match(lhs,rhs){
                (Value::Number(a),Value::Number(b))=>{if *b==0{Err(runtime_error("Modulo by zero"))}else{Ok(Value::Number(a%b))}},
                (Value::Float(a),Value::Float(b))=>{if *b==0.0{Err(runtime_error("Modulo by zero"))}else{Ok(Value::Float(a%b))}},
                (Value::Float(a),Value::Number(b))=>{if *b==0{Err(runtime_error("Modulo by zero"))}else{Ok(Value::Float(a%(*b as f64)))}},
                (Value::Number(a),Value::Float(b))=>{if *b==0.0{Err(runtime_error("Modulo by zero"))}else{Ok(Value::Float((*a as f64)%b))}},
                (l,r)=>Err(type_error(&format!("Cannot apply % to {} and {}",self.type_name(l),self.type_name(r)))),
            },
            _=>Err(runtime_error(&format!("Unknown operator '{}'",op))),
        }
    }
    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a,b){
            (Value::Number(x),Value::Number(y))=>x==y,
            (Value::Float(x),Value::Float(y))=>(x-y).abs()<1e-12,
            (Value::Number(x),Value::Float(y))=>(*x as f64-y).abs()<1e-12,
            (Value::Float(x),Value::Number(y))=>(x-*y as f64).abs()<1e-12,
            (Value::Str(x),Value::Str(y))=>x==y,
            (Value::Bool(x),Value::Bool(y))=>x==y,
            (Value::Nil,Value::Nil)=>true,
            (Value::EnumVariant{enum_name:e1,variant:v1},Value::EnumVariant{enum_name:e2,variant:v2})=>e1==e2&&v1==v2,
            _=>self.val_to_string(a)==self.val_to_string(b),
        }
    }
    fn check_type(&self,v:&Value,ann:&TypeAnnotation,label:&str)->Result<(),FluxisError>{
        if matches!(ann,TypeAnnotation::Any){return Ok(());}
        // num accepts float and vice-versa for flexibility
        if matches!(ann,TypeAnnotation::Num)&&matches!(v,Value::Float(_)){return Ok(());}
        if matches!(ann,TypeAnnotation::Float)&&matches!(v,Value::Number(_)){return Ok(());}
        let e=ann.name();let a=self.type_name(v);
        if e!=a{return Err(type_error(&format!("'{}' declared as '{}' but got '{}'",label,e,a)).with_hint(&format!("Change the value to match '{}'",e)));}
        Ok(())
    }
    fn is_truthy(&self,v:&Value)->bool{match v{Value::Bool(b)=>*b,Value::Number(n)=>*n!=0,Value::Float(f)=>*f!=0.0,Value::Str(s)=>!s.is_empty(),Value::Nil=>false,Value::Array(a)=>!a.is_empty(),Value::Map(m)=>!m.is_empty(),_=>true}}
    pub fn type_name(&self,v:&Value)->&'static str{match v{Value::Number(_)=>"num",Value::Float(_)=>"float",Value::Str(_)=>"str",Value::Bool(_)=>"bool",Value::Nil=>"nil",Value::Array(_)=>"array",Value::Map(_)=>"map",Value::Struct{..}=>"struct",Value::EnumVariant{..}=>"enum",Value::Dotion{..}=>"dotion"}}
    pub fn val_to_string(&self,v:&Value)->String{match v{
        Value::Number(n)=>n.to_string(),Value::Float(f)=>{let s=format!("{:.6}",f);s.trim_end_matches('0').trim_end_matches('.').to_string()},Value::Str(s)=>s.clone(),Value::Bool(b)=>b.to_string(),Value::Nil=>"nil".to_string(),
        Value::Array(a)=>format!("[{}]",a.iter().map(|v|self.val_to_string(v)).collect::<Vec<_>>().join(", ")),
        Value::Map(m)=>format!("{{{}}}",m.iter().map(|(k,v)|format!("{}: {}",k,self.val_to_string(v))).collect::<Vec<_>>().join(", ")),
        Value::Struct{name,fields}=>format!("{} {{{}}}",name,fields.iter().map(|(k,v)|format!("{}: {}",k,self.val_to_string(v))).collect::<Vec<_>>().join(", ")),
        Value::EnumVariant{enum_name,variant}=>format!("{}::{}",enum_name,variant),
        Value::Dotion{name,fields,..}=>format!("dotion({}) {{{}}}",name,fields.iter().map(|(k,v)|format!("{}: {}",k,self.val_to_string(v))).collect::<Vec<_>>().join(", ")),
    }}
}

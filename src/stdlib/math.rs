// FLUXIS — stdlib/math.rs
// Math standard library. Called by VM runtime — no VM state needed.

use crate::vm::value::Value;
use crate::error::{FluxisError, runtime_error, type_error, arity_error};

pub fn call(name: &str, args: &[Value]) -> Result<Value, FluxisError> {
    let fv = |v: &Value| -> f64 { match v { Value::Float(f) => *f, Value::Number(n) => *n as f64, _ => 0.0 } };

    match name {
        "abs"       => Ok(match &args[0] { Value::Number(n)=>Value::Number(n.abs()), Value::Float(f)=>Value::Float(f.abs()), _=>return Err(type_error("abs() expects num or float")) }),
        "sign"      => { if args.len()!=1{return Err(arity_error("sign",1,args.len()));} let n=match &args[0]{Value::Number(n)=>*n,_=>return Err(type_error("sign() expects num"))}; Ok(Value::Number(if n>0{1}else if n<0{-1}else{0})) }
        "sqrt"      => { let f=fv(&args[0]); if f<0.0{return Err(runtime_error("sqrt() of negative number").with_hint("Use abs() first if needed"));} Ok(Value::Float(f.sqrt())) }
        "floor"     => Ok(match &args[0] { Value::Float(f)=>Value::Number(f.floor()as i64), Value::Number(n)=>Value::Number(*n), _=>return Err(type_error("floor() expects num or float")) }),
        "ceil"      => Ok(match &args[0] { Value::Float(f)=>Value::Number(f.ceil()as i64),  Value::Number(n)=>Value::Number(*n), _=>return Err(type_error("ceil() expects num or float")) }),
        "max"       => { if args.len()!=2{return Err(arity_error("max",2,args.len()));} match(&args[0],&args[1]){ (Value::Number(a),Value::Number(b))=>Ok(Value::Number(*a.max(b))), (Value::Float(a),Value::Float(b))=>Ok(Value::Float(a.max(*b))), (Value::Float(a),Value::Number(b))=>Ok(Value::Float(a.max(*b as f64))), (Value::Number(a),Value::Float(b))=>Ok(Value::Float((*a as f64).max(*b))), _=>Err(type_error("max() requires numbers")) } }
        "min"       => { if args.len()!=2{return Err(arity_error("min",2,args.len()));} match(&args[0],&args[1]){ (Value::Number(a),Value::Number(b))=>Ok(Value::Number(*a.min(b))), (Value::Float(a),Value::Float(b))=>Ok(Value::Float(a.min(*b))), (Value::Float(a),Value::Number(b))=>Ok(Value::Float(a.min(*b as f64))), (Value::Number(a),Value::Float(b))=>Ok(Value::Float((*a as f64).min(*b))), _=>Err(type_error("min() requires numbers")) } }
        "pow"       => { if args.len()!=2{return Err(arity_error("pow",2,args.len()));} match(&args[0],&args[1]){ (Value::Number(b),Value::Number(e))=>{if *e<0{return Err(runtime_error("pow() negative exponent"));}Ok(Value::Number(b.pow(*e as u32)))},(Value::Float(b),Value::Number(e))=>Ok(Value::Float(b.powi(*e as i32))),(Value::Float(b),Value::Float(e))=>Ok(Value::Float(b.powf(*e))),(Value::Number(b),Value::Float(e))=>Ok(Value::Float((*b as f64).powf(*e))),_=>Err(type_error("pow() requires numbers")) } }
        "clamp"     => { if args.len()!=3{return Err(arity_error("clamp",3,args.len()));} match(&args[0],&args[1],&args[2]){ (Value::Number(v),Value::Number(lo),Value::Number(hi))=>Ok(Value::Number((*v).clamp(*lo,*hi))), _=>Err(type_error("clamp() requires three numbers")) } }
        "rand"      => {
            if args.len()!=2{return Err(arity_error("rand",2,args.len()));}
            match (&args[0],&args[1]) {
                (Value::Number(lo),Value::Number(hi)) => {
                    if lo>hi{return Err(runtime_error("rand() min must be <= max"));}
                    use std::time::{SystemTime,UNIX_EPOCH};
                    let s=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
                    let r=(s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407))>>17;
                    Ok(Value::Number(lo+(r%((hi-lo+1)as u64))as i64))
                }
                _ => Err(type_error("rand() requires two numbers"))
            }
        }
        "rand_float"=> {
            use std::time::{SystemTime,UNIX_EPOCH};
            let s=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
            let r=(s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407))>>33;
            Ok(Value::Float((r%1000000) as f64/1000000.0))
        }
        "log"       => { let f=fv(&args[0]); if f<=0.0{return Err(runtime_error("log() requires positive number").with_hint("log(x) is undefined for x <= 0"));} Ok(Value::Float(f.ln())) }
        "log2"      => { let f=fv(&args[0]); if f<=0.0{return Err(runtime_error("log2() requires positive number"));} Ok(Value::Float(f.log2())) }
        "log10"     => { let f=fv(&args[0]); if f<=0.0{return Err(runtime_error("log10() requires positive number"));} Ok(Value::Float(f.log10())) }
        "sin"       => Ok(Value::Float(fv(&args[0]).sin())),
        "cos"       => Ok(Value::Float(fv(&args[0]).cos())),
        "tan"       => Ok(Value::Float(fv(&args[0]).tan())),
        "pi"        => Ok(Value::Float(std::f64::consts::PI)),
        _           => Err(runtime_error(&format!("Unknown math function '{}'", name))),
    }
}

pub fn is_math_fn(name: &str) -> bool {
    matches!(name, "abs"|"sign"|"sqrt"|"floor"|"ceil"|"max"|"min"|"pow"|"clamp"
                 |"rand"|"rand_float"|"log"|"log2"|"log10"|"sin"|"cos"|"tan"|"pi")
}


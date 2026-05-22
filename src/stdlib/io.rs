// FLUXIS — stdlib/io.rs
// IO standard library. Called by VM runtime — no VM state needed.

use crate::vm::value::Value;
use crate::error::{FluxisError, runtime_error, type_error, arity_error};

pub fn call(name: &str, args: &[Value]) -> Result<Value, FluxisError> {
    match name {
        "read_line"   => {
            use std::io::{self, Write};
            let mut s = String::new();
            io::stdout().flush().ok();
            io::stdin().read_line(&mut s).ok();
            Ok(Value::Str(s.trim_end_matches('\n').trim_end_matches('\r').to_string()))
        }
        "print_err"   => {
            if args.len()!=1{return Err(arity_error("print_err",1,args.len()));}
            eprintln!("{}", args[0].display());
            Ok(Value::Nil)
        }
        "read_file"   => {
            if args.len()!=1{return Err(arity_error("read_file",1,args.len()));}
            let path = match &args[0]{Value::Str(s)=>s.clone(),_=>return Err(type_error("read_file() expects string path"))};
            match std::fs::read_to_string(&path) {
                Ok(c) => Ok(Value::Str(c)),
                Err(e) => Err(runtime_error(&format!("read_file(\"{}\") failed: {}", path, e))
                    .with_hint("Check the file exists and you have read permission")),
            }
        }
        "write_file"  => {
            if args.len()!=2{return Err(arity_error("write_file",2,args.len()));}
            let path = match &args[0]{Value::Str(s)=>s.clone(),_=>return Err(type_error("write_file() first arg must be string path"))};
            match std::fs::write(&path, args[1].display()) {
                Ok(_)  => Ok(Value::Nil),
                Err(e) => Err(runtime_error(&format!("write_file(\"{}\") failed: {}", path, e))),
            }
        }
        "append_file" => {
            if args.len()!=2{return Err(arity_error("append_file",2,args.len()));}
            let path = match &args[0]{Value::Str(s)=>s.clone(),_=>return Err(type_error("append_file() first arg must be string path"))};
            use std::io::Write;
            match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                Ok(mut f) => { f.write_all(args[1].display().as_bytes()).ok(); Ok(Value::Nil) }
                Err(e)    => Err(runtime_error(&format!("append_file(\"{}\") failed: {}", path, e))),
            }
        }
        "file_exists" => {
            if args.len()!=1{return Err(arity_error("file_exists",1,args.len()));}
            let path = match &args[0]{Value::Str(s)=>s.clone(),_=>return Err(type_error("file_exists() expects string path"))};
            Ok(Value::Bool(std::path::Path::new(&path).exists()))
        }
        "exit"        => {
            let code = match args.first(){Some(Value::Number(n))=>*n as i32, None=>0, _=>0};
            std::process::exit(code);
        }
        "sleep"       => {
            if args.len()!=1{return Err(arity_error("sleep",1,args.len()));}
            let ms = match &args[0]{Value::Number(n)=>*n as u64, Value::Float(f)=>*f as u64, _=>return Err(type_error("sleep() expects num (milliseconds)"))};
            std::thread::sleep(std::time::Duration::from_millis(ms));
            Ok(Value::Nil)
        }
        "time_now"    => {
            use std::time::{SystemTime, UNIX_EPOCH};
            Ok(Value::Number(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64))
        }
        _ => Err(runtime_error(&format!("Unknown io function '{}'", name))),
    }
}

pub fn is_io_fn(name: &str) -> bool {
    matches!(name, "read_line"|"print_err"|"read_file"|"write_file"|"append_file"
                 |"file_exists"|"exit"|"sleep"|"time_now")
}


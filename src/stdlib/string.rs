// FLUXIS — stdlib/string.rs
// String standard library. Called by VM runtime — no VM state needed.

use crate::vm::value::Value;
use crate::error::{FluxisError, runtime_error, type_error, arity_error};

pub fn call(name: &str, args: &[Value]) -> Result<Value, FluxisError> {
    let rs = |f: &str, args: &[Value], i: usize| -> Result<String, FluxisError> {
        if args.len() <= i { return Err(arity_error(f, i+1, args.len())); }
        match &args[i] { Value::Str(s) => Ok(s.clone()), _ => Err(type_error(&format!("{} arg {} must be string", f, i+1))) }
    };
    match name {
        "upper"       => Ok(Value::Str(rs("upper",args,0)?.to_uppercase())),
        "lower"       => Ok(Value::Str(rs("lower",args,0)?.to_lowercase())),
        "trim"        => Ok(Value::Str(rs("trim",args,0)?.trim().to_string())),
        "str_len"     => Ok(Value::Number(rs("str_len",args,0)?.len() as i64)),
        "contains"    => { let s=rs("contains",args,0)?; let p=rs("contains",args,1)?; Ok(Value::Bool(s.contains(&*p))) }
        "starts_with" => { let s=rs("starts_with",args,0)?; let p=rs("starts_with",args,1)?; Ok(Value::Bool(s.starts_with(&*p))) }
        "ends_with"   => { let s=rs("ends_with",args,0)?; let p=rs("ends_with",args,1)?; Ok(Value::Bool(s.ends_with(&*p))) }
        "replace"     => { let s=rs("replace",args,0)?; let f=rs("replace",args,1)?; let t=rs("replace",args,2)?; Ok(Value::Str(s.replace(&*f,&t))) }
        "split"       => { let s=rs("split",args,0)?; let sep=rs("split",args,1)?; Ok(Value::Array(s.split(&*sep).map(|p|Value::Str(p.to_string())).collect())) }
        "join"        => {
            if args.len()!=2{return Err(arity_error("join",2,args.len()));}
            let sep=rs("join",args,1)?;
            match &args[0] { Value::Array(a)=>Ok(Value::Str(a.iter().map(|v|v.display()).collect::<Vec<_>>().join(&sep))), _=>Err(type_error("join() first arg must be array")) }
        }
        "repeat"      => { let s=rs("repeat",args,0)?; match args.get(1){Some(Value::Number(n))=>Ok(Value::Str(s.repeat(*n as usize))),_=>Err(type_error("repeat() second arg must be num"))} }
        "char_at"     => { let s=rs("char_at",args,0)?; match args.get(1){Some(Value::Number(i))=>Ok(s.chars().nth(*i as usize).map(|c|Value::Str(c.to_string())).unwrap_or(Value::Nil)),_=>Err(type_error("char_at() second arg must be num"))} }
        "pad_left"    => {
            let s=rs("pad_left",args,0)?;
            let w=match args.get(1){Some(Value::Number(n))=>*n as usize,_=>return Err(type_error("pad_left() second arg must be num"))};
            let pc=match args.get(2){Some(Value::Str(c))=>c.chars().next().unwrap_or(' '),_=>' '};
            if s.len()>=w{return Ok(Value::Str(s));}
            let pad:String=std::iter::repeat(pc).take(w-s.len()).collect();
            Ok(Value::Str(format!("{}{}",pad,s)))
        }
        "pad_right"   => {
            let s=rs("pad_right",args,0)?;
            let w=match args.get(1){Some(Value::Number(n))=>*n as usize,_=>return Err(type_error("pad_right() second arg must be num"))};
            let pc=match args.get(2){Some(Value::Str(c))=>c.chars().next().unwrap_or(' '),_=>' '};
            if s.len()>=w{return Ok(Value::Str(s));}
            let pad:String=std::iter::repeat(pc).take(w-s.len()).collect();
            Ok(Value::Str(format!("{}{}",s,pad)))
        }
        "parse_int"   => { let s=rs("parse_int",args,0)?; Ok(s.trim().parse::<i64>().map(Value::Number).unwrap_or(Value::Nil)) }
        "parse_float" => { let s=rs("parse_float",args,0)?; Ok(s.trim().parse::<f64>().map(Value::Float).unwrap_or(Value::Nil)) }
        "format"      => {
            if args.is_empty(){return Err(arity_error("format",1,0));}
            let template=match &args[0]{Value::Str(s)=>s.clone(),other=>return Err(type_error(&format!("format() first arg must be string, got {}",other.type_name())))};
            let extra=&args[1..];
            let mut result=String::new();
            let chars:Vec<char>=template.chars().collect();
            let mut i=0usize; let mut arg_idx=0usize;
            while i<chars.len(){
                if chars[i]=='{'&&i+1<chars.len(){
                    if chars[i+1]=='}'{ if arg_idx<extra.len(){result.push_str(&extra[arg_idx].display());arg_idx+=1;}i+=2; }
                    else {
                        let mut nm=String::new(); let mut j=i+1;
                        while j<chars.len()&&chars[j]!='}'&&chars[j]!='{'{nm.push(chars[j]);j+=1;}
                        if j<chars.len()&&chars[j]=='}'&&!nm.is_empty(){
                            if let Ok(idx)=nm.parse::<usize>(){if idx<extra.len(){result.push_str(&extra[idx].display());}}
                            else{result.push('{');result.push_str(&nm);result.push('}');}
                            i=j+1;
                        }else{result.push(chars[i]);i+=1;}
                    }
                }else{result.push(chars[i]);i+=1;}
            }
            Ok(Value::Str(result))
        }
        _             => Err(runtime_error(&format!("Unknown string function '{}'", name))),
    }
}

pub fn is_string_fn(name: &str) -> bool {
    matches!(name, "upper"|"lower"|"trim"|"str_len"|"contains"|"starts_with"|"ends_with"
                 |"replace"|"split"|"join"|"repeat"|"char_at"|"pad_left"|"pad_right"
                 |"parse_int"|"parse_float"|"format")
}


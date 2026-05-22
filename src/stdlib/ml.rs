// FLUXIS — stdlib/ml.rs
// Machine learning standard library. Called by VM runtime — no VM state needed.

use crate::vm::value::Value;
use crate::error::{FluxisError, runtime_error, arity_error};

fn fv(v: &Value) -> f64 { match v { Value::Float(f)=>*f, Value::Number(n)=>*n as f64, _=>0.0 } }
fn flat(v: &Value) -> Vec<f64> { match v { Value::Array(a)=>a.iter().map(fv).collect(), Value::Float(f)=>vec![*f], Value::Number(n)=>vec![*n as f64], _=>vec![] } }
fn mat(v: &Value) -> Vec<Vec<f64>> { match v { Value::Array(rows)=>rows.iter().map(|r| match r { Value::Array(cells)=>cells.iter().map(fv).collect(), _=>vec![] }).collect(), _=>vec![] } }
fn arr(v: Vec<f64>) -> Value { Value::Array(v.into_iter().map(Value::Float).collect()) }
fn mat2d(m: Vec<Vec<f64>>) -> Value { Value::Array(m.into_iter().map(|r| Value::Array(r.into_iter().map(Value::Float).collect())).collect()) }
fn sig(x: f64) -> f64 { 1.0 / (1.0 + (-x).exp()) }

pub fn call(name: &str, args: &[Value]) -> Result<Value, FluxisError> {
    match name {
        "ml_zeros"      => { if args.len()!=2{return Err(arity_error("ml_zeros",2,args.len()));} let(r,c)=(fv(&args[0])as usize,fv(&args[1])as usize); Ok(mat2d(vec![vec![0.0;c];r])) }
        "ml_ones"       => { if args.len()!=2{return Err(arity_error("ml_ones",2,args.len()));} let(r,c)=(fv(&args[0])as usize,fv(&args[1])as usize); Ok(mat2d(vec![vec![1.0;c];r])) }
        "ml_identity"   => { if args.len()!=1{return Err(arity_error("ml_identity",1,args.len()));} let n=fv(&args[0])as usize; let mut m=vec![vec![0.0;n];n]; for i in 0..n{m[i][i]=1.0;} Ok(mat2d(m)) }
        "ml_new"        => { if args.len()!=3{return Err(arity_error("ml_new",3,args.len()));} let(r,c,v)=(fv(&args[0])as usize,fv(&args[1])as usize,fv(&args[2])); Ok(mat2d(vec![vec![v;c];r])) }
        "ml_get"        => { if args.len()!=3{return Err(arity_error("ml_get",3,args.len()));} let m=mat(&args[0]);let(r,c)=(fv(&args[1])as usize,fv(&args[2])as usize); Ok(Value::Float(m.get(r).and_then(|row|row.get(c)).copied().unwrap_or(0.0))) }
        "ml_set"        => { if args.len()!=4{return Err(arity_error("ml_set",4,args.len()));} let mut m=mat(&args[0]);let(r,c,v)=(fv(&args[1])as usize,fv(&args[2])as usize,fv(&args[3])); if r<m.len()&&c<m[r].len(){m[r][c]=v;} Ok(mat2d(m)) }
        "ml_shape"      => { if args.len()!=1{return Err(arity_error("ml_shape",1,args.len()));} let m=mat(&args[0]);let r=m.len()as f64;let c=m.first().map(|r|r.len()).unwrap_or(0)as f64; Ok(Value::Array(vec![Value::Float(r),Value::Float(c)])) }
        "ml_add"        => { if args.len()!=2{return Err(arity_error("ml_add",2,args.len()));} let(a,b)=(mat(&args[0]),mat(&args[1])); Ok(mat2d(a.iter().zip(b.iter()).map(|(ar,br)|ar.iter().zip(br.iter()).map(|(x,y)|x+y).collect()).collect())) }
        "ml_sub"        => { if args.len()!=2{return Err(arity_error("ml_sub",2,args.len()));} let(a,b)=(mat(&args[0]),mat(&args[1])); Ok(mat2d(a.iter().zip(b.iter()).map(|(ar,br)|ar.iter().zip(br.iter()).map(|(x,y)|x-y).collect()).collect())) }
        "ml_mul"        => { if args.len()!=2{return Err(arity_error("ml_mul",2,args.len()));} let(a,b)=(mat(&args[0]),mat(&args[1])); Ok(mat2d(a.iter().zip(b.iter()).map(|(ar,br)|ar.iter().zip(br.iter()).map(|(x,y)|x*y).collect()).collect())) }
        "ml_scale"      => { if args.len()!=2{return Err(arity_error("ml_scale",2,args.len()));} let(m,s)=(mat(&args[0]),fv(&args[1])); Ok(mat2d(m.iter().map(|r|r.iter().map(|x|x*s).collect()).collect())) }
        "ml_matmul"     => {
            if args.len()!=2{return Err(arity_error("ml_matmul",2,args.len()));}
            let(a,b)=(mat(&args[0]),mat(&args[1]));
            let(ra,ca)=(a.len(),a.first().map(|r|r.len()).unwrap_or(0));
            let cb=b.first().map(|r|r.len()).unwrap_or(0);
            if ca!=b.len(){return Err(runtime_error(&format!("ml_matmul: shape mismatch {}x{} vs {}x{}",ra,ca,b.len(),cb)).with_hint("cols of A must equal rows of B"));}
            let mut res=vec![vec![0.0f64;cb];ra];
            for i in 0..ra{for j in 0..cb{for k in 0..ca{res[i][j]+=a[i][k]*b[k][j];}}}
            Ok(mat2d(res))
        }
        "ml_transpose"  => { if args.len()!=1{return Err(arity_error("ml_transpose",1,args.len()));} let m=mat(&args[0]);let(rows,cols)=(m.len(),m.first().map(|r|r.len()).unwrap_or(0));let mut t=vec![vec![0.0f64;rows];cols];for i in 0..rows{for j in 0..cols{t[j][i]=m[i][j];}}Ok(mat2d(t)) }
        "ml_dot"        => { if args.len()!=2{return Err(arity_error("ml_dot",2,args.len()));} Ok(Value::Float(flat(&args[0]).iter().zip(flat(&args[1]).iter()).map(|(x,y)|x*y).sum())) }
        "sigmoid"       => { if args.len()!=1{return Err(arity_error("sigmoid",1,args.len()));} Ok(Value::Float(sig(fv(&args[0])))) }
        "sigmoid_arr"   => { if args.len()!=1{return Err(arity_error("sigmoid_arr",1,args.len()));} Ok(arr(flat(&args[0]).iter().map(|&x|sig(x)).collect())) }
        "sigmoid_deriv" => { if args.len()!=1{return Err(arity_error("sigmoid_deriv",1,args.len()));} let s=sig(fv(&args[0])); Ok(Value::Float(s*(1.0-s))) }
        "sigmoid_deriv_from_output" => { if args.len()!=1{return Err(arity_error("sigmoid_deriv_from_output",1,args.len()));} let s=fv(&args[0]); Ok(Value::Float(s*(1.0-s))) }
        "sigmoid_deriv_arr" => { if args.len()!=1{return Err(arity_error("sigmoid_deriv_arr",1,args.len()));} Ok(arr(flat(&args[0]).iter().map(|&s|s*(1.0-s)).collect())) }
        "relu"          => { if args.len()!=1{return Err(arity_error("relu",1,args.len()));} Ok(Value::Float(fv(&args[0]).max(0.0))) }
        "relu_arr"      => { if args.len()!=1{return Err(arity_error("relu_arr",1,args.len()));} Ok(arr(flat(&args[0]).iter().map(|&x|x.max(0.0)).collect())) }
        "leaky_relu"    => { if args.len()!=2{return Err(arity_error("leaky_relu",2,args.len()));} let(x,a)=(fv(&args[0]),fv(&args[1])); Ok(Value::Float(if x>=0.0{x}else{a*x})) }
        "tanh_f"        => { if args.len()!=1{return Err(arity_error("tanh_f",1,args.len()));} Ok(Value::Float(fv(&args[0]).tanh())) }
        "softmax"       => {
            if args.len()!=1{return Err(arity_error("softmax",1,args.len()));}
            let a=flat(&args[0]);
            let max_v=a.iter().cloned().fold(f64::NEG_INFINITY,f64::max);
            let exps:Vec<f64>=a.iter().map(|x|(x-max_v).exp()).collect();
            let sum:f64=exps.iter().sum();
            Ok(arr(exps.iter().map(|&e|e/sum).collect()))
        }
        "linear"        => { if args.len()!=1{return Err(arity_error("linear",1,args.len()));} Ok(args[0].clone()) }
        "ml_mse"        => { if args.len()!=2{return Err(arity_error("ml_mse",2,args.len()));} let(p,a)=(flat(&args[0]),flat(&args[1]));let n=p.len()as f64;if n==0.0{return Ok(Value::Float(0.0));} Ok(Value::Float(p.iter().zip(a.iter()).map(|(pi,ai)|(pi-ai).powi(2)).sum::<f64>()/n)) }
        "ml_mae"        => { if args.len()!=2{return Err(arity_error("ml_mae",2,args.len()));} let(p,a)=(flat(&args[0]),flat(&args[1]));let n=p.len()as f64;if n==0.0{return Ok(Value::Float(0.0));} Ok(Value::Float(p.iter().zip(a.iter()).map(|(pi,ai)|(pi-ai).abs()).sum::<f64>()/n)) }
        "ml_cross_entropy" => { if args.len()!=2{return Err(arity_error("ml_cross_entropy",2,args.len()));} let(p,a)=(flat(&args[0]),flat(&args[1]));let n=p.len()as f64;if n==0.0{return Ok(Value::Float(0.0));}let loss:f64=p.iter().zip(a.iter()).map(|(&pi,&ai)|{let pc=pi.max(1e-9).min(1.0-1e-9);-ai*pc.ln()-(1.0-ai)*(1.0-pc).ln()}).sum(); Ok(Value::Float(loss/n)) }
        "ml_mean"       => { if args.len()!=1{return Err(arity_error("ml_mean",1,args.len()));} let a=flat(&args[0]);let n=a.len()as f64; Ok(Value::Float(if n==0.0{0.0}else{a.iter().sum::<f64>()/n})) }
        "ml_sum"        => { if args.len()!=1{return Err(arity_error("ml_sum",1,args.len()));} Ok(Value::Float(flat(&args[0]).iter().sum())) }
        "ml_max_val"    => { if args.len()!=1{return Err(arity_error("ml_max_val",1,args.len()));} Ok(Value::Float(flat(&args[0]).iter().cloned().fold(f64::NEG_INFINITY,f64::max))) }
        "ml_min"        => { if args.len()!=1{return Err(arity_error("ml_min",1,args.len()));} Ok(Value::Float(flat(&args[0]).iter().cloned().fold(f64::INFINITY,f64::min))) }
        "ml_std"        => { if args.len()!=1{return Err(arity_error("ml_std",1,args.len()));} let a=flat(&args[0]);let n=a.len()as f64;if n==0.0{return Ok(Value::Float(0.0));}let mean=a.iter().sum::<f64>()/n;let var=a.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/n; Ok(Value::Float(var.sqrt())) }
        "ml_normalize"  => { if args.len()!=1{return Err(arity_error("ml_normalize",1,args.len()));} let a=flat(&args[0]);let mn=a.iter().cloned().fold(f64::INFINITY,f64::min);let mx=a.iter().cloned().fold(f64::NEG_INFINITY,f64::max);let range=mx-mn;if range==0.0{return Ok(arr(vec![0.0;a.len()]));} Ok(arr(a.iter().map(|x|(x-mn)/range).collect())) }
        "ml_flatten"    => { if args.len()!=1{return Err(arity_error("ml_flatten",1,args.len()));} Ok(arr(mat(&args[0]).into_iter().flatten().collect())) }
        "ml_layer_forward" => { if args.len()!=3{return Err(arity_error("ml_layer_forward",3,args.len()));} let inputs=flat(&args[0]);let weights=mat(&args[1]);let biases=flat(&args[2]);let outputs:Vec<f64>=(0..weights.len()).map(|i|weights[i].iter().zip(inputs.iter()).map(|(wi,xi)|wi*xi).sum::<f64>()+biases.get(i).copied().unwrap_or(0.0)).collect(); Ok(arr(outputs)) }
        "ml_grad_desc_step" => { if args.len()!=3{return Err(arity_error("ml_grad_desc_step",3,args.len()));} let(w,g,lr)=(mat(&args[0]),mat(&args[1]),fv(&args[2])); Ok(mat2d(w.iter().zip(g.iter()).map(|(wr,gr)|wr.iter().zip(gr.iter()).map(|(wi,gi)|wi-lr*gi).collect()).collect())) }
        "ml_random_weights" => {
            if args.len()!=2{return Err(arity_error("ml_random_weights",2,args.len()));}
            let(r,c)=(fv(&args[0])as usize,fv(&args[1])as usize);
            use std::time::{SystemTime,UNIX_EPOCH};
            let mut seed=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
            let limit=(6.0f64/(r+c)as f64).sqrt();
            let mut rng=||{seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);let u=(seed>>11)as f64/(1u64<<53)as f64;u*2.0*limit-limit};
            Ok(mat2d((0..r).map(|_|(0..c).map(|_|rng()).collect()).collect()))
        }
        "ml_outer"      => { if args.len()!=2{return Err(arity_error("ml_outer",2,args.len()));} let(a,b)=(flat(&args[0]),flat(&args[1])); Ok(mat2d(a.iter().map(|&ai|b.iter().map(|&bi|ai*bi).collect()).collect())) }
        "ml_vec_add"    => { if args.len()!=2{return Err(arity_error("ml_vec_add",2,args.len()));} Ok(arr(flat(&args[0]).iter().zip(flat(&args[1]).iter()).map(|(x,y)|x+y).collect())) }
        "ml_vec_sub"    => { if args.len()!=2{return Err(arity_error("ml_vec_sub",2,args.len()));} Ok(arr(flat(&args[0]).iter().zip(flat(&args[1]).iter()).map(|(x,y)|x-y).collect())) }
        "ml_vec_mul"    => { if args.len()!=2{return Err(arity_error("ml_vec_mul",2,args.len()));} Ok(arr(flat(&args[0]).iter().zip(flat(&args[1]).iter()).map(|(x,y)|x*y).collect())) }
        "ml_vec_scale"  => { if args.len()!=2{return Err(arity_error("ml_vec_scale",2,args.len()));} let(a,s)=(flat(&args[0]),fv(&args[1])); Ok(arr(a.iter().map(|x|x*s).collect())) }
        "ml_mat_T_vec"  => { if args.len()!=2{return Err(arity_error("ml_mat_T_vec",2,args.len()));} let(m,v)=(mat(&args[0]),flat(&args[1]));let(rows,cols)=(m.len(),m.first().map(|r|r.len()).unwrap_or(0));let mut result=vec![0.0f64;cols];for i in 0..rows.min(v.len()){for j in 0..cols{result[j]+=m[i][j]*v[i];}} Ok(arr(result)) }
        "ml_bias_update"=> { if args.len()!=3{return Err(arity_error("ml_bias_update",3,args.len()));} let(b,g,lr)=(flat(&args[0]),flat(&args[1]),fv(&args[2])); Ok(arr(b.iter().zip(g.iter()).map(|(bi,gi)|bi-lr*gi).collect())) }
        "ml_vec_to_mat" => { if args.len()!=1{return Err(arity_error("ml_vec_to_mat",1,args.len()));} Ok(mat2d(vec![flat(&args[0])])) }
        "ml_mat_to_vec" => { if args.len()!=1{return Err(arity_error("ml_mat_to_vec",1,args.len()));} Ok(arr(mat(&args[0]).into_iter().next().unwrap_or_default())) }
        "ml_print_mat"  => {
            if args.len()!=1{return Err(arity_error("ml_print_mat",1,args.len()));}
            let m=mat(&args[0]);
            println!("Matrix {}×{}:", m.len(), m.first().map(|r|r.len()).unwrap_or(0));
            for row in &m { let s:Vec<String>=row.iter().map(|x|format!("{:8.4}",x)).collect(); println!("  [{}]",s.join("  ")); }
            Ok(Value::Nil)
        }
        _ => Err(runtime_error(&format!("Unknown ml function '{}'", name))),
    }
}

pub fn is_ml_fn(name: &str) -> bool {
    name.starts_with("ml_") || matches!(name,
        "sigmoid"|"sigmoid_arr"|"sigmoid_deriv"|"sigmoid_deriv_from_output"|"sigmoid_deriv_arr"
        |"relu"|"relu_arr"|"leaky_relu"|"tanh_f"|"softmax"|"linear")
}


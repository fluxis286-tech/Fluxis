// FLUXIS — stdlib/gfx.rs
// 2D graphics: terminal canvas (ASCII) + pixel image buffer (PPM export).
// Called by Runtime — all state is passed through a thread-local.

use crate::vm::value::Value;
use crate::error::{FluxisError, runtime_error, type_error, arity_error};
use std::cell::RefCell;

// ── TERMINAL CANVAS STATE ─────────────────────────────────────────────────
thread_local! {
    static CANVAS: RefCell<Option<Vec<Vec<char>>>> = RefCell::new(None);
    static CANVAS_W: RefCell<usize> = RefCell::new(0);
    static CANVAS_H: RefCell<usize> = RefCell::new(0);
}

fn with_canvas<F: FnOnce(&mut Vec<Vec<char>>) -> Result<Value, FluxisError>>(f: F) -> Result<Value, FluxisError> {
    CANVAS.with(|c| {
        let mut c = c.borrow_mut();
        match c.as_mut() {
            Some(canvas) => f(canvas),
            None => Err(runtime_error("No canvas — call gfx_canvas(width, height) first")),
        }
    })
}

pub fn call(name: &str, args: &[Value]) -> Result<Value, FluxisError> {
    let num = |v: &Value| -> i64 { match v { Value::Number(n) => *n, Value::Float(f) => *f as i64, _ => 0 } };
    let ch  = |v: &Value| -> char { match v { Value::Str(s) => s.chars().next().unwrap_or(' '), Value::Number(n) => char::from_u32(*n as u32).unwrap_or('#'), _ => '#' } };

    match name {
        // ── TERMINAL CANVAS ───────────────────────────────────────────
        "gfx_canvas" => {
            if args.len() != 2 { return Err(arity_error("gfx_canvas", 2, args.len())); }
            let (w, h) = (num(&args[0]) as usize, num(&args[1]) as usize);
            CANVAS.with(|c| *c.borrow_mut() = Some(vec![vec![' '; w]; h]));
            CANVAS_W.with(|cw| *cw.borrow_mut() = w);
            CANVAS_H.with(|ch| *ch.borrow_mut() = h);
            Ok(Value::Nil)
        }
        "gfx_clear" => {
            with_canvas(|c| { for row in c.iter_mut() { for cell in row.iter_mut() { *cell = ' '; } } Ok(Value::Nil) })
        }
        "gfx_pixel" => {
            if args.len() != 3 { return Err(arity_error("gfx_pixel", 3, args.len())); }
            let (x, y, c) = (num(&args[0]) as usize, num(&args[1]) as usize, ch(&args[2]));
            with_canvas(|canvas| {
                if y < canvas.len() && x < canvas[y].len() { canvas[y][x] = c; }
                Ok(Value::Nil)
            })
        }
        "gfx_text" => {
            if args.len() != 3 { return Err(arity_error("gfx_text", 3, args.len())); }
            let (x, y) = (num(&args[0]) as usize, num(&args[1]) as usize);
            let text = match &args[2] { Value::Str(s) => s.clone(), v => v.display() };
            with_canvas(|canvas| {
                if y < canvas.len() {
                    for (i, ch) in text.chars().enumerate() {
                        if x + i < canvas[y].len() { canvas[y][x + i] = ch; }
                    }
                }
                Ok(Value::Nil)
            })
        }
        "gfx_rect" => {
            if args.len() != 5 { return Err(arity_error("gfx_rect", 5, args.len())); }
            let (x, y, w, h, c) = (num(&args[0]) as usize, num(&args[1]) as usize, num(&args[2]) as usize, num(&args[3]) as usize, ch(&args[4]));
            with_canvas(|canvas| {
                for xi in x..x+w {
                    if y < canvas.len() && xi < canvas[y].len() { canvas[y][xi] = c; }
                    if y+h > 0 && y+h-1 < canvas.len() && xi < canvas[y+h-1].len() { canvas[y+h-1][xi] = c; }
                }
                for yi in y..y+h {
                    if yi < canvas.len() {
                        if x < canvas[yi].len() { canvas[yi][x] = c; }
                        if x+w > 0 && x+w-1 < canvas[yi].len() { canvas[yi][x+w-1] = c; }
                    }
                }
                Ok(Value::Nil)
            })
        }
        "gfx_fill_rect" => {
            if args.len() != 5 { return Err(arity_error("gfx_fill_rect", 5, args.len())); }
            let (x, y, w, h, c) = (num(&args[0]) as usize, num(&args[1]) as usize, num(&args[2]) as usize, num(&args[3]) as usize, ch(&args[4]));
            with_canvas(|canvas| {
                for yi in y..y+h { if yi < canvas.len() { for xi in x..x+w { if xi < canvas[yi].len() { canvas[yi][xi] = c; } } } }
                Ok(Value::Nil)
            })
        }
        "gfx_line" => {
            if args.len() != 5 { return Err(arity_error("gfx_line", 5, args.len())); }
            let (mut x0, mut y0, x1, y1, c) = (num(&args[0]), num(&args[1]), num(&args[2]), num(&args[3]), ch(&args[4]));
            let (dx, dy) = ((x1-x0).abs(), (y1-y0).abs());
            let (sx, sy) = (if x0<x1{1}else{-1}, if y0<y1{1}else{-1});
            let mut err = dx - dy;
            with_canvas(|canvas| {
                loop {
                    let (xi, yi) = (x0 as usize, y0 as usize);
                    if yi < canvas.len() && xi < canvas[yi].len() { canvas[yi][xi] = c; }
                    if x0 == x1 && y0 == y1 { break; }
                    let e2 = 2 * err;
                    if e2 > -dy { err -= dy; x0 += sx; }
                    if e2 < dx  { err += dx; y0 += sy; }
                }
                Ok(Value::Nil)
            })
        }
        "gfx_circle" => {
            if args.len() != 4 { return Err(arity_error("gfx_circle", 4, args.len())); }
            let (cx, cy, r, c) = (num(&args[0]), num(&args[1]), num(&args[2]), ch(&args[3]));
            with_canvas(|canvas| {
                let plot = |canvas: &mut Vec<Vec<char>>, x: i64, y: i64| {
                    if y >= 0 && x >= 0 {
                        let (xi, yi) = (x as usize, y as usize);
                        if yi < canvas.len() && xi < canvas[yi].len() { canvas[yi][xi] = c; }
                    }
                };
                let (mut x, mut y, mut d) = (0i64, r, 1-r);
                while x <= y {
                    plot(canvas,cx+x,cy+y); plot(canvas,cx-x,cy+y);
                    plot(canvas,cx+x,cy-y); plot(canvas,cx-x,cy-y);
                    plot(canvas,cx+y,cy+x); plot(canvas,cx-y,cy+x);
                    plot(canvas,cx+y,cy-x); plot(canvas,cx-y,cy-x);
                    if d < 0 { d += 2*x+3; } else { d += 2*(x-y)+5; y -= 1; }
                    x += 1;
                }
                Ok(Value::Nil)
            })
        }
        "gfx_render" => {
            CANVAS.with(|c| {
                if let Some(ref canvas) = *c.borrow() {
                    let w = canvas.first().map(|r| r.len()).unwrap_or(0);
                    println!("\x1b[2J\x1b[H");
                    print!("+"); for _ in 0..w { print!("-"); } println!("+");
                    for row in canvas { print!("|"); for ch in row { print!("{}", ch); } println!("|"); }
                    print!("+"); for _ in 0..w { print!("-"); } println!("+");
                }
                Ok(Value::Nil)
            })
        }
        "gfx_reset" => {
            CANVAS.with(|c| *c.borrow_mut() = None);
            CANVAS_W.with(|w| *w.borrow_mut() = 0);
            CANVAS_H.with(|h| *h.borrow_mut() = 0);
            Ok(Value::Nil)
        }

        // ── PIXEL IMAGE BUFFER ─────────────────────────────────────────
        // Image format: [width, height, r0,g0,b0, r1,g1,b1, ...]
        "gfx_image" => {
            if args.len() != 2 { return Err(arity_error("gfx_image", 2, args.len())); }
            let (w, h) = (num(&args[0]), num(&args[1]));
            let mut img = vec![Value::Number(w), Value::Number(h)];
            for _ in 0..w*h { img.push(Value::Number(0)); img.push(Value::Number(0)); img.push(Value::Number(0)); }
            Ok(Value::Array(img))
        }
        "gfx_set_pixel" => {
            if args.len() != 6 { return Err(arity_error("gfx_set_pixel", 6, args.len())); }
            if let Value::Array(mut img) = args[0].clone() {
                let iw = num(&img[0]);
                let (x, y, r, g, b) = (num(&args[1]), num(&args[2]), num(&args[3]).clamp(0,255), num(&args[4]).clamp(0,255), num(&args[5]).clamp(0,255));
                let ih = num(&img[1]);
                if x >= 0 && y >= 0 && x < iw && y < ih {
                    let idx = (2 + (y*iw+x)*3) as usize;
                    if idx + 2 < img.len() {
                        img[idx]   = Value::Number(r);
                        img[idx+1] = Value::Number(g);
                        img[idx+2] = Value::Number(b);
                    }
                }
                Ok(Value::Array(img))
            } else { Err(type_error("gfx_set_pixel() first arg must be a gfx image")) }
        }
        "gfx_fill" => {
            if args.len() != 4 { return Err(arity_error("gfx_fill", 4, args.len())); }
            if let Value::Array(mut img) = args[0].clone() {
                let (iw, ih) = (num(&img[0]), num(&img[1]));
                let (r, g, b) = (num(&args[1]).clamp(0,255), num(&args[2]).clamp(0,255), num(&args[3]).clamp(0,255));
                for i in 0..iw*ih {
                    let idx = (2 + i*3) as usize;
                    if idx + 2 < img.len() { img[idx]=Value::Number(r); img[idx+1]=Value::Number(g); img[idx+2]=Value::Number(b); }
                }
                Ok(Value::Array(img))
            } else { Err(type_error("gfx_fill() first arg must be a gfx image")) }
        }
        "gfx_draw_rect" => {
            if args.len() != 8 { return Err(arity_error("gfx_draw_rect", 8, args.len())); }
            if let Value::Array(mut img) = args[0].clone() {
                let iw = num(&img[0]);
                let (rx, ry, rw, rh) = (num(&args[1]), num(&args[2]), num(&args[3]), num(&args[4]));
                let (r, g, b) = (num(&args[5]).clamp(0,255), num(&args[6]).clamp(0,255), num(&args[7]).clamp(0,255));
                for dy in 0..rh { for dx in 0..rw {
                    let idx = (2 + ((ry+dy)*iw+(rx+dx))*3) as usize;
                    if idx + 2 < img.len() { img[idx]=Value::Number(r); img[idx+1]=Value::Number(g); img[idx+2]=Value::Number(b); }
                }}
                Ok(Value::Array(img))
            } else { Err(type_error("gfx_draw_rect() first arg must be a gfx image")) }
        }
        "gfx_draw_circle" => {
            if args.len() != 7 { return Err(arity_error("gfx_draw_circle", 7, args.len())); }
            if let Value::Array(mut img) = args[0].clone() {
                let (iw, ih) = (num(&img[0]), num(&img[1]));
                let (cx, cy, cr) = (num(&args[1]), num(&args[2]), num(&args[3]));
                let (r, g, b) = (num(&args[4]).clamp(0,255), num(&args[5]).clamp(0,255), num(&args[6]).clamp(0,255));
                for dy in -cr..=cr { for dx in -cr..=cr {
                    if dx*dx+dy*dy <= cr*cr {
                        let (px, py) = (cx+dx, cy+dy);
                        if px>=0&&py>=0&&px<iw&&py<ih {
                            let idx = (2+(py*iw+px)*3) as usize;
                            if idx+2<img.len() { img[idx]=Value::Number(r); img[idx+1]=Value::Number(g); img[idx+2]=Value::Number(b); }
                        }
                    }
                }}
                Ok(Value::Array(img))
            } else { Err(type_error("gfx_draw_circle() first arg must be a gfx image")) }
        }
        "gfx_draw_line" => {
            if args.len() != 8 { return Err(arity_error("gfx_draw_line", 8, args.len())); }
            if let Value::Array(mut img) = args[0].clone() {
                let (iw, ih) = (num(&img[0]), num(&img[1]));
                let (mut x0, mut y0, x1, y1) = (num(&args[1]), num(&args[2]), num(&args[3]), num(&args[4]));
                let (r, g, b) = (num(&args[5]).clamp(0,255), num(&args[6]).clamp(0,255), num(&args[7]).clamp(0,255));
                let (dx, dy) = ((x1-x0).abs(), (y1-y0).abs());
                let (sx, sy) = (if x0<x1{1}else{-1}, if y0<y1{1}else{-1});
                let mut err = dx - dy;
                loop {
                    if x0>=0&&y0>=0&&x0<iw&&y0<ih {
                        let idx = (2+(y0*iw+x0)*3) as usize;
                        if idx+2<img.len() { img[idx]=Value::Number(r); img[idx+1]=Value::Number(g); img[idx+2]=Value::Number(b); }
                    }
                    if x0==x1&&y0==y1 { break; }
                    let e2=2*err;
                    if e2>-dy{err-=dy;x0+=sx;}
                    if e2<dx{err+=dx;y0+=sy;}
                }
                Ok(Value::Array(img))
            } else { Err(type_error("gfx_draw_line() first arg must be a gfx image")) }
        }
        "gfx_gradient" => {
            if args.len() != 8 { return Err(arity_error("gfx_gradient", 8, args.len())); }
            if let Value::Array(mut img) = args[0].clone() {
                let (iw, ih) = (num(&img[0]), num(&img[1]));
                let (r1,g1,b1) = (num(&args[1]),num(&args[2]),num(&args[3]));
                let (r2,g2,b2) = (num(&args[4]),num(&args[5]),num(&args[6]));
                let dir = num(&args[7]);
                for py in 0..ih { for px in 0..iw {
                    let t = if dir==0 { if iw<=1{0}else{px*1000/(iw-1)} } else { if ih<=1{0}else{py*1000/(ih-1)} };
                    let (cr,cg,cb) = ((r1+(r2-r1)*t/1000).clamp(0,255),(g1+(g2-g1)*t/1000).clamp(0,255),(b1+(b2-b1)*t/1000).clamp(0,255));
                    let idx = (2+(py*iw+px)*3) as usize;
                    if idx+2<img.len() { img[idx]=Value::Number(cr); img[idx+1]=Value::Number(cg); img[idx+2]=Value::Number(cb); }
                }}
                Ok(Value::Array(img))
            } else { Err(type_error("gfx_gradient() first arg must be a gfx image")) }
        }
        "gfx_save" => {
            if args.len() != 2 { return Err(arity_error("gfx_save", 2, args.len())); }
            if let Value::Array(ref img) = args[0] {
                let w = match &img[0] { Value::Number(n) => *n, _ => return Err(type_error("Invalid image")) };
                let h = match &img[1] { Value::Number(n) => *n, _ => return Err(type_error("Invalid image")) };
                let fname = match &args[1] { Value::Str(s) => s.clone(), _ => return Err(type_error("gfx_save() filename must be string")) };
                let header = format!("P6\n{} {}\n255\n", w, h);
                let mut bytes: Vec<u8> = header.into_bytes();
                for i in 0..w*h {
                    let idx = (2 + i*3) as usize;
                    let r = match img.get(idx)   { Some(Value::Number(n)) => (*n).clamp(0,255) as u8, _ => 0 };
                    let g = match img.get(idx+1) { Some(Value::Number(n)) => (*n).clamp(0,255) as u8, _ => 0 };
                    let b = match img.get(idx+2) { Some(Value::Number(n)) => (*n).clamp(0,255) as u8, _ => 0 };
                    bytes.push(r); bytes.push(g); bytes.push(b);
                }
                std::fs::write(&fname, &bytes)
                    .map_err(|e| runtime_error(&format!("gfx_save() failed to write '{}': {}", fname, e)))?;
                println!("Image saved: {} ({}x{} PPM)", fname, w, h);
                Ok(Value::Str(fname))
            } else { Err(type_error("gfx_save() first arg must be a gfx image")) }
        }
        _ => Err(runtime_error(&format!("Unknown gfx function '{}'", name))),
    }
}

pub fn is_gfx_fn(name: &str) -> bool {
    name.starts_with("gfx_")
}


// FLUXIS v9.0 — main.rs
// Execution model: source → Lexer → Parser → Compiler → Core + Runtime
//
//   fluxis                    → REPL
//   fluxis program.fx         → run file
//   fluxis --dis program.fx   → disassemble bytecode
//   fluxis --check program.fx → lint (lex + parse only)
//   fluxis --test program.fx  → run all test_*() functions
//   fluxis --version          → version info
//   fluxis --help             → usage

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::time::Instant;

mod ast;
mod compiler;
mod dop;
mod error;
mod lexer;
mod parser;
mod stdlib;
mod token;
mod vm;

use crate::compiler::Compiler;
use crate::vm::{Core, Runtime};
use error::FluxisError;
use lexer::Lexer;
use parser::Parser;

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const GRAY: &str = "\x1b[90m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const BLUE: &str = "\x1b[34m";
const WHITE: &str = "\x1b[97m";

const VERSION: &str = "9.0.0";
const TAGLINE: &str = "Bytecode VM · DOP · Actors · Messaging · AI · ML · 2D Graphics";

// ── SESSION ───────────────────────────────────────────────────────────────
struct Session {
    rt: Runtime,
    core: Core,
}

impl Session {
    fn new() -> Self {
        Self {
            rt: Runtime::new(HashMap::new()),
            core: Core::new(),
        }
    }

    fn run(&mut self, source: &str, filename: &str) -> bool {
        let tokens = match Lexer::new(source).lex() {
            Ok(t) => t,
            Err(e) => {
                print_file_error(&e, filename);
                return false;
            }
        };
        let program = match Parser::new(tokens, source).parse() {
            Ok(p) => p,
            Err(e) => {
                print_file_error(&e, filename);
                return false;
            }
        };
        let mut compiler = Compiler::new();
        // Seed compiler with already-known struct/enum/dotion types for REPL continuity
        let chunk = match compiler.compile(program) {
            Ok(c) => c,
            Err(_) => {
                if compiler.errors.len() == 1 {
                    print_file_error(&compiler.errors[0], filename);
                } else {
                    eprintln!(
                        "\n\x1b[1m\x1b[36m[{}]\x1b[0m Found {} errors:\n",
                        filename,
                        compiler.errors.len()
                    );
                    for (i, e) in compiler.errors.iter().enumerate() {
                        eprintln!("  \x1b[1m[{}]\x1b[0m", i + 1);
                        print_file_error(e, filename);
                    }
                }
                return false;
            }
        };
        for (name, fn_chunk) in compiler.functions {
            self.rt.fn_chunks.insert(name, fn_chunk);
        }
        if self.rt.fn_chunks.contains_key("__tick_block__") {
            self.rt.tick_block = Some("__tick_block__".to_string());
        }
        // Use persistent core so REPL variables survive across lines
        match self.core.run(&chunk, &mut self.rt) {
            Ok(_) => true,
            Err(e) => {
                print_file_error(&e, filename);
                false
            }
        }
    }

    fn reset(&mut self) {
        self.rt = Runtime::new(HashMap::new());
        self.core = Core::new();
    }
}

fn print_file_error(e: &FluxisError, filename: &str) {
    eprint!("{}{}[{}]{} ", BOLD, CYAN, filename, RESET);
    e.display();
}

fn run_check(source: &str, filename: &str) -> bool {
    let tokens = match Lexer::new(source).lex() {
        Ok(t) => t,
        Err(e) => {
            print_file_error(&e, filename);
            return false;
        }
    };
    match Parser::new(tokens, source).parse() {
        Ok(_) => true,
        Err(e) => {
            print_file_error(&e, filename);
            false
        }
    }
}

fn run_dis(source: &str, filename: &str) {
    let tokens = match Lexer::new(source).lex() {
        Ok(t) => t,
        Err(e) => {
            print_file_error(&e, filename);
            return;
        }
    };
    let program = match Parser::new(tokens, source).parse() {
        Ok(p) => p,
        Err(e) => {
            print_file_error(&e, filename);
            return;
        }
    };
    let mut compiler = Compiler::new();
    let chunk = match compiler.compile(program) {
        Ok(c) => c,
        Err(e) => {
            e.display();
            return;
        }
    };
    Core::disassemble(&chunk);
    for (name, fn_chunk) in &compiler.functions {
        println!("{}{}fn: {}{}", BOLD, CYAN, name, RESET);
        Core::disassemble(fn_chunk);
    }
}

fn run_file(path: &str) {
    if !path.ends_with(".fx") {
        eprintln!(
            "\n{}{}Error:{} FLUXIS source files must have the .fx extension.",
            BOLD, RED, RESET
        );
        std::process::exit(1);
    }
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "\n{}{}Error:{} Cannot open '{}': {}",
                BOLD, RED, RESET, path, e
            );
            std::process::exit(1);
        }
    };
    let filename = std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    println!();
    print_banner_small(&filename);
    println!();
    io::stdout().flush().unwrap();
    let mut session = Session::new();
    let t0 = Instant::now();
    let ok = session.run(&source, &filename);
    let elapsed = t0.elapsed();
    println!();
    println!("{}{}  ─────────────────────────────{}", DIM, GRAY, RESET);
    if ok {
        println!(
            "{}{}  ✓ Done{}  {}({:.0?}){}",
            BOLD, GREEN, RESET, GRAY, elapsed, RESET
        );
    } else {
        println!("{}{}  ✗ Failed{}", BOLD, RED, RESET);
        std::process::exit(1);
    }
    println!();
}

fn check_file(path: &str) {
    if !path.ends_with(".fx") {
        eprintln!("{}Error:{} Not a .fx file.", BOLD, RESET);
        std::process::exit(1);
    }
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}Error:{} Cannot open '{}': {}", BOLD, RESET, path, e);
            std::process::exit(1);
        }
    };
    let filename = std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    println!();
    println!(
        "{}{}  🔍 FLUXIS Check{}  {}{}{}",
        BOLD, CYAN, RESET, GRAY, filename, RESET
    );
    println!("{}{}  ─────────────────────────────{}", DIM, GRAY, RESET);
    println!();
    let ok = run_check(&source, &filename);
    println!();
    println!("{}{}  ─────────────────────────────{}", DIM, GRAY, RESET);
    if ok {
        let lines = source.lines().count();
        let non_empty = source
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim().starts_with("//"))
            .count();
        println!(
            "{}{}  ✓ No errors{}  {}({} lines, {} non-empty){}",
            BOLD, GREEN, RESET, GRAY, lines, non_empty, RESET
        );
    } else {
        println!("{}{}  ✗ Errors found{}", BOLD, RED, RESET);
        std::process::exit(1);
    }
    println!();
}

fn run_repl() {
    print_banner_full();
    println!("  {}Commands:{}", BOLD, RESET);
    println!("  {}  exit{}       — quit", CYAN, RESET);
    println!("  {}  :clear{}     — reset session", CYAN, RESET);
    println!("  {}  :help{}      — quick reference", CYAN, RESET);
    println!(
        "{}{}  ─────────────────────────────────────{}",
        DIM, GRAY, RESET
    );
    println!();

    let mut session = Session::new();
    let mut session_line = 0usize;

    loop {
        let mut input = String::new();
        let mut line = String::new();
        let mut brace_count = 0i32;
        let mut first_line = true;

        print!(
            "{}{}fluxis {}{}[{}]{} >> {}",
            BOLD,
            CYAN,
            RESET,
            GRAY,
            session_line + 1,
            RESET,
            RESET
        );
        io::stdout().flush().unwrap();

        loop {
            line.clear();
            io::stdin().read_line(&mut line).unwrap();
            let trimmed = line.trim();

            if first_line {
                match trimmed {
                    "exit" | "quit" | ":exit" | ":quit" => {
                        println!();
                        println!("{}{}  👋 Goodbye!{}", BOLD, YELLOW, RESET);
                        println!();
                        return;
                    }
                    ":clear" | ":reset" => {
                        session.reset();
                        session_line = 0;
                        println!("{}{}  ✓ Session reset.{}", BOLD, GREEN, RESET);
                        println!();
                        print!(
                            "{}{}fluxis {}{}[{}]{} >> {}",
                            BOLD,
                            CYAN,
                            RESET,
                            GRAY,
                            session_line + 1,
                            RESET,
                            RESET
                        );
                        io::stdout().flush().unwrap();
                        continue;
                    }
                    ":help" | ":h" => {
                        print_quick_ref();
                        println!();
                        print!(
                            "{}{}fluxis {}{}[{}]{} >> {}",
                            BOLD,
                            CYAN,
                            RESET,
                            GRAY,
                            session_line + 1,
                            RESET,
                            RESET
                        );
                        io::stdout().flush().unwrap();
                        continue;
                    }
                    _ => {}
                }
            }

            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;
            input.push_str(&line);
            first_line = false;
            if brace_count <= 0 && !input.trim().is_empty() {
                break;
            }
            print!("{}{}  ...  {}", DIM, GRAY, RESET);
            io::stdout().flush().unwrap();
        }

        let trimmed_input = input.trim();
        if trimmed_input.is_empty() {
            continue;
        }

        let wrapped = if !trimmed_input.contains("start")
            && !trimmed_input.starts_with("fn ")
            && !trimmed_input.starts_with("dotion ")
            && !trimmed_input.starts_with("actor ")
            && !trimmed_input.starts_with("struct ")
            && !trimmed_input.starts_with("enum ")
            && !trimmed_input.starts_with("import ")
        {
            format!("start {{\n{}\n}}", trimmed_input)
        } else {
            trimmed_input.to_string()
        };

        session_line += 1;
        let t0 = Instant::now();
        let ok = session.run(&wrapped, "repl");
        let elapsed = t0.elapsed();
        if ok && elapsed.as_millis() > 0 {
            println!("{}  ({:.0?}){}", GRAY, elapsed, RESET);
        }
        println!();
    }
}

fn run_tests(path: &str) {
    if !path.ends_with(".fx") {
        eprintln!("{}Error:{} Not a .fx file.", BOLD, RESET);
        std::process::exit(1);
    }
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "\n{}{}Error:{} Cannot open '{}': {}",
                BOLD, RED, RESET, path, e
            );
            std::process::exit(1);
        }
    };
    let filename = std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    println!();
    println!(
        "{}{}  🧪 FLUXIS Test Runner{}  {}{}{}",
        BOLD, CYAN, RESET, DIM, filename, RESET
    );
    println!("{}{}  ─────────────────────────────{}", DIM, GRAY, RESET);
    println!();

    let tokens = match Lexer::new(&source).lex() {
        Ok(t) => t,
        Err(e) => {
            e.display();
            std::process::exit(1);
        }
    };
    let program = match Parser::new(tokens, &source).parse() {
        Ok(p) => p,
        Err(e) => {
            e.display();
            std::process::exit(1);
        }
    };

    let test_names: Vec<String> = program
        .iter()
        .filter_map(|s| {
            if let crate::ast::Statement::FunctionDef { name, .. } = s {
                if name.starts_with("test_") {
                    return Some(name.clone());
                }
            }
            None
        })
        .collect();

    if test_names.is_empty() {
        println!("{}  No test functions found.{}", GRAY, RESET);
        println!(
            "{}  Define functions named test_*() to add tests.{}",
            DIM, RESET
        );
        println!();
        return;
    }

    println!("{}  Found {} test(s){}", GRAY, test_names.len(), RESET);
    println!();

    let (mut passed, mut failed) = (0usize, 0usize);
    for test_name in &test_names {
        let runner_src = format!("{}\nstart {{ {}(); }}", source, test_name);
        let t0 = Instant::now();
        let mut session = Session::new();
        let ok = session.run(&runner_src, &filename);
        let elapsed = t0.elapsed();
        if ok {
            println!(
                "  {}✓{}  {}{}{}  {}({:.0?}){}",
                GREEN, RESET, BOLD, test_name, RESET, GRAY, elapsed, RESET
            );
            passed += 1;
        } else {
            println!("  {}✗{}  {}{}{}", RED, RESET, BOLD, test_name, RESET);
            failed += 1;
        }
    }

    println!();
    println!("{}{}  ─────────────────────────────{}", DIM, GRAY, RESET);
    if failed == 0 {
        println!("{}{}  ✓ {} passed{}", BOLD, GREEN, passed, RESET);
    } else {
        println!(
            "{}{}  ✗ {} failed{}  {}  {} passed{}",
            BOLD, RED, failed, RESET, GRAY, passed, RESET
        );
        std::process::exit(1);
    }
    println!();
}

fn print_banner_full() {
    println!();
    println!("{}{}  ██████╗ FLUXIS  v{}{}", BOLD, CYAN, VERSION, RESET);
    println!("{}{}  {}{}", DIM, GRAY, TAGLINE, RESET);
    println!(
        "{}{}  Built by Dipanshu & Suyogya — age 16{}",
        DIM, GRAY, RESET
    );
    println!(
        "{}{}  ─────────────────────────────────────{}",
        DIM, GRAY, RESET
    );
    println!();
}

fn print_banner_small(filename: &str) {
    println!(
        "{}{}  🔥 FLUXIS v{}{}  {}{}{}",
        BOLD, CYAN, VERSION, RESET, DIM, GRAY, RESET
    );
    println!("{}{}  ▶  {}{}", BOLD, WHITE, filename, RESET);
    println!("{}{}  ─────────────────────────────{}", DIM, GRAY, RESET);
}

fn print_quick_ref() {
    println!();
    println!("{}{}  FLUXIS Quick Reference{}", BOLD, WHITE, RESET);
    println!(
        "{}{}  ─────────────────────────────────────{}",
        DIM, GRAY, RESET
    );
    let items = [
        ("Variables", "x = 10;   x: num = 10;   x = nil;"),
        ("Output", "out(value);"),
        ("Input", "x = in();"),
        ("If", "if(cond){ }  else{ }"),
        ("While", "while(cond){ }"),
        ("For", "for(i=0; i<10; i++;){ }"),
        ("For-in", "for item in arr { }"),
        ("Functions", "fn name(a, b){ return a+b; }"),
        ("Arrays", "[1,2,3]  push(arr,v)  len(arr)  arr[0]"),
        ("Maps", r#"{"key": val}  has(m,"k")  del(m,"k")"#),
        ("Structs", "struct Point{ x, y }  p = Point{x:1,y:2};"),
        ("Enums", "enum Color{ Red,Green }  c = Color::Red;"),
        (
            "Dotion",
            "dotion Name{ field:0, fn m(){ self.field += 1; } }",
        ),
        ("Tick", "tick{ }   tick(10);"),
        ("Messaging", "send(d,\"msg\",val);  broadcast(\"msg\",val);"),
        ("AI", "import \"ai\";  ai_set_key(k);  ai_ask(prompt);"),
        ("ML", "import \"ml\";  ml_zeros(3,3)  ml_matmul(a,b)"),
        (
            "Graphics",
            "import \"gfx\";  gfx_canvas(40,20)  gfx_render();",
        ),
        ("File I/O", "read_file(p)  write_file(p,s)  file_exists(p)"),
        (
            "Program",
            "exit(0);  sleep(500);  time_now()  assert(cond,\"msg\");",
        ),
        (
            "Imports",
            "import \"utils.fx\";  — load defs from another .fx file",
        ),
    ];
    for (category, example) in &items {
        println!(
            "  {}{:<12}{}  {}{}{}",
            BOLD, category, RESET, GRAY, example, RESET
        );
    }
    println!(
        "{}{}  ─────────────────────────────────────{}",
        DIM, GRAY, RESET
    );
}

fn print_help() {
    println!();
    println!("{}{}FLUXIS v{}{}", BOLD, CYAN, VERSION, RESET);
    println!("{}  {}{}", GRAY, TAGLINE, RESET);
    println!();
    println!("{}{}USAGE:{}", BOLD, WHITE, RESET);
    println!(
        "  {}fluxis{}                    Start interactive REPL",
        CYAN, RESET
    );
    println!(
        "  {}fluxis <file.fx>{}           Run a source file",
        CYAN, RESET
    );
    println!(
        "  {}fluxis --check  <file.fx>{}  Check syntax only",
        CYAN, RESET
    );
    println!(
        "  {}fluxis --test   <file.fx>{}  Run all test_*() functions",
        CYAN, RESET
    );
    println!(
        "  {}fluxis --dis    <file.fx>{}  Disassemble to bytecode",
        CYAN, RESET
    );
    println!("  {}fluxis --version{}           Show version", CYAN, RESET);
    println!(
        "  {}fluxis --help{}              Show this message",
        CYAN, RESET
    );
    println!();
    println!("{}{}DOCS:{}", BOLD, WHITE, RESET);
    println!("  {}https://fluxislang.netlify.app{}", BLUE, RESET);
    println!();
}

fn print_version() {
    println!();
    println!("{}{}FLUXIS v{}{}", BOLD, CYAN, VERSION, RESET);
    println!("{}  Bytecode VM · built in Rust on Termux", GRAY);
    println!("  Built by Dipanshu & Suyogya — age 16");
    println!("  https://fluxislang.netlify.app{}", RESET);
    println!();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => run_repl(),
        2 => match args[1].as_str() {
            "--help" | "-h" => print_help(),
            "--version" | "-v" => print_version(),
            path => run_file(path),
        },
        3 => {
            let flag = &args[1];
            let path = &args[2];
            let source = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "\n{}{}Error:{} Cannot open '{}': {}",
                        BOLD, RED, RESET, path, e
                    );
                    std::process::exit(1);
                }
            };
            let filename = std::path::Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            match flag.as_str() {
                "--dis" => {
                    println!();
                    println!(
                        "{}{}  🔍 Disassembler{}  {}{}{}",
                        BOLD, CYAN, RESET, GRAY, filename, RESET
                    );
                    println!();
                    run_dis(&source, &filename);
                }
                "--check" => check_file(path),
                "--test" => run_tests(path),
                _ => {
                    eprintln!("\n{}{}Error:{} Unknown flag '{}'", BOLD, RED, RESET, flag);
                    eprintln!("  Run {}fluxis --help{} for usage.", CYAN, RESET);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("{}{}Error:{} Too many arguments.", BOLD, RED, RESET);
            std::process::exit(1);
        }
    }
}

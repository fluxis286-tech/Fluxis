# Fluxis
<div align="center">

# 🔥 FLUXIS

**A programming language built at 16, on a phone, in Rust.**

*DOP · Actors · Messaging · AI/LLM · ML · 2D Graphics · Bytecode VM*

[![version](https://img.shields.io/badge/version-4.0.0-cyan?style=flat-square)](https://github.com/dqgamer75-oss/Fluxis)
[![language](https://img.shields.io/badge/built%20in-Rust-orange?style=flat-square)](https://www.rust-lang.org)
[![platform](https://img.shields.io/badge/runs%20on-Termux%20%7C%20Linux%20%7C%20Windows-green?style=flat-square)](https://fluxislang.netlify.app)
[![license](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

[Website](https://fluxislang.netlify.app) · [Docs](https://fluxislang.netlify.app/#docs) · [Playground](https://fluxislang.netlify.app/#playground)

</div>

---

## What is FLUXIS?

FLUXIS is an original programming language written entirely in Rust — designed, built, and maintained by **Dipanshu** and **Suyogya**, both age 16, using Termux on an Android phone.

It introduces **Dotion-Oriented Programming (DOP)** — a paradigm where programs are built from stateful *Dotions* that evolve over time via a tick engine, communicate via messages, and are driven by autonomous Actor brains.

```
dotion Enemy {
    health: 100,
    on "damage"(amt) {
        self.health -= amt..
    }
}

start {
    tick { }
    e = dotion Enemy{}..
    send(e, "damage", 30)..
    tick(1)..
    out(e.health)..   // 70
}
```

---

## Features

| Feature | Description |
|---|---|
| **DOP** | Dotions — stateful entities with methods, message handlers, actor brains |
| **Tick Engine** | Time-based simulation: `tick{ }` + `tick(n)..` |
| **Messaging** | `send(d, "msg", val)..` · `broadcast("msg", val)..` |
| **Actors** | Pure decision-makers attached to Dotions via `with ActorName` |
| **AI / LLM** | `import "ai"..` — talk to Claude directly from FLUXIS code |
| **ML Library** | Matrices, activations, loss functions, neural net layers |
| **2D Graphics** | Terminal canvas + PPM image engine |
| **Bytecode VM** | Compiler → 40+ instruction stack machine + disassembler |
| **Type system** | Optional annotations: `x: num = 10..` · `fn:num add(a,b)` |
| **Match** | Pattern matching on numbers, strings, enums, booleans, nil |
| **For-in** | `for item in arr { }` · `for ch in "string" { }` |
| **Compound ops** | `+=` `-=` `*=` `/=` `%=` everywhere including `self.field +=` |
| **String escapes** | `\n` `\t` `\\` `\"` in string literals |
| **format()** | `format("Hello {}!", name)` |
| **range()** | `range(0, 10)` · `range(0, 10, 2)` · `range(5, 0, -1)` |
| **Array ops** | `sort_arr` `sort_desc` `reverse` `slice` `insert` `remove` `flatten` `zip` |
| **HOF** | `map_fn` `filter_fn` `reduce_fn` `any_fn` `all_fn` |

---

## Install

### Linux / macOS / Termux (one command)

```bash
curl -fsSL https://raw.githubusercontent.com/dqgamer75-oss/Fluxis/main/install.sh | bash
```

Or manually:

```bash
git clone https://github.com/dqgamer75-oss/Fluxis
cd Fluxis
cargo build --release
# Binary is at target/release/fluxis
# Add to PATH:
cp target/release/fluxis ~/.local/bin/fluxis
```

### Windows

```powershell
irm https://raw.githubusercontent.com/dqgamer75-oss/Fluxis/main/install.ps1 | iex
```

Or download the pre-built binary from [Releases](https://github.com/dqgamer75-oss/Fluxis/releases).

### Requirements

- **Rust 1.70+** (only needed to build from source)
- No runtime dependencies — single binary

---

## Usage

```bash
fluxis                     # Start the interactive REPL
fluxis program.fx          # Run a source file
fluxis --check program.fx  # Check syntax without running
fluxis --vm    program.fx  # Run using the bytecode VM
fluxis --dis   program.fx  # Disassemble to bytecode
fluxis --version           # Show version info
fluxis --help              # Show usage
```

---

## Language Tour

### Variables & Types
```
x = 10..
name: str = "Dipanshu"..
ratio: float = 3.14..
active: bool = true..
empty = nil..
```

### Control Flow
```
if(x > 5){ out("big").. } else { out("small").. }

for i in range(0, 5) {
    out(i)..
}

match color {
    Color::Red   => { out("red").. }
    Color::Green => { out("green").. }
    _            => { out("other").. }
}
```

### Functions
```
fn:num factorial(n) {
    if(n <= 1){ return 1.. }
    return n * factorial(n - 1)..
}
out(factorial(10))..
```

### Arrays & Maps
```
arr = [1, 2, 3, 4, 5]..
out(sort_arr(arr))..
out(map_fn(arr, "double"))..
out(filter_fn(arr, "is_even"))..

m = {"name": "Dipanshu", "age": 16}..
out(has(m, "name"))..
```

### DOP — Dotion-Oriented Programming
```
dotion Counter {
    count: 0,
    fn inc() { self.count += 1.. }
    fn reset() { self.count = 0.. }
}

start {
    c = dotion Counter{}..
    c.inc()..
    c.inc()..
    out(c.count)..   // 2
}
```

### AI Integration
```
import "ai"..
start {
    ai_set_key("sk-ant-...")..
    reply = ai_ask("What is DOP?")..
    out(reply)..
}
```

### ML
```
import "ml"..
start {
    w = ml_random_weights(4, 3)..
    inputs = [0.5, 0.8, 0.2]..
    biases = [0.0, 0.0, 0.0, 0.0]..
    out = ml_layer_forward(inputs, w, biases)..
    activated = relu_arr(out)..
    out(activated)..
}
```

---

## Project Structure

```
src/
  token.rs        — Token types and spans
  lexer.rs        — Source → token stream
  ast.rs          — Abstract syntax tree nodes
  parser.rs       — Token stream → AST
  interpreter.rs  — Tree-walking interpreter (main execution path)
  compiler.rs     — AST → bytecode compiler
  bytecode.rs     — Instruction set and Chunk type
  vm.rs           — Stack-based bytecode VM
  dop.rs          — DOP runtime: Dotions, Actors, TickEngine
  stdlib.rs       — Standard library module registry
  error.rs        — Typed error system with source display
  main.rs         — CLI: REPL, file runner, --check, --vm, --dis
```

---

## Roadmap

- [x] Lexer · Parser · AST · Interpreter
- [x] Bytecode VM + Disassembler
- [x] Dotion-Oriented Programming
- [x] Actors + Messaging + Tick Engine
- [x] AI/LLM module (via curl, works on Termux)
- [x] ML library (matrices, activations, neural nets)
- [x] 2D Graphics (terminal canvas + PPM)
- [x] Float type · Type annotations · Typed returns
- [x] Match · For-in · Compound assignment · nil · format()
- [ ] Multi-file imports (`import "mymodule.fx"`)
- [ ] Full DOP bytecode VM (currently interpreter-only for DOP)
- [ ] Native compilation (LLVM or Cranelift backend)

---

## Built By

**Dipanshu** — Founder · Language Designer · Compiler Engineer  
Designed and built the entire compiler stack in Rust on an Android phone using Termux. Invented the DOP paradigm.

**Suyogya** — Co-founder · Frontend Engineer  
Built the website, live playground, and browser-side FLUXIS interpreter.

> *"Why do programs just run once and stop? What if they could live — evolving through time, with methods, messaging, and autonomous actor brains making decisions each tick?"*
> — Dipanshu

---

<div align="center">

**[fluxislang.netlify.app](https://fluxislang.netlify.app)** · [@project_fluxis](https://instagram.com/project_fluxis) · [fluxis286@gmail.com](mailto:fluxis286@gmail.com)

</div>


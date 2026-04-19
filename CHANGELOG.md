# FLUXIS — Changelog

All notable changes to FLUXIS are documented here.

---

## [4.0.0] — 2025

### Added
- `match` expression — pattern match on numbers, strings, booleans, nil, and enum variants
- `for item in iterable` — for-in loop over arrays, strings, and map keys
- Compound assignment operators — `+=` `-=` `*=` `/=` `%=` everywhere, including `self.field +=`
- `nil` keyword — explicit nil literal in source code
- String escape sequences — `\n` `\t` `\\` `\"` in string literals
- `format()` — string interpolation: `format("Hello {}!", name)`
- `range(start, end)` / `range(start, end, step)` — generate number arrays
- Array builtins — `sort_arr` `sort_desc` `reverse` `slice` `insert` `remove` `flatten` `zip`
- Higher-order array functions — `map_fn` `filter_fn` `reduce_fn` `any_fn` `all_fn`
- `dotion_list()` — get all dotions currently in scope
- REPL commands — `:clear` `:vars` `:help` with auto-wrap for bare statements
- `--check` flag — lint syntax without executing
- `--version` / `--help` flags
- Execution time shown after file runs
- VS Code extension — syntax highlighting, 30+ snippets, run button, hover docs

### Fixed
- Parser: `Identifier {` no longer greedily consumed as struct init inside `for-in` blocks
- Compiler: duplicate match/for-in/compound-assign arms removed
- Error display: extra argument in format string removed

---

## [3.0.0] — 2025

### Added
- Dotion-Oriented Programming (DOP) — original paradigm by Dipanshu
- Dotions — stateful entities with fields, methods (`fn`), message handlers (`on`)
- `self` keyword — reference to current dotion inside methods and handlers
- Actor brains — `actor` definition + `with ActorName` attachment
- Tick engine — `tick { }` block + `tick(n)..` to advance simulation
- Messaging — `send(dotion, "msg", val)..` and `broadcast("msg", val)..`
- `tick_count()` — returns current tick number
- Bytecode VM — full stack-based virtual machine with 40+ instructions
- Compiler — AST → bytecode with jump patching and scope management
- Disassembler — `fluxis --dis file.fx`
- Native `float` type — `3.14` literals, `to_float()`, mixed int/float arithmetic
- `%` modulo operator + `rand(min, max)` + `rand_float()`
- Typed function returns — `fn:num add(a, b)`
- `break` and `continue` in all loop types
- `i++..` and `i--..` increment/decrement
- `import "ai"` — `ai_ask` `ai_model` `ai_chat` (Claude via curl, works on Termux)
- `import "ml"` — matrices, activations, loss functions, neural net layers
- `import "gfx"` — terminal canvas + PPM image engine
- `.fx` file extension

---

## [2.0.0] — 2025

### Added
- Optional type annotations — `x: num = 10..` `fn:num add(a: num, b: num)`
- Arrays — `[1, 2, 3]` literals, auto-expand with nil, `len` `push` `pop` `keys`
- Maps — `{"key": value}` literals, `has` `del`
- Structs — `struct Point { x, y }` + `Point { x: 1, y: 2 }`
- Enums — `enum Color { Red, Green, Blue }` + `Color::Red`
- Functions with return values — `return val..`
- `while`, `for`, `do-while` loops
- Lexical scope stack
- `type_of()` `to_str()` `to_num()` builtins
- Math stdlib — `abs` `max` `min` `pow` `sqrt` `floor` `ceil` `clamp` `sign`
- String stdlib — `upper` `lower` `trim` `split` `join` `replace` `contains` `char_at`

---

## [0.1.0] — 2025

### Initial Release
- Lexer → Parser → AST → Tree-walking Interpreter
- `start { }` entry point
- `..` statement terminator
- Variables, arithmetic, comparisons, logical operators
- `if` / `else`
- `out()` and `in()` builtins
- Multi-line REPL

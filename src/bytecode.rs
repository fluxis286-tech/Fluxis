// FLUXIS v8.0 — bytecode.rs
// Bytecode instructions for the FLUXIS Virtual Machine

use crate::vm::Value;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Instruction {
    // ── STACK ──────────────────────────────────────────────────────────
    Push(Value),    // push literal onto stack
    Pop,            // discard top of stack
    Dup,            // duplicate top of stack

    // ── VARIABLES ──────────────────────────────────────────────────────
    Load(String),   // push variable value onto stack
    Store(String),  // pop stack → store in variable
    Inc(String),    // increment variable by 1  (i++..)
    Dec(String),    // decrement variable by 1  (i--..)

    // ── ARITHMETIC ─────────────────────────────────────────────────────
    Add, Sub, Mul, Div, Mod,
    // Lt2: pop (b, a) → push a < b  (used by ForIn index check)
    Lt2,

    // ── COMPARISON ─────────────────────────────────────────────────────
    Eq, Ne, Lt, Gt, Le, Ge,

    // ── LOGICAL ────────────────────────────────────────────────────────
    And, Or, Not,

    // ── CONCATENATION ──────────────────────────────────────────────────
    Concat,  // string/mixed + (when not both numbers)

    // ── CONTROL FLOW ───────────────────────────────────────────────────
    Jump(usize),        // unconditional jump to instruction index
    JumpIfFalse(usize), // pop top, jump if falsy
    JumpIfTrue(usize),  // pop top, jump if truthy (for ||)

    // ── FUNCTIONS ──────────────────────────────────────────────────────
    Call(String, usize),      // call function by name, n args
    CallBuiltin(String, usize),
    Return,                    // return top of stack from function
    MakeFunction(String),      // register function by name (body is next chunk)
    // ── DOP ──────────────────────────────────────────────────────────
    CallMethod(String, usize),    // call method on top dotion, n args
    SendMsg,                      // pop (arg, msg_str, dotion) → enqueue message
    LoadSelf,                     // push __self__ dotion onto stack
    TickStep,                     // run one tick cycle (process mailboxes + tick block)

    // ── I/O ────────────────────────────────────────────────────────────
    Print,   // pop and print top of stack
    Input,   // push user input onto stack

    // ── ARRAYS ─────────────────────────────────────────────────────────
    MakeArray(usize),  // pop N items → make array
    IndexGet,          // pop [index, array] → push array[index]
    IndexSet,          // pop [value, index, array_name_str] → set

    // ── MAPS ───────────────────────────────────────────────────────────
    MakeMap(usize),    // pop N key-value pairs → make map
    FieldGet(String),  // pop object → push object.field
    FieldSet(String),  // pop [value, object_name] → set field

    // ── STRUCTS / ENUMS ────────────────────────────────────────────────
    MakeStruct(String, usize), // struct name, N fields
    MakeEnum(String, String),  // enum_name, variant

    // ── SCOPE ──────────────────────────────────────────────────────────
    PushScope,   // enter a new variable scope block
    PopScope,    // exit current scope block

    // ── MISC ───────────────────────────────────────────────────────────
    Nop,         // no-op (placeholder for patching jumps)
    Halt,        // stop execution
}

// A compiled chunk of bytecode (program or function body)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Chunk {
    pub name:         String,
    pub instructions: Vec<Instruction>,
    pub constants:    Vec<Value>,
}

impl Chunk {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), instructions: Vec::new(), constants: Vec::new() }
    }

    pub fn emit(&mut self, instr: Instruction) -> usize {
        self.instructions.push(instr);
        self.instructions.len() - 1
    }

    // Patch a previously emitted Jump/JumpIfFalse to point to current position
    pub fn patch_jump(&mut self, idx: usize) {
        let target = self.instructions.len();
        match &mut self.instructions[idx] {
            Instruction::Jump(t)                => *t = target,
            Instruction::JumpIfFalse(t)         => *t = target,
            Instruction::JumpIfTrue(t)          => *t = target,
            _ => panic!("patch_jump called on non-jump instruction"),
        }
    }

    pub fn current_pos(&self) -> usize { self.instructions.len() }
}

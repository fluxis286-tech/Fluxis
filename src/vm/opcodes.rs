#![allow(dead_code)]
// FLUXIS — vm/opcodes.rs
// All bytecode instruction definitions. Nothing else lives here.

use crate::vm::value::Value;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Opcode {
    // ── STACK ──────────────────────────────────────────────────────────
    Push(Value), // push literal onto operand stack
    Pop,         // discard top of stack
    Dup,         // duplicate top of stack

    // ── VARIABLES ──────────────────────────────────────────────────────
    Load(String),  // push variable value from env onto stack
    Store(String), // pop stack → bind in current env scope
    Inc(String),   // variable += 1
    Dec(String),   // variable -= 1

    // ── ARITHMETIC ─────────────────────────────────────────────────────
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt2, // pop (b, a) → push a < b  (for-loop index check)

    // ── COMPARISON ─────────────────────────────────────────────────────
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,

    // ── LOGICAL ────────────────────────────────────────────────────────
    And,
    Or,
    Not,

    // ── STRING ─────────────────────────────────────────────────────────
    Concat, // string concatenation (non-numeric +)

    // ── CONTROL FLOW ───────────────────────────────────────────────────
    Jump(usize),        // unconditional jump to instruction index
    JumpIfFalse(usize), // pop top, jump if falsy
    JumpIfTrue(usize),  // pop top, jump if truthy (used for ||)

    // ── FUNCTIONS ──────────────────────────────────────────────────────
    Call(String, usize),        // call named function with N args
    CallBuiltin(String, usize), // call stdlib builtin with N args
    Return,                     // return top-of-stack from current call frame
    MakeFunction(String),       // register function chunk by name

    // ── DOP ────────────────────────────────────────────────────────────
    CallMethod(String, usize), // call method on top-of-stack dotion
    SendMsg,                   // pop (arg, msg_str, dotion) → enqueue message
    LoadSelf,                  // push __self__ dotion onto stack
    TickStep,                  // signal DOP scheduler to run one tick cycle

    // ── I/O ────────────────────────────────────────────────────────────
    Print, // pop and print top of stack
    Input, // push user input onto stack

    // ── COLLECTIONS ────────────────────────────────────────────────────
    MakeArray(usize), // pop N items → push Array
    IndexGet,         // pop (index, collection) → push collection[index]
    IndexSet,         // pop (new_val, index, name_str) → mutate

    MakeMap(usize),   // pop N key-value pairs → push Map
    FieldGet(String), // pop object → push object.field
    FieldSet(String), // pop (new_val, object_name) → set field

    // ── TYPES ──────────────────────────────────────────────────────────
    MakeStruct(String, usize), // struct type name, N fields
    MakeEnum(String, String),  // enum_name, variant_name

    // ── SCOPE ──────────────────────────────────────────────────────────
    PushScope, // open a new env scope frame
    PopScope,  // close current env scope frame

    // ── META ───────────────────────────────────────────────────────────
    Nop,               // no-op placeholder (used during jump patching)
    Halt,              // stop execution
    TryCatch(usize),   // begin try block — target = catch block start
    TryEnd(usize),     // end try block — target = after catch block
    CatchBind(String), // bind error message to named variable
}

/// A compiled unit of bytecode — either the main program or a function body.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub name: String,
    pub instructions: Vec<Opcode>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            instructions: Vec::new(),
            constants: Vec::new(),
        }
    }

    /// Emit one opcode, returning its index (used for jump patching).
    pub fn emit(&mut self, op: Opcode) -> usize {
        self.instructions.push(op);
        self.instructions.len() - 1
    }

    /// Patch a previously emitted jump to point to the current position.
    pub fn patch_jump(&mut self, idx: usize) {
        let target = self.instructions.len();
        match &mut self.instructions[idx] {
            Opcode::Jump(t) => *t = target,
            Opcode::JumpIfFalse(t) => *t = target,
            Opcode::JumpIfTrue(t) => *t = target,
            Opcode::TryCatch(t) => *t = target,
            Opcode::TryEnd(t) => *t = target,
            _ => panic!("patch_jump called on non-jump opcode at index {}", idx),
        }
    }

    pub fn current_pos(&self) -> usize {
        self.instructions.len()
    }
}

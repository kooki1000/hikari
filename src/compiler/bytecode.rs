use crate::lexer::Span;

use super::builtins::BuiltinFn;

// ── Instruction set ───────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum Instruction {
    LoadConst(u16),  // push constants[idx] onto the stack
    LoadLocal(u16),  // push locals[slot] onto the stack
    StoreLocal(u16), // pop stack top → locals[slot]
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Equal,                      // pop two values, push Bool (==)
    LessThan,                   // pop two values, push Bool (<)
    GreaterThan,                // pop two values, push Bool (>)
    LessEqual,                  // pop two values, push Bool (<=)
    GreaterEqual,               // pop two values, push Bool (>=)
    NotEqual,                   // pop two values, push Bool (!=)
    Negate,                     // pop one value, push its arithmetic negation
    Not,                        // pop one Bool, push its negation
    JumpIfFalse(u16),           // pop Bool; jump to absolute offset if false
    JumpIfTrue(u16),            // pop Bool; jump to absolute offset if true
    Jump(u16),                  // unconditional jump to absolute offset
    Call(u16, u8),              // Call(fn_idx, arg_count)
    CallBuiltin(BuiltinFn, u8), // CallBuiltin(builtin, arg_count)
    Print,                      // pop and print top of stack
    Return,
    MakeMap(u16),       // pop 2*n values (key,val pairs), push a new Value::Map
    MakeArray(u16),     // pop n values (in order), push a new Value::Array
    GetIndex,           // pop index, pop array, push the element at index
    SetIndex,           // pop value, pop index, pop array, mutate array in place
    ArrayLen,           // pop array, push its length as Value::Int
    TryStart(u16, u16), // TryStart(catch_target ip, error_var's local slot)
    TryEnd,             // marks successful completion of a try block
    // Field names in the SOURCE order their values were pushed (RecordLit's
    // parsed field order), not necessarily the type's declared field order.
    MakeRecord(Vec<String>),
    GetField(String),             // pop a record, push the named field's value
    SetField(String),             // pop value, pop record, set the named field in place
    MakeEnum(String, String, u8), // MakeEnum(enum_name, variant, payload_count)
    // Pops a Value::Enum, pushes Bool(its variant == the given name). Does
    // NOT consume the value for later payload extraction; callers reload
    // from a local slot if they need the payload after a successful check.
    TagEquals(String),
    GetPayload(u8), // pop a Value::Enum, push payload[index] (clone)
    // Push a function value onto the stack.
    LoadFn {
        chunk_index: usize,
        arity: u8,
    },
    // Pop `capture_count` values (pushed in capture order) and push a closure
    // capturing them. The captured values are seeded into the callee's locals
    // at slots [arity, arity + capture_count) when the closure is called.
    MakeClosure {
        chunk_index: usize,
        arity: u8,
        capture_count: u8,
    },
    // Pop function value and arg_count args off the stack, call the function.
    CallValue(u8),
}

// ── Function chunk ────────────────────────────────────────────────────────────

// One compiled function: its instructions and the number of parameters
// (params occupy locals[0..param_count]).
#[derive(Debug, Clone)]
pub struct Chunk {
    pub instructions: Vec<Instruction>,
    #[allow(dead_code)] // reserved for arity checking in the type checker
    pub param_count: u8,
    // Source-span checkpoints in ascending instruction-index order: each
    // `(start, span)` says "instructions at index >= start belong to the
    // statement at `span`, until the next checkpoint". Recorded per statement
    // (expressions have no spans), so a runtime error maps to its statement's
    // source line. See `Frame::span_at`, which performs the lookup at runtime.
    pub spans: Vec<(usize, Span)>,
}

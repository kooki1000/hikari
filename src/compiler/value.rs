use std::collections::HashMap;

// ── Value (constant pool entries) ─────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    // Rc<RefCell<>> gives arrays reference semantics so mutations via
    // index-assignment are visible through aliased variables.
    Array(std::rc::Rc<std::cell::RefCell<Vec<Value>>>),
    // Same Rc<RefCell<>> reference-semantics pattern as Array: assigning a
    // record to another variable aliases the same storage.
    Record(std::rc::Rc<std::cell::RefCell<HashMap<String, Value>>>),
    // Same Rc<RefCell<>> reference-semantics pattern as Array/Record.
    Map(std::rc::Rc<std::cell::RefCell<HashMap<String, Value>>>),
    // Unlike Array/Record, enum instances have no mutation operation defined
    // on them in this design, so plain by-value Clone semantics (no
    // Rc<RefCell<>>) are correct and simpler.
    Enum {
        enum_name: String,
        variant: String,
        payload: Vec<Value>,
    },
    // A first-class function value. `captured` holds the values closed over at
    // creation time (capture-by-value); it is empty for named functions and
    // non-capturing lambdas. Captured values are seeded into the callee's
    // locals at slots [arity, arity + captured.len()) when the function runs,
    // so the body reads them as ordinary locals (no upvalue instruction).
    Function {
        chunk_index: usize,
        arity: u8,
        captured: Vec<Value>,
    },
}

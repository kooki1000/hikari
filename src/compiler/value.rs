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
    // A first-class function pointer.
    Function {
        chunk_index: usize,
        arity: u8,
    },
}

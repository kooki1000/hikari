use std::cell::RefCell;
use std::rc::Rc;

use std::collections::HashMap;

use crate::compiler::{BuiltinFn, Chunk, Instruction, Value};

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum RuntimeError {
    StackUnderflow,
    UninitializedLocal(u16),
    DivisionByZero,
    TypeMismatch,
    InvalidConversion(String),
    IndexOutOfBounds { index: i64, len: usize },
    IntegerOverflow,
    // 取り出す on an empty array: there is no valid index to report, so this
    // gets its own variant rather than overloading IndexOutOfBounds.
    EmptyArray,
    // Map key lookup failed.
    KeyNotFound(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::StackUnderflow => {
                write!(f, "スタックが空の状態で値を取り出そうとしました。")
            }
            RuntimeError::UninitializedLocal(slot) => write!(
                f,
                "変数（スロット {}）が初期化される前に使用されました。",
                slot
            ),
            RuntimeError::DivisionByZero => write!(
                f,
                "ゼロで割ることはできません。（ヒント: 割る数が０にならないか確認してください）"
            ),
            RuntimeError::TypeMismatch => write!(f, "演算で扱う値の型が一致しません。"),
            RuntimeError::InvalidConversion(msg) => write!(f, "変換に失敗しました: {}", msg),
            RuntimeError::IndexOutOfBounds { index, len } => {
                write!(f, "添字 {} は範囲外です（配列の長さ: {}）。", index, len)
            }
            RuntimeError::IntegerOverflow => {
                write!(f, "整数の計算結果が大きすぎます（オーバーフロー）。")
            }
            RuntimeError::EmptyArray => {
                write!(f, "空の配列から要素を取り出すことはできません。")
            }
            RuntimeError::KeyNotFound(key) => {
                write!(f, "辞書にキー「{}」が見つかりません。", key)
            }
        }
    }
}

// ── Call frame ────────────────────────────────────────────────────────────────

struct Frame {
    instructions: Vec<Instruction>,
    ip: usize,
    locals: Vec<Option<Value>>,
}

impl Frame {
    fn new(chunk: &Chunk, args: Vec<Value>) -> Self {
        let mut locals: Vec<Option<Value>> = vec![None; 256];
        // Seed parameter slots from args (left-to-right = slot 0, 1, …).
        for (i, arg) in args.into_iter().enumerate() {
            locals[i] = Some(arg);
        }
        Self {
            instructions: chunk.instructions.clone(),
            ip: 0,
            locals,
        }
    }
}

// ── Try/catch handler ────────────────────────────────────────────────────────

struct TryHandler {
    catch_target: usize,
    error_slot: u16,
    stack_len: usize,
    frame_depth: usize,
}

// ── VM ────────────────────────────────────────────────────────────────────────

pub struct Vm {
    constants: Vec<Value>,
    chunks: Vec<Chunk>,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    try_stack: Vec<TryHandler>,
}

enum StepResult {
    Continue,
    Halt(Option<Value>),
}

impl Vm {
    /// Construct from a script chunk (no named functions).
    #[allow(dead_code)] // used in low-level unit tests that bypass the compiler
    pub fn new(constants: Vec<Value>, instructions: Vec<Instruction>) -> Self {
        let script_chunk = Chunk {
            instructions,
            param_count: 0,
        };
        let frame = Frame::new(&script_chunk, vec![]);
        Self {
            constants,
            chunks: vec![script_chunk],
            stack: Vec::new(),
            frames: vec![frame],
            try_stack: Vec::new(),
        }
    }

    /// Construct with a full set of chunks (script + named functions).
    pub fn with_chunks(
        constants: Vec<Value>,
        chunks: Vec<Chunk>,
        script: Vec<Instruction>,
    ) -> Self {
        let script_chunk = Chunk {
            instructions: script,
            param_count: 0,
        };
        let frame = Frame::new(&script_chunk, vec![]);
        Self {
            constants,
            chunks,
            stack: Vec::new(),
            frames: vec![frame],
            try_stack: Vec::new(),
        }
    }

    fn step(&mut self) -> Result<StepResult, RuntimeError> {
        let frame = self.frames.last_mut().expect("no active frame");
        // Reached the end of a chunk without an explicit 返す.
        if frame.ip >= frame.instructions.len() {
            self.frames.pop();
            if self.frames.is_empty() {
                // The top-level script finished: the whole program is done.
                return Ok(StepResult::Halt(None));
            }
            // A called function (e.g. one returning 無) fell off the end of
            // its body: treat it as a void return and resume the caller
            // rather than halting the entire program.
            return Ok(StepResult::Continue);
        }
        let instr = frame.instructions[frame.ip].clone();
        frame.ip += 1;

        match instr {
            Instruction::LoadConst(idx) => {
                self.stack.push(self.constants[idx as usize].clone());
            }
            Instruction::LoadLocal(slot) => {
                let val = self.frames.last().unwrap().locals[slot as usize]
                    .clone()
                    .ok_or(RuntimeError::UninitializedLocal(slot))?;
                self.stack.push(val);
            }
            Instruction::StoreLocal(slot) => {
                let val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                self.frames.last_mut().unwrap().locals[slot as usize] = Some(val);
            }
            Instruction::Add => {
                let (l, r) = self.pop2()?;
                match (l, r) {
                    (Value::Str(a), Value::Str(b)) => {
                        self.stack.push(Value::Str(a + &b));
                    }
                    (l, r) => {
                        self.stack
                            .push(arith(l, r, i64::checked_add, |a, b| a + b)?);
                    }
                }
            }
            Instruction::Sub => {
                let (l, r) = self.pop2()?;
                self.stack
                    .push(arith(l, r, i64::checked_sub, |a, b| a - b)?);
            }
            Instruction::Mul => {
                let (l, r) = self.pop2()?;
                self.stack
                    .push(arith(l, r, i64::checked_mul, |a, b| a * b)?);
            }
            Instruction::Div => {
                let (l, r) = self.pop2()?;
                match &r {
                    Value::Int(0) => return Err(RuntimeError::DivisionByZero),
                    Value::Float(f) if *f == 0.0 => return Err(RuntimeError::DivisionByZero),
                    _ => {}
                }
                // checked_div also guards i64::MIN / -1, which would overflow.
                self.stack
                    .push(arith(l, r, i64::checked_div, |a, b| a / b)?);
            }
            Instruction::Mod => {
                let (l, r) = self.pop2()?;
                if let Value::Int(0) = &r {
                    return Err(RuntimeError::DivisionByZero);
                }
                // Rust's float `%` never panics on a zero divisor (it yields
                // NaN per IEEE 754), so floats need no explicit zero-check.
                self.stack
                    .push(arith(l, r, i64::checked_rem, |a, b| a % b)?);
            }
            Instruction::Call(fn_idx, arg_count) => {
                let chunk = &self.chunks[fn_idx as usize];
                // Pop args off the stack (they were pushed left-to-right).
                let stack_len = self.stack.len();
                let args = self.stack.split_off(stack_len - arg_count as usize);
                let new_frame = Frame::new(chunk, args);
                self.frames.push(new_frame);
                // Execution continues inside the new frame on the next iteration.
            }
            Instruction::CallBuiltin(builtin, argc) => {
                // HOF builtins need access to the frame machinery,
                // so they are handled here rather than in call_builtin.
                match builtin {
                    BuiltinFn::MapArray => {
                        // Stack: [..., array, fn_val]
                        let fn_val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                        let arr_val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                        let (chunk_index, _arity) = match fn_val {
                            Value::Function { chunk_index, arity } => (chunk_index, arity),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let elements = match arr_val {
                            Value::Array(a) => a.borrow().clone(),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let mut results = Vec::new();
                        for elem in elements {
                            let result = self.call_function(chunk_index, vec![elem])?;
                            results.push(result);
                        }
                        self.stack
                            .push(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(
                                results,
                            ))));
                    }
                    BuiltinFn::FilterArray => {
                        // Stack: [..., array, fn_val]
                        let fn_val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                        let arr_val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                        let (chunk_index, _arity) = match fn_val {
                            Value::Function { chunk_index, arity } => (chunk_index, arity),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let elements = match arr_val {
                            Value::Array(a) => a.borrow().clone(),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let mut results = Vec::new();
                        for elem in elements {
                            let result = self.call_function(chunk_index, vec![elem.clone()])?;
                            match result {
                                Value::Bool(true) => results.push(elem),
                                Value::Bool(false) => {}
                                _ => return Err(RuntimeError::TypeMismatch),
                            }
                        }
                        self.stack
                            .push(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(
                                results,
                            ))));
                    }
                    BuiltinFn::FoldArray => {
                        // Stack: [..., array, init, fn_val]
                        let fn_val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                        let init = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                        let arr_val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                        let (chunk_index, _arity) = match fn_val {
                            Value::Function { chunk_index, arity } => (chunk_index, arity),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let elements = match arr_val {
                            Value::Array(a) => a.borrow().clone(),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let mut acc = init;
                        for elem in elements {
                            acc = self.call_function(chunk_index, vec![acc, elem])?;
                        }
                        self.stack.push(acc);
                    }
                    _ => {
                        let stack_len = self.stack.len();
                        let mut args = self.stack.split_off(stack_len - argc as usize);
                        let result = call_builtin(builtin, &mut args)?;
                        self.stack.push(result);
                    }
                }
            }
            Instruction::Equal => {
                let (l, r) = self.pop2()?;
                self.stack.push(Value::Bool(l == r));
            }
            Instruction::LessThan => {
                let (l, r) = self.pop2()?;
                self.stack.push(Value::Bool(cmp_lt(l, r)?));
            }
            Instruction::GreaterThan => {
                let (l, r) = self.pop2()?;
                self.stack.push(Value::Bool(cmp_gt(l, r)?));
            }
            Instruction::LessEqual => {
                let (l, r) = self.pop2()?;
                self.stack.push(Value::Bool(cmp_le(l, r)?));
            }
            Instruction::GreaterEqual => {
                let (l, r) = self.pop2()?;
                self.stack.push(Value::Bool(cmp_ge(l, r)?));
            }
            Instruction::NotEqual => {
                let (l, r) = self.pop2()?;
                self.stack.push(Value::Bool(l != r));
            }
            Instruction::Negate => {
                let val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let result = match val {
                    Value::Int(n) => {
                        Value::Int(n.checked_neg().ok_or(RuntimeError::IntegerOverflow)?)
                    }
                    Value::Float(f) => Value::Float(-f),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                self.stack.push(result);
            }
            Instruction::Not => {
                let val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                match val {
                    Value::Bool(b) => self.stack.push(Value::Bool(!b)),
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
            Instruction::JumpIfFalse(offset) => {
                let val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                match val {
                    Value::Bool(b) => {
                        if !b {
                            self.frames.last_mut().unwrap().ip = offset as usize;
                        }
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
            Instruction::JumpIfTrue(offset) => {
                let val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                match val {
                    Value::Bool(b) => {
                        if b {
                            self.frames.last_mut().unwrap().ip = offset as usize;
                        }
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
            Instruction::Jump(offset) => {
                self.frames.last_mut().unwrap().ip = offset as usize;
            }
            Instruction::Print => {
                let val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                println!("{}", display_value(&val));
            }
            Instruction::Return => {
                let return_val = self.stack.pop();
                self.frames.pop();
                if self.frames.is_empty() {
                    return Ok(StepResult::Halt(return_val));
                }
                // Push return value back onto the caller's stack.
                if let Some(val) = return_val {
                    self.stack.push(val);
                }
            }
            Instruction::MakeMap(n) => {
                let stack_len = self.stack.len();
                let pairs = self.stack.split_off(stack_len - (n as usize) * 2);
                let mut map = HashMap::new();
                let mut iter = pairs.into_iter();
                while let (Some(k), Some(v)) = (iter.next(), iter.next()) {
                    match k {
                        Value::Str(s) => {
                            map.insert(s, v);
                        }
                        _ => return Err(RuntimeError::TypeMismatch),
                    }
                }
                self.stack.push(Value::Map(Rc::new(RefCell::new(map))));
            }
            Instruction::MakeArray(n) => {
                let stack_len = self.stack.len();
                let elements = self.stack.split_off(stack_len - n as usize);
                self.stack
                    .push(Value::Array(Rc::new(RefCell::new(elements))));
            }
            Instruction::GetIndex => {
                let index_val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let collection = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                match collection {
                    Value::Array(arr) => {
                        let index = match index_val {
                            Value::Int(i) => i,
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let borrowed = arr.borrow();
                        if index < 0 || index as usize >= borrowed.len() {
                            return Err(RuntimeError::IndexOutOfBounds {
                                index,
                                len: borrowed.len(),
                            });
                        }
                        self.stack.push(borrowed[index as usize].clone());
                    }
                    Value::Map(map) => {
                        let key = match index_val {
                            Value::Str(s) => s,
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let borrowed = map.borrow();
                        let val = borrowed
                            .get(&key)
                            .ok_or_else(|| RuntimeError::KeyNotFound(key.clone()))?
                            .clone();
                        self.stack.push(val);
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
            Instruction::SetIndex => {
                let value = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let index_val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let collection = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                match collection {
                    Value::Array(arr) => {
                        let index = match index_val {
                            Value::Int(i) => i,
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let mut borrowed = arr.borrow_mut();
                        if index < 0 || index as usize >= borrowed.len() {
                            return Err(RuntimeError::IndexOutOfBounds {
                                index,
                                len: borrowed.len(),
                            });
                        }
                        borrowed[index as usize] = value;
                    }
                    Value::Map(map) => {
                        let key = match index_val {
                            Value::Str(s) => s,
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        map.borrow_mut().insert(key, value);
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
            Instruction::ArrayLen => {
                let arr = match self.stack.pop().ok_or(RuntimeError::StackUnderflow)? {
                    Value::Array(a) => a,
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                self.stack.push(Value::Int(arr.borrow().len() as i64));
            }
            Instruction::TryStart(catch_target, error_slot) => {
                self.try_stack.push(TryHandler {
                    catch_target: catch_target as usize,
                    error_slot,
                    stack_len: self.stack.len(),
                    frame_depth: self.frames.len(),
                });
            }
            Instruction::TryEnd => {
                self.try_stack.pop();
            }
            Instruction::MakeRecord(field_names) => {
                let stack_len = self.stack.len();
                let values = self.stack.split_off(stack_len - field_names.len());
                let map: std::collections::HashMap<String, Value> =
                    field_names.into_iter().zip(values).collect();
                self.stack.push(Value::Record(Rc::new(RefCell::new(map))));
            }
            Instruction::GetField(name) => {
                let record = match self.stack.pop().ok_or(RuntimeError::StackUnderflow)? {
                    Value::Record(r) => r,
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                let borrowed = record.borrow();
                // Guaranteed present by the typechecker, so a missing key
                // here would indicate a compiler/typechecker bug, not a
                // user-reachable runtime error.
                let val = borrowed
                    .get(&name)
                    .expect("field presence guaranteed by typechecker")
                    .clone();
                self.stack.push(val);
            }
            Instruction::SetField(name) => {
                let value = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let record = match self.stack.pop().ok_or(RuntimeError::StackUnderflow)? {
                    Value::Record(r) => r,
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                record.borrow_mut().insert(name, value);
            }
            Instruction::MakeEnum(enum_name, variant, payload_count) => {
                let stack_len = self.stack.len();
                let payload = self.stack.split_off(stack_len - payload_count as usize);
                self.stack.push(Value::Enum {
                    enum_name,
                    variant,
                    payload,
                });
            }
            Instruction::TagEquals(variant) => {
                let instance = match self.stack.pop().ok_or(RuntimeError::StackUnderflow)? {
                    Value::Enum { variant: v, .. } => v,
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                self.stack.push(Value::Bool(instance == variant));
            }
            Instruction::GetPayload(index) => {
                let payload = match self.stack.pop().ok_or(RuntimeError::StackUnderflow)? {
                    Value::Enum { payload, .. } => payload,
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                // In-bounds is guaranteed by the typechecker (binder count
                // matches the matched variant's payload arity).
                self.stack.push(payload[index as usize].clone());
            }
            // push a function value onto the stack
            Instruction::LoadFn { chunk_index, arity } => {
                self.stack.push(Value::Function { chunk_index, arity });
            }
            // pop function value + args, push a new frame
            Instruction::CallValue(arg_count) => {
                let fn_val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                match fn_val {
                    Value::Function { chunk_index, arity } => {
                        if arg_count != arity {
                            return Err(RuntimeError::TypeMismatch);
                        }
                        let chunk = &self.chunks[chunk_index];
                        let stack_len = self.stack.len();
                        let args = self.stack.split_off(stack_len - arg_count as usize);
                        let new_frame = Frame::new(chunk, args);
                        self.frames.push(new_frame);
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
        }

        Ok(StepResult::Continue)
    }

    pub fn run(&mut self) -> Result<Option<Value>, RuntimeError> {
        loop {
            match self.step() {
                Ok(StepResult::Continue) => {}
                Ok(StepResult::Halt(v)) => return Ok(v),
                Err(e) => {
                    if let Some(handler) = self.try_stack.pop() {
                        // Truncate frames before the stack: truncating the
                        // stack first would panic if a now-discarded frame's
                        // saved arity assumptions still referenced it, and
                        // popping frames doesn't touch self.stack itself.
                        self.frames.truncate(handler.frame_depth);
                        self.stack.truncate(handler.stack_len);
                        let frame = self
                            .frames
                            .last_mut()
                            .expect("try handler's frame must still be on the stack");
                        frame.locals[handler.error_slot as usize] = Some(Value::Str(e.to_string()));
                        frame.ip = handler.catch_target;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    pub fn run_repl_line(
        &mut self,
        new_instrs: Vec<Instruction>,
    ) -> Result<Option<Value>, RuntimeError> {
        let start_ip = self.frames[0].instructions.len();
        self.frames[0].instructions.extend(new_instrs);
        self.frames[0].ip = start_ip;

        loop {
            if self.frames.len() == 1 && self.frames[0].ip >= self.frames[0].instructions.len() {
                return Ok(self.stack.pop());
            }
            match self.step() {
                Ok(StepResult::Continue) => {}
                Ok(StepResult::Halt(v)) => {
                    // A top-level 返す in REPL input ends frame 0 (matching
                    // ordinary script semantics) — restart a fresh, empty
                    // frame 0 so the session can keep going, at the cost of
                    // losing this session's variable bindings.
                    if self.frames.is_empty() {
                        self.frames.push(Frame {
                            instructions: Vec::new(),
                            ip: 0,
                            locals: vec![None; 256],
                        });
                    }
                    return Ok(v);
                }
                Err(e) => {
                    if let Some(handler) = self.try_stack.pop() {
                        self.frames.truncate(handler.frame_depth);
                        self.stack.truncate(handler.stack_len);
                        let frame = self
                            .frames
                            .last_mut()
                            .expect("try handler's frame must still be on the stack");
                        frame.locals[handler.error_slot as usize] = Some(Value::Str(e.to_string()));
                        frame.ip = handler.catch_target;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    pub fn sync_program(&mut self, constants: Vec<Value>, chunks: Vec<Chunk>) {
        self.constants = constants;
        self.chunks = chunks;
    }

    fn pop2(&mut self) -> Result<(Value, Value), RuntimeError> {
        let rhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
        let lhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
        Ok((lhs, rhs))
    }

    /// call a chunk by index with the given arguments and run it to
    /// completion, returning the produced value. Used by HOF builtins.
    fn call_function(
        &mut self,
        chunk_index: usize,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let chunk = &self.chunks[chunk_index];
        let frame = Frame::new(chunk, args);
        let target_depth = self.frames.len(); // depth BEFORE pushing the new frame
        self.frames.push(frame);
        // Run until we pop back to target_depth.
        loop {
            if self.frames.len() == target_depth {
                // The called frame finished; its return value (if any) is on
                // the stack. Pop and return it.
                return self.stack.pop().ok_or(RuntimeError::StackUnderflow);
            }
            match self.step()? {
                StepResult::Continue => {}
                StepResult::Halt(Some(v)) => return Ok(v),
                StepResult::Halt(None) => return Err(RuntimeError::StackUnderflow),
            }
        }
    }
}

pub fn display_value(val: &Value) -> String {
    match val {
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Str(s) => s.clone(),
        Value::Bool(b) => if *b { "真" } else { "偽" }.to_string(),
        Value::Array(arr) => format!(
            "【{}】",
            arr.borrow()
                .iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join("、")
        ),
        Value::Record(rec) => {
            let borrowed = rec.borrow();
            // HashMap iteration order is unspecified; sort keys for
            // deterministic display/test output rather than building
            // ordered-map machinery just for this.
            let mut keys: Vec<&String> = borrowed.keys().collect();
            keys.sort();
            format!(
                "｛{}｝",
                keys.iter()
                    .map(|k| format!("{}：{}", k, display_value(&borrowed[*k])))
                    .collect::<Vec<_>>()
                    .join("、")
            )
        }
        Value::Map(map) => {
            let borrowed = map.borrow();
            let mut keys: Vec<&String> = borrowed.keys().collect();
            keys.sort();
            if keys.is_empty() {
                "｛｝".to_string()
            } else {
                format!(
                    "｛{}｝",
                    keys.iter()
                        .map(|k| format!("「{}」：{}", k, display_value(&borrowed[*k])))
                        .collect::<Vec<_>>()
                        .join("、")
                )
            }
        }
        Value::Enum {
            variant, payload, ..
        } => {
            if payload.is_empty() {
                variant.clone()
            } else {
                format!(
                    "{}（{}）",
                    variant,
                    payload
                        .iter()
                        .map(display_value)
                        .collect::<Vec<_>>()
                        .join("、")
                )
            }
        }
        Value::Function { chunk_index, arity } => {
            format!("関数＜チャンク{}、引数{}＞", chunk_index, arity)
        }
    }
}

fn cmp_lt(lhs: Value, rhs: Value) -> Result<bool, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(a < b),
        (Value::Float(a), Value::Float(b)) => Ok(a < b),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

fn cmp_gt(lhs: Value, rhs: Value) -> Result<bool, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(a > b),
        (Value::Float(a), Value::Float(b)) => Ok(a > b),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

fn cmp_le(lhs: Value, rhs: Value) -> Result<bool, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(a <= b),
        (Value::Float(a), Value::Float(b)) => Ok(a <= b),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

fn cmp_ge(lhs: Value, rhs: Value) -> Result<bool, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(a >= b),
        (Value::Float(a), Value::Float(b)) => Ok(a >= b),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

// 整数化/小数化 accept strings users naturally type with full-width digits
// (e.g. from 入力), so normalize before handing off to Rust's ASCII parser.
fn normalize_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{FF10}'..='\u{FF19}' => char::from_u32(c as u32 - 0xFF10 + '0' as u32).unwrap_or(c),
            '．' => '.',
            'ー' => '-',
            _ => c,
        })
        .collect()
}

fn call_builtin(builtin: BuiltinFn, args: &mut Vec<Value>) -> Result<Value, RuntimeError> {
    match builtin {
        BuiltinFn::Len => match args.pop() {
            Some(Value::Str(s)) => Ok(Value::Int(s.chars().count() as i64)),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Input => {
            let mut line = String::new();
            std::io::stdin()
                .read_line(&mut line)
                .map_err(|e| RuntimeError::InvalidConversion(e.to_string()))?;
            let trimmed = line.trim_end_matches(['\n', '\r']);
            Ok(Value::Str(trimmed.to_string()))
        }
        BuiltinFn::ParseInt => match args.pop() {
            Some(Value::Str(s)) => {
                normalize_digits(&s)
                    .parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| {
                        RuntimeError::InvalidConversion(format!(
                            "「{}」は整数に変換できません。",
                            s
                        ))
                    })
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::ParseFloat => match args.pop() {
            Some(Value::Str(s)) => normalize_digits(&s)
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| {
                    RuntimeError::InvalidConversion(format!("「{}」は小数に変換できません。", s))
                }),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::ToStr => match args.pop() {
            Some(val) => Ok(Value::Str(display_value(&val))),
            None => Err(RuntimeError::StackUnderflow),
        },
        BuiltinFn::Abs => match args.pop() {
            Some(Value::Int(n)) => Ok(Value::Int(n.wrapping_abs())),
            Some(Value::Float(f)) => Ok(Value::Float(f.abs())),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Sqrt => match args.pop() {
            Some(Value::Int(n)) => {
                if n < 0 {
                    Err(RuntimeError::InvalidConversion(
                        "負の数の平方根は計算できません。".to_string(),
                    ))
                } else {
                    Ok(Value::Float((n as f64).sqrt()))
                }
            }
            Some(Value::Float(f)) => {
                if f < 0.0 {
                    Err(RuntimeError::InvalidConversion(
                        "負の数の平方根は計算できません。".to_string(),
                    ))
                } else {
                    Ok(Value::Float(f.sqrt()))
                }
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Random => {
            let (min, max) = match (args.first().cloned(), args.get(1).cloned()) {
                (Some(Value::Int(min)), Some(Value::Int(max))) => (min, max),
                _ => return Err(RuntimeError::TypeMismatch),
            };
            if min > max {
                return Err(RuntimeError::InvalidConversion(
                    "乱数の範囲が無効です（最小値が最大値より大きいです）。".to_string(),
                ));
            }
            Ok(Value::Int(next_random_i64(min, max)))
        }
        BuiltinFn::Max => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Int(a)), Some(Value::Int(b))) => Ok(Value::Int(a.max(b))),
            (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.max(b))),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Min => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Int(a)), Some(Value::Int(b))) => Ok(Value::Int(a.min(b))),
            (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.min(b))),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Split => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Str(s)), Some(Value::Str(sep))) => {
                let parts: Vec<Value> = s
                    .split(sep.as_str())
                    .map(|p| Value::Str(p.to_string()))
                    .collect();
                Ok(Value::Array(Rc::new(RefCell::new(parts))))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Join => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Array(arr)), Some(Value::Str(sep))) => {
                let joined = arr
                    .borrow()
                    .iter()
                    .map(display_value)
                    .collect::<Vec<_>>()
                    .join(&sep);
                Ok(Value::Str(joined))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Contains => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Str(s)), Some(Value::Str(needle))) => Ok(Value::Bool(s.contains(&needle))),
            (Some(Value::Map(m)), Some(Value::Str(key))) => {
                Ok(Value::Bool(m.borrow().contains_key(&key)))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Replace => {
            match (
                args.first().cloned(),
                args.get(1).cloned(),
                args.get(2).cloned(),
            ) {
                (Some(Value::Str(s)), Some(Value::Str(old)), Some(Value::Str(new))) => {
                    Ok(Value::Str(s.replace(&old, &new)))
                }
                _ => Err(RuntimeError::TypeMismatch),
            }
        }
        BuiltinFn::Pow => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Int(base)), Some(Value::Int(exp))) => {
                // Negative exponents have no integer result; checked_pow also
                // needs a u32, so both bounds are validated up front.
                let exp_u32 = u32::try_from(exp).map_err(|_| {
                    RuntimeError::InvalidConversion(
                        "整数の累乗では指数は０以上である必要があります。".to_string(),
                    )
                })?;
                base.checked_pow(exp_u32)
                    .map(Value::Int)
                    .ok_or(RuntimeError::IntegerOverflow)
            }
            (Some(Value::Float(base)), Some(Value::Float(exp))) => Ok(Value::Float(base.powf(exp))),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Floor => match args.pop() {
            Some(Value::Float(f)) => float_to_int(f.floor()),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Ceil => match args.pop() {
            Some(Value::Float(f)) => float_to_int(f.ceil()),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Round => match args.pop() {
            Some(Value::Float(f)) => float_to_int(f.round()),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Rem => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Int(_)), Some(Value::Int(0))) => Err(RuntimeError::DivisionByZero),
            (Some(Value::Int(a)), Some(Value::Int(b))) => Ok(Value::Int(a % b)),
            (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a % b)),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::ArrayLen => match args.pop() {
            Some(Value::Array(a)) => Ok(Value::Int(a.borrow().len() as i64)),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Push => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Array(a)), Some(val)) => {
                a.borrow_mut().push(val);
                // 追加 is 無-typed; there's no Value::Void, so CallBuiltin
                // (which always pushes one result) gets a placeholder that's
                // never observed since the typechecker forbids using a 無
                // call's result as a value.
                Ok(Value::Int(0))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Pop => match args.pop() {
            Some(Value::Array(a)) => a.borrow_mut().pop().ok_or(RuntimeError::EmptyArray),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::ArrayContains => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Array(a)), Some(val)) => Ok(Value::Bool(a.borrow().contains(&val))),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::IndexOf => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Array(a)), Some(val)) => {
                let pos = a.borrow().iter().position(|v| v == &val);
                Ok(Value::Int(pos.map(|p| p as i64).unwrap_or(-1)))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Reverse => match args.pop() {
            Some(Value::Array(a)) => {
                a.borrow_mut().reverse();
                Ok(Value::Array(a))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Sort => match args.pop() {
            Some(Value::Array(a)) => {
                {
                    let mut borrowed = a.borrow_mut();
                    sort_values(&mut borrowed)?;
                }
                Ok(Value::Array(a))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        // Unlike the other array builtins, 部分列 returns a NEW array rather
        // than mutating the original — slicing-as-copy matches the common
        // convention in other languages, even though 逆順/整列/追加/取り出す
        // here all mutate in place.
        BuiltinFn::Slice => {
            match (
                args.first().cloned(),
                args.get(1).cloned(),
                args.get(2).cloned(),
            ) {
                (Some(Value::Array(a)), Some(Value::Int(start)), Some(Value::Int(end))) => {
                    let borrowed = a.borrow();
                    let len = borrowed.len();
                    if start < 0 || end < 0 || start > end || end as usize > len {
                        return Err(RuntimeError::IndexOutOfBounds {
                            index: if start < 0 || start as usize > len {
                                start
                            } else {
                                end
                            },
                            len,
                        });
                    }
                    let slice = borrowed[start as usize..end as usize].to_vec();
                    Ok(Value::Array(Rc::new(RefCell::new(slice))))
                }
                _ => Err(RuntimeError::TypeMismatch),
            }
        }
        BuiltinFn::MapKeys => match args.pop() {
            Some(Value::Map(m)) => {
                let borrowed = m.borrow();
                let mut keys: Vec<Value> = borrowed.keys().map(|k| Value::Str(k.clone())).collect();
                // Sort for deterministic ordering.
                keys.sort_by(|a, b| match (a, b) {
                    (Value::Str(x), Value::Str(y)) => x.cmp(y),
                    _ => std::cmp::Ordering::Equal,
                });
                Ok(Value::Array(Rc::new(RefCell::new(keys))))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::MapValues => match args.pop() {
            Some(Value::Map(m)) => {
                let borrowed = m.borrow();
                // Sort by key for deterministic ordering.
                let mut entries: Vec<(&String, &Value)> = borrowed.iter().collect();
                entries.sort_by_key(|(k, _)| k.as_str());
                let vals: Vec<Value> = entries.into_iter().map(|(_, v)| v.clone()).collect();
                Ok(Value::Array(Rc::new(RefCell::new(vals))))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::MapDelete => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Map(m)), Some(Value::Str(key))) => {
                m.borrow_mut().remove(&key);
                // 削除 is 無-typed; return a placeholder like Push does.
                Ok(Value::Int(0))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        // HOF builtins are handled directly in step() since they
        // need access to the frame machinery; they never reach call_builtin.
        BuiltinFn::MapArray | BuiltinFn::FilterArray | BuiltinFn::FoldArray => {
            unreachable!("HOF builtins are handled in step()")
        }
    }
}

fn float_to_int(f: f64) -> Result<Value, RuntimeError> {
    if f.is_finite() && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
        Ok(Value::Int(f as i64))
    } else {
        Err(RuntimeError::InvalidConversion(
            "結果が整数の範囲に収まりません。".to_string(),
        ))
    }
}

fn sort_values(values: &mut [Value]) -> Result<(), RuntimeError> {
    if values
        .iter()
        .any(|v| !matches!(v, Value::Int(_) | Value::Float(_) | Value::Str(_)))
    {
        return Err(RuntimeError::TypeMismatch);
    }
    values.sort_by(|a, b| match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Str(x), Value::Str(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    Ok(())
}

// Each call combines the current time with a process-local counter so two
// 乱数 calls within the same nanosecond still get distinct seeds.
static RANDOM_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_random_i64(min: i64, max: i64) -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let counter = RANDOM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut seed = nanos ^ counter;
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    // Compute the span in i128 so a wide [min, max] range can't overflow i64.
    let span = (max as i128) - (min as i128) + 1;
    let offset = (seed as u128 % span as u128) as i64;
    min + offset
}

// Integer ops use checked arithmetic so an overflow surfaces as a catchable
// RuntimeError instead of panicking the interpreter.
fn arith(
    lhs: Value,
    rhs: Value,
    int_op: impl Fn(i64, i64) -> Option<i64>,
    float_op: impl Fn(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => int_op(a, b)
            .map(Value::Int)
            .ok_or(RuntimeError::IntegerOverflow),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(a, b))),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{Compiler, Instruction, Value};
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn run(src: &str) -> Option<Value> {
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        Vm::with_chunks(compiler.constants, compiler.chunks, script)
            .run()
            .unwrap()
    }

    #[test]
    fn test_vm_push_constant() {
        let constants = vec![Value::Int(42)];
        let instructions = vec![Instruction::LoadConst(0), Instruction::Return];
        let result = Vm::new(constants, instructions).run().unwrap();
        assert_eq!(result, Some(Value::Int(42)));
    }

    #[test]
    fn test_vm_store_and_load_local() {
        let result = run("整数 年齢 ＝ ２０；返す 年齢；");
        assert_eq!(result, Some(Value::Int(20)));
    }

    #[test]
    fn test_vm_addition() {
        let result = run("整数 結果 ＝ ３ ＋ ４；返す 結果；");
        assert_eq!(result, Some(Value::Int(7)));
    }

    #[test]
    fn test_vm_operator_precedence() {
        let result = run("整数 結果 ＝ ２ ＋ ３ ＊ ４；返す 結果；");
        assert_eq!(result, Some(Value::Int(14)));
    }

    #[test]
    fn test_vm_function_body_via_call() {
        // 関数 加算一（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
        // 返す 加算一（９）；  →  10
        let src = "関数 加算一（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝返す 加算一（９）；";
        assert_eq!(run(src), Some(Value::Int(10)));
    }

    #[test]
    fn test_vm_call_function() {
        // 関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝
        // 返す 二倍（５）；  →  10
        let src = "関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝返す 二倍（５）；";
        assert_eq!(run(src), Some(Value::Int(10)));
    }

    #[test]
    fn test_vm_print_integer() {
        // 印刷（４２）；  — should not panic and produces no return value
        let result = run("印刷（４２）；");
        assert_eq!(result, None);
    }

    #[test]
    fn test_vm_print_variable() {
        // 整数 Ａ ＝ ７；  印刷（Ａ）；
        let result = run("整数 Ａ ＝ ７；印刷（Ａ）；");
        assert_eq!(result, None);
    }

    #[test]
    fn test_vm_print_then_return() {
        // 印刷（１）；  返す ２；  — prints 1, returns 2
        let result = run("印刷（１）；返す ２；");
        assert_eq!(result, Some(Value::Int(2)));
    }

    #[test]
    fn test_vm_bool_literal_as_if_condition() {
        // 真偽 フラグ ＝ 真；もし フラグ ならば ｛ 返す １； ｝ 違えば ｛ 返す ０； ｝
        let result =
            run("真偽 フラグ ＝ 真；もし フラグ ならば ｛ 返す １； ｝ 違えば ｛ 返す ０； ｝");
        assert_eq!(result, Some(Value::Int(1)));
    }

    #[test]
    fn test_vm_if_true_branch() {
        // もし １ ＝＝ １ ならば ｛ 返す １０； ｝
        // Condition is true → returns 10
        let result = run("もし １ ＝＝ １ ならば ｛ 返す １０； ｝");
        assert_eq!(result, Some(Value::Int(10)));
    }

    #[test]
    fn test_vm_if_false_branch_skipped() {
        // もし １ ＝＝ ２ ならば ｛ 返す １０； ｝ 返す ０；
        // Condition is false → skips then_body, returns 0
        let result = run("もし １ ＝＝ ２ ならば ｛ 返す １０； ｝返す ０；");
        assert_eq!(result, Some(Value::Int(0)));
    }

    #[test]
    fn test_vm_if_else() {
        // もし １ ＝＝ ２ ならば ｛ 返す １； ｝ 違えば ｛ 返す ２； ｝
        // Condition false → else branch → returns 2
        let result = run("もし １ ＝＝ ２ ならば ｛ 返す １； ｝ 違えば ｛ 返す ２； ｝");
        assert_eq!(result, Some(Value::Int(2)));
    }

    #[test]
    fn test_vm_comparison_lt_gt() {
        // 整数 Ａ ＝ ３；  もし Ａ ＜ ５ ならば ｛ 返す １； ｝ 違えば ｛ 返す ０； ｝
        let result =
            run("整数 Ａ ＝ ３；もし Ａ ＜ ５ ならば ｛ 返す １； ｝ 違えば ｛ 返す ０； ｝");
        assert_eq!(result, Some(Value::Int(1)));
    }

    #[test]
    fn test_vm_call_with_expression_arg() {
        // 関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝
        // 返す 二倍（３ ＋ ４）；  →  14
        let src = "関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝返す 二倍（３ ＋ ４）；";
        assert_eq!(run(src), Some(Value::Int(14)));
    }

    #[test]
    fn test_vm_division_by_zero_returns_error() {
        // 整数 結果 ＝ １ ／ ０；
        let ast = Parser::new(Lexer::new("整数 結果 ＝ １ ／ ０；").tokenize())
            .parse()
            .unwrap();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
        assert_eq!(result, Err(RuntimeError::DivisionByZero));
    }

    #[test]
    fn test_vm_float_division_by_zero_returns_error() {
        let ast = Parser::new(Lexer::new("小数 結果 ＝ １．０ ／ ０．０；").tokenize())
            .parse()
            .unwrap();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
        assert_eq!(result, Err(RuntimeError::DivisionByZero));
    }

    #[test]
    fn test_vm_reassignment() {
        // 整数 年齢 ＝ ２０；年齢 ＝ ３０；返す 年齢；
        let result = run("整数 年齢 ＝ ２０；年齢 ＝ ３０；返す 年齢；");
        assert_eq!(result, Some(Value::Int(30)));
    }

    #[test]
    fn test_vm_multi_param_call() {
        // 関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝
        // 返す 加算（３、４）；  →  7
        let src =
            "関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝返す 加算（３、４）；";
        assert_eq!(run(src), Some(Value::Int(7)));
    }

    #[test]
    fn test_vm_unary_minus() {
        let result = run("整数 結果 ＝ ー５；返す 結果；");
        assert_eq!(result, Some(Value::Int(-5)));
    }

    #[test]
    fn test_vm_unary_minus_in_expression() {
        let result = run("整数 結果 ＝ １０ ＋ ー３；返す 結果；");
        assert_eq!(result, Some(Value::Int(7)));
    }

    #[test]
    fn test_vm_logical_and() {
        assert_eq!(run("返す 真 かつ 偽；"), Some(Value::Bool(false)));
        assert_eq!(run("返す 真 かつ 真；"), Some(Value::Bool(true)));
    }

    #[test]
    fn test_vm_logical_or() {
        assert_eq!(run("返す 真 または 偽；"), Some(Value::Bool(true)));
        assert_eq!(run("返す 偽 または 偽；"), Some(Value::Bool(false)));
    }

    #[test]
    fn test_vm_logical_not() {
        assert_eq!(run("返す 否定 真；"), Some(Value::Bool(false)));
        assert_eq!(run("返す 否定 偽；"), Some(Value::Bool(true)));
    }

    #[test]
    fn test_vm_additional_comparison_operators() {
        assert_eq!(run("返す ３ ≦ ３；"), Some(Value::Bool(true)));
        assert_eq!(run("返す ５ ≧ １０；"), Some(Value::Bool(false)));
        assert_eq!(run("返す １ ≠ ２；"), Some(Value::Bool(true)));
    }

    #[test]
    fn test_vm_string_concatenation() {
        let result = run("文字列 結果 ＝ 「あ」 ＋ 「い」；返す 結果；");
        assert_eq!(result, Some(Value::Str("あい".to_string())));
    }

    #[test]
    fn test_vm_builtin_strlen() {
        let result = run("返す 文字数（「こんにちは」）；");
        assert_eq!(result, Some(Value::Int(5)));
    }

    #[test]
    fn test_vm_builtin_parse_int() {
        let result = run("返す 整数化（「４２」）；");
        assert_eq!(result, Some(Value::Int(42)));
    }

    #[test]
    fn test_vm_builtin_parse_float() {
        let result = run("返す 小数化（「３．５」）；");
        assert_eq!(result, Some(Value::Float(3.5)));
    }

    #[test]
    fn test_vm_builtin_to_str_int() {
        let result = run("返す 文字列化（４２）；");
        assert_eq!(result, Some(Value::Str("42".to_string())));
    }

    #[test]
    fn test_vm_builtin_to_str_float() {
        let result = run("返す 文字列化（３．５）；");
        assert_eq!(result, Some(Value::Str("3.5".to_string())));
    }

    #[test]
    fn test_vm_builtin_to_str_bool() {
        let result = run("返す 文字列化（真）；");
        assert_eq!(result, Some(Value::Str("真".to_string())));
    }

    #[test]
    fn test_vm_builtin_parse_int_invalid_returns_error() {
        let ast = Parser::new(Lexer::new("返す 整数化（「abc」）；").tokenize())
            .parse()
            .unwrap();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
        assert!(matches!(result, Err(RuntimeError::InvalidConversion(_))));
    }

    #[test]
    fn test_vm_uninitialized_local_returns_error() {
        // A hand-built program that reads local slot 0 without ever storing
        // into it first; this can't be produced by the compiler from valid
        // Hikari source (every declared variable is stored immediately), so
        // the instruction stream is constructed directly to exercise the
        // VM's own guard against reading an uninitialized local.
        let instructions = vec![Instruction::LoadLocal(0), Instruction::Return];
        let result = Vm::new(vec![], instructions).run();
        assert_eq!(result, Err(RuntimeError::UninitializedLocal(0)));
    }

    #[test]
    fn test_vm_array_literal_and_index() {
        let result = run("整数列 数字 ＝ 【１、２、３】；返す 数字【１】；");
        assert_eq!(result, Some(Value::Int(2)));
    }

    #[test]
    fn test_vm_index_assign_mutates_array() {
        let result = run("整数列 数字 ＝ 【１、２、３】；数字【０】＝ ９；返す 数字【０】；");
        assert_eq!(result, Some(Value::Int(9)));
    }

    #[test]
    fn test_vm_array_aliasing_reference_semantics() {
        // Assigning Ａ to Ｂ shares the same underlying Rc<RefCell<>>, so
        // mutating through Ｂ must be visible through Ａ.
        let src = "整数列 Ａ ＝ 【１、２、３】；整数列 Ｂ ＝ Ａ；Ｂ【０】＝ ９９；返す Ａ【０】；";
        let result = run(src);
        assert_eq!(result, Some(Value::Int(99)));
    }

    #[test]
    fn test_vm_index_out_of_bounds_returns_error() {
        let ast =
            Parser::new(Lexer::new("整数列 数字 ＝ 【１、２】；返す 数字【５】；").tokenize())
                .parse()
                .unwrap();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
        assert_eq!(
            result,
            Err(RuntimeError::IndexOutOfBounds { index: 5, len: 2 })
        );
    }

    #[test]
    fn test_vm_for_range_sums_to_ten() {
        // 繰り返す カウンタ ＝ ０ から ５ ならば ｛ 合計 ＝ 合計 ＋ カウンタ； ｝
        let src = "整数 合計 ＝ ０；繰り返す カウンタ ＝ ０ から ５ ならば ｛ 合計 ＝ 合計 ＋ カウンタ； ｝返す 合計；";
        let result = run(src);
        assert_eq!(result, Some(Value::Int(1 + 2 + 3 + 4))); // 0..5, excludes 5
    }

    #[test]
    fn test_vm_for_each_sums_array_elements() {
        let src = "整数列 数字 ＝ 【１、２、３】；整数 合計 ＝ ０；各 要素 ： 数字 ならば ｛ 合計 ＝ 合計 ＋ 要素； ｝返す 合計；";
        let result = run(src);
        assert_eq!(result, Some(Value::Int(6)));
    }

    #[test]
    fn test_vm_nested_for_each_loops_no_slot_collision() {
        let src = "整数列 Ａ ＝ 【１、２】；整数列 Ｂ ＝ 【１０、２０、３０】；整数 合計 ＝ ０；各 外側 ： Ａ ならば ｛ 各 内側 ： Ｂ ならば ｛ 合計 ＝ 合計 ＋ 内側； ｝ ｝返す 合計；";
        let result = run(src);
        // Outer loop runs twice; inner sum (10+20+30=60) accumulates each time.
        assert_eq!(result, Some(Value::Int(120)));
    }

    #[test]
    fn test_vm_sequential_for_each_loops_no_slot_collision() {
        let src = "整数列 Ａ ＝ 【１、２】；整数列 Ｂ ＝ 【１０、２０】；整数 合計 ＝ ０；各 要素 ： Ａ ならば ｛ 合計 ＝ 合計 ＋ 要素； ｝各 要素 ： Ｂ ならば ｛ 合計 ＝ 合計 ＋ 要素； ｝返す 合計；";
        let result = run(src);
        assert_eq!(result, Some(Value::Int(33)));
    }

    #[test]
    fn test_vm_print_array() {
        let result = run("印刷（【１、２、３】）；");
        assert_eq!(result, None);
    }

    #[test]
    fn test_vm_if_body_var_does_not_shadow_outer_slot() {
        let src = "整数 Ｎ ＝ １０；もし 真 ならば ｛ 整数 Ｎ ＝ ５； ｝返す Ｎ；";
        let result = run(src);
        assert_eq!(result, Some(Value::Int(10)));
    }

    #[test]
    fn test_vm_while_body_var_does_not_leak_into_outer_slot() {
        let src = "整数 Ｎ ＝ １０；整数 カウンタ ＝ ０；間 カウンタ ＜ ３ ならば ｛ 整数 Ｎ ＝ ９９；カウンタ ＝ カウンタ ＋ １； ｝返す Ｎ；";
        let result = run(src);
        assert_eq!(result, Some(Value::Int(10)));
    }

    #[test]
    fn test_vm_try_catch_success_skips_catch_body() {
        let src =
            "整数 結果 ＝ ０；試す ｛ 結果 ＝ １； ｝ 失敗 失敗内容 ｛ 結果 ＝ ２； ｝返す 結果；";
        let result = run(src);
        assert_eq!(result, Some(Value::Int(1)));
    }

    #[test]
    fn test_vm_try_catch_catches_division_by_zero_and_continues() {
        let src = "整数 結果 ＝ ０；試す ｛ 結果 ＝ １ ／ ０； ｝ 失敗 失敗内容 ｛ 結果 ＝ ９； ｝返す 結果；";
        let result = run(src);
        assert_eq!(result, Some(Value::Int(9)));
    }

    #[test]
    fn test_vm_try_catch_binds_error_message() {
        let src = "整数 結果 ＝ ０；試す ｛ 結果 ＝ １ ／ ０； ｝ 失敗 失敗内容 ｛ 結果 ＝ 文字数（失敗内容）； ｝返す 結果；";
        let result = run(src);
        match result {
            Some(Value::Int(n)) => assert!(n > 0),
            other => panic!("expected a non-empty error message length, got {:?}", other),
        }
    }

    #[test]
    fn test_vm_try_catch_unwinds_nested_function_call() {
        // Error occurs inside a function call made from within try_body, so
        // unwinding must pop the callee's Frame, not just truncate the stack.
        let src = "関数 割る（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ／ Ｂ； ｝整数 結果 ＝ ０；試す ｛ 結果 ＝ 割る（１０、０）； ｝ 失敗 失敗内容 ｛ 結果 ＝ ７； ｝整数 後 ＝ 割る（２０、４）；返す 結果 ＋ 後；";
        let result = run(src);
        // Catch sets 結果＝7; a fresh, unrelated call to 割る after the
        // try/catch must still work correctly, proving frames/stack were
        // left in a valid, non-corrupted state.
        assert_eq!(result, Some(Value::Int(7 + 5)));
    }

    #[test]
    fn test_vm_uncaught_error_still_propagates() {
        let ast = Parser::new(Lexer::new("整数 結果 ＝ １ ／ ０；").tokenize())
            .parse()
            .unwrap();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
        assert_eq!(result, Err(RuntimeError::DivisionByZero));
    }

    #[test]
    fn test_vm_nested_try_catch_inner_handler_catches_own_error() {
        // Inner 試す catches its own division-by-zero; the outer handler is
        // never triggered since the inner one already handled it.
        let src = "整数 結果 ＝ ０；試す ｛ 試す ｛ 結果 ＝ １ ／ ０； ｝ 失敗 失敗内容 ｛ 結果 ＝ １； ｝ ｝ 失敗 失敗内容 ｛ 結果 ＝ ２； ｝返す 結果；";
        let result = run(src);
        assert_eq!(result, Some(Value::Int(1)));
    }

    #[test]
    fn test_vm_abs_int_and_float() {
        let result = run("取り込む 「数学」；返す 絶対値（ー５）；");
        assert_eq!(result, Some(Value::Int(5)));

        let result = run("取り込む 「数学」；返す 絶対値（ー５．５）；");
        assert_eq!(result, Some(Value::Float(5.5)));
    }

    #[test]
    fn test_vm_sqrt_of_perfect_square() {
        let result = run("取り込む 「数学」；返す 平方根（９）；");
        assert_eq!(result, Some(Value::Float(3.0)));
    }

    #[test]
    fn test_vm_sqrt_of_negative_returns_error() {
        let ast = Parser::new(
            Lexer::new("取り込む 「数学」；整数 結果 ＝ ー１；返す 平方根（結果）；").tokenize(),
        )
        .parse()
        .unwrap();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
        assert!(matches!(result, Err(RuntimeError::InvalidConversion(_))));
    }

    #[test]
    fn test_vm_random_within_bounds() {
        for _ in 0..200 {
            let result = run("取り込む 「数学」；返す 乱数（５、１０）；");
            match result {
                Some(Value::Int(n)) => assert!((5..=10).contains(&n)),
                other => panic!("expected Int, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_vm_random_invalid_range_returns_error() {
        let ast = Parser::new(Lexer::new("取り込む 「数学」；返す 乱数（１０、５）；").tokenize())
            .parse()
            .unwrap();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
        assert!(matches!(result, Err(RuntimeError::InvalidConversion(_))));
    }

    #[test]
    fn test_vm_max_min_happy_path() {
        let result = run("取り込む 「数学」；返す 最大（３、７）；");
        assert_eq!(result, Some(Value::Int(7)));

        let result = run("取り込む 「数学」；返す 最小（３、７）；");
        assert_eq!(result, Some(Value::Int(3)));
    }

    #[test]
    fn test_vm_split_and_join_round_trip() {
        let result = run(
            "取り込む 「文字列」；文字列列 部分 ＝ 分割（「あ、い、う」、「、」）；返す 結合（部分、「、」）；",
        );
        assert_eq!(result, Some(Value::Str("あ、い、う".to_string())));
    }

    #[test]
    fn test_vm_contains_true_and_false() {
        let result = run("取り込む 「文字列」；返す 含む（「あいう」、「い」）；");
        assert_eq!(result, Some(Value::Bool(true)));

        let result = run("取り込む 「文字列」；返す 含む（「あいう」、「え」）；");
        assert_eq!(result, Some(Value::Bool(false)));
    }

    #[test]
    fn test_vm_repl_persists_locals_across_lines() {
        let mut compiler = Compiler::new();
        let mut vm = Vm::with_chunks(Vec::new(), Vec::new(), Vec::new());

        let ast1 = Parser::new(Lexer::new("整数 値 ＝ １０；").tokenize())
            .parse()
            .unwrap();
        let instrs1 = compiler.compile(&ast1);
        vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
        let result1 = vm.run_repl_line(instrs1).unwrap();
        assert_eq!(result1, None);

        let ast2 = Parser::new(Lexer::new("値；").tokenize()).parse().unwrap();
        let instrs2 = compiler.compile(&ast2);
        vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
        let result2 = vm.run_repl_line(instrs2).unwrap();
        assert_eq!(result2, Some(Value::Int(10)));
    }

    #[test]
    fn test_vm_repl_line_with_explicit_return_resets_frame_without_panicking() {
        let mut compiler = Compiler::new();
        let mut vm = Vm::with_chunks(Vec::new(), Vec::new(), Vec::new());

        let ast1 = Parser::new(Lexer::new("返す １；").tokenize())
            .parse()
            .unwrap();
        let instrs1 = compiler.compile(&ast1);
        vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
        let result1 = vm.run_repl_line(instrs1).unwrap();
        assert_eq!(result1, Some(Value::Int(1)));

        let ast2 = Parser::new(Lexer::new("印刷（２）；").tokenize())
            .parse()
            .unwrap();
        let instrs2 = compiler.compile(&ast2);
        vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
        let result2 = vm.run_repl_line(instrs2);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_vm_repl_line_bare_expression_surfaces_value() {
        let mut compiler = Compiler::new();
        let mut vm = Vm::with_chunks(Vec::new(), Vec::new(), Vec::new());

        let ast = Parser::new(Lexer::new("１ ＋ １；").tokenize())
            .parse()
            .unwrap();
        let instrs = compiler.compile(&ast);
        vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
        let result = vm.run_repl_line(instrs).unwrap();
        assert_eq!(result, Some(Value::Int(2)));
    }

    #[test]
    fn test_vm_replace_happy_path() {
        let result = run("取り込む 「文字列」；返す 置換（「あいう」、「い」、「え」）；");
        assert_eq!(result, Some(Value::Str("あえう".to_string())));
    }

    fn run_result(src: &str) -> Result<Option<Value>, RuntimeError> {
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        Vm::with_chunks(compiler.constants, compiler.chunks, script).run()
    }

    #[test]
    fn test_vm_void_function_call_does_not_halt_program() {
        // A 無-returning function falls off the end of its body without an
        // explicit 返す; the caller must resume rather than the whole program
        // terminating. 表示 prints its arg, then the script returns 99.
        let src = "関数 表示（整数 Ａ）ー＞ 無 ｛ 印刷（Ａ）； ｝表示（５）；返す ９９；";
        assert_eq!(run_result(src).unwrap(), Some(Value::Int(99)));
    }

    #[test]
    fn test_vm_integer_addition_overflow_is_runtime_error() {
        let src = "整数 Ｘ ＝ ９２２３３７２０３６８５４７７５８０７ ＋ １；返す Ｘ；";
        assert_eq!(run_result(src), Err(RuntimeError::IntegerOverflow));
    }

    #[test]
    fn test_vm_integer_multiplication_overflow_is_runtime_error() {
        let src = "整数 Ｘ ＝ ９２２３３７２０３６８５４７７５８０７ ＊ ２；返す Ｘ；";
        assert_eq!(run_result(src), Err(RuntimeError::IntegerOverflow));
    }

    // ── 7a: modulo ───────────────────────────────────────────────────────

    #[test]
    fn test_vm_modulo_int() {
        assert_eq!(run("返す １０ ％ ３；"), Some(Value::Int(1)));
    }

    #[test]
    fn test_vm_modulo_float() {
        assert_eq!(run("返す １０．０ ％ ３．０；"), Some(Value::Float(1.0)));
    }

    #[test]
    fn test_vm_modulo_by_zero_returns_error() {
        assert_eq!(
            run_result("返す １０ ％ ０；"),
            Err(RuntimeError::DivisionByZero)
        );
    }

    #[test]
    fn test_vm_fizzbuzz_with_modulo() {
        let src =
            "整数 Ｎ ＝ １５；もし Ｎ ％ １５ ＝＝ ０ ならば ｛ 返す ０； ｝違えば｛ 返す Ｎ； ｝";
        assert_eq!(run(src), Some(Value::Int(0)));
    }

    // ── 7b: array builtins ──────────────────────────────────────────────

    #[test]
    fn test_vm_array_len_builtin() {
        let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；返す 要素数（数字）；";
        assert_eq!(run(src), Some(Value::Int(3)));
    }

    #[test]
    fn test_vm_push_pop_round_trip() {
        let src = "取り込む 「配列」；整数列 数字 ＝ 新配列＜整数＞；追加（数字、１）；追加（数字、２）；整数 最後 ＝ 取り出す（数字）；もし 最後 ＝＝ ２ かつ 要素数（数字） ＝＝ １ ならば ｛ 返す １； ｝違えば｛ 返す ０； ｝";
        assert_eq!(run(src), Some(Value::Int(1)));
    }

    #[test]
    fn test_vm_pop_empty_array_returns_error() {
        let src = "取り込む 「配列」；整数列 数字 ＝ 新配列＜整数＞；返す 取り出す（数字）；";
        assert_eq!(run_result(src), Err(RuntimeError::EmptyArray));
    }

    #[test]
    fn test_vm_contains_array_found_and_not_found() {
        let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；返す 含む配列（数字、２）；";
        assert_eq!(run(src), Some(Value::Bool(true)));

        let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；返す 含む配列（数字、９）；";
        assert_eq!(run(src), Some(Value::Bool(false)));
    }

    #[test]
    fn test_vm_index_of_found_and_not_found() {
        let src =
            "取り込む 「配列」；整数列 数字 ＝ 【１０、２０、３０】；返す 位置（数字、２０）；";
        assert_eq!(run(src), Some(Value::Int(1)));

        let src =
            "取り込む 「配列」；整数列 数字 ＝ 【１０、２０、３０】；返す 位置（数字、９９）；";
        assert_eq!(run(src), Some(Value::Int(-1)));
    }

    #[test]
    fn test_vm_reverse_mutates_in_place_and_returns_array() {
        let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；整数列 同じ ＝ 逆順（数字）；返す 数字【０】；";
        assert_eq!(run(src), Some(Value::Int(3)));
    }

    #[test]
    fn test_vm_sort_int_array() {
        let src =
            "取り込む 「配列」；整数列 数字 ＝ 【３、１、２】；整列（数字）；返す 数字【０】；";
        assert_eq!(run(src), Some(Value::Int(1)));
    }

    #[test]
    fn test_vm_sort_float_array() {
        let src = "取り込む 「配列」；小数列 数字 ＝ 【３．０、１．０、２．０】；整列（数字）；返す 数字【０】；";
        assert_eq!(run(src), Some(Value::Float(1.0)));
    }

    #[test]
    fn test_vm_sort_string_array() {
        let src = "取り込む 「配列」；文字列列 文字 ＝ 【「う」、「あ」、「い」】；整列（文字）；返す 文字【０】；";
        assert_eq!(run(src), Some(Value::Str("あ".to_string())));
    }

    #[test]
    fn test_vm_slice_returns_new_array_without_mutating_original() {
        let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３、４】；整数列 部分 ＝ 部分列（数字、１、３）；もし 部分【０】 ＝＝ ２ かつ 部分【１】 ＝＝ ３ かつ 要素数（数字） ＝＝ ４ ならば ｛ 返す １； ｝違えば｛ 返す ０； ｝";
        assert_eq!(run(src), Some(Value::Int(1)));
    }

    #[test]
    fn test_vm_slice_out_of_bounds_returns_error() {
        let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２】；返す 部分列（数字、０、５）；";
        assert!(matches!(
            run_result(src),
            Err(RuntimeError::IndexOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_vm_new_array_then_build_and_check_len() {
        let src = "取り込む 「配列」；文字列列 単語 ＝ 新配列＜文字列＞；追加（単語、「あ」）；追加（単語、「い」）；返す 要素数（単語）；";
        assert_eq!(run(src), Some(Value::Int(2)));
    }

    // ── 7c: more math builtins ─────────────────────────────────────────

    #[test]
    fn test_vm_pow_int_and_float() {
        let src = "取り込む 「数学」；返す 累乗（２、１０）；";
        assert_eq!(run(src), Some(Value::Int(1024)));

        let src = "取り込む 「数学」；返す 累乗（２．０、０．５）；";
        match run(src) {
            Some(Value::Float(f)) => assert!((f - std::f64::consts::SQRT_2).abs() < 1e-9),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    #[test]
    fn test_vm_pow_negative_exponent_returns_error() {
        let src = "取り込む 「数学」；返す 累乗（２、ー１）；";
        assert!(matches!(
            run_result(src),
            Err(RuntimeError::InvalidConversion(_))
        ));
    }

    #[test]
    fn test_vm_floor_ceil_round() {
        let src = "取り込む 「数学」；返す 切り捨て（３．７）；";
        assert_eq!(run(src), Some(Value::Int(3)));

        let src = "取り込む 「数学」；返す 切り上げ（３．２）；";
        assert_eq!(run(src), Some(Value::Int(4)));

        let src = "取り込む 「数学」；返す 四捨五入（３．５）；";
        assert_eq!(run(src), Some(Value::Int(4)));
    }

    #[test]
    fn test_vm_remainder_function_form() {
        let src = "取り込む 「数学」；返す 余り（１０、３）；";
        assert_eq!(run(src), Some(Value::Int(1)));
    }

    #[test]
    fn test_vm_remainder_by_zero_returns_error() {
        let src = "取り込む 「数学」；返す 余り（１０、０）；";
        assert_eq!(run_result(src), Err(RuntimeError::DivisionByZero));
    }

    // ── 8b: break / continue ─────────────────────────────────────────────

    #[test]
    fn test_vm_while_break_stops_early() {
        let src = "整数 カウンタ ＝ ０；間 カウンタ ＜ １０ ならば ｛ もし カウンタ ＝＝ ５ ならば ｛ 抜ける； ｝ カウンタ ＝ カウンタ ＋ １； ｝返す カウンタ；";
        assert_eq!(run(src), Some(Value::Int(5)));
    }

    #[test]
    fn test_vm_while_continue_skips_even_numbers_sums_odds() {
        // 続ける skips adding on even numbers, but the loop variable still
        // increments and the loop still completes: 1+3+5+7+9 = 25.
        let src = "整数 合計 ＝ ０；整数 ｉ ＝ １；間 ｉ ≦ １０ ならば ｛ もし ｉ ％ ２ ＝＝ ０ ならば ｛ ｉ ＝ ｉ ＋ １；続ける； ｝ 合計 ＝ 合計 ＋ ｉ；ｉ ＝ ｉ ＋ １； ｝返す 合計；";
        assert_eq!(run(src), Some(Value::Int(25)));
    }

    #[test]
    fn test_vm_for_range_break_stops_early() {
        let src = "整数 合計 ＝ ０；繰り返す ｉ ＝ ０ から １０ ならば ｛ もし ｉ ＝＝ ５ ならば ｛ 抜ける； ｝ 合計 ＝ 合計 ＋ １； ｝返す 合計；";
        assert_eq!(run(src), Some(Value::Int(5)));
    }

    #[test]
    fn test_vm_for_range_continue_skips_even_increments_correctly() {
        // Proves the increment step still runs on a 続ける'd iteration:
        // summing 0..10 while skipping evens gives 1+3+5+7+9 = 25.
        let src = "整数 合計 ＝ ０；繰り返す ｉ ＝ ０ から １０ ならば ｛ もし ｉ ％ ２ ＝＝ ０ ならば ｛ 続ける； ｝ 合計 ＝ 合計 ＋ ｉ； ｝返す 合計；";
        assert_eq!(run(src), Some(Value::Int(25)));
    }

    #[test]
    fn test_vm_for_each_break_stops_early() {
        let src = "整数列 数字 ＝ 【１、２、３、４、５】；整数 合計 ＝ ０；各 値 ： 数字 ならば ｛ もし 値 ＝＝ ３ ならば ｛ 抜ける； ｝ 合計 ＝ 合計 ＋ 値； ｝返す 合計；";
        assert_eq!(run(src), Some(Value::Int(3))); // 1 + 2
    }

    #[test]
    fn test_vm_for_each_continue_skips_even_increments_correctly() {
        let src = "整数列 数字 ＝ 【１、２、３、４、５、６、７、８、９、１０】；整数 合計 ＝ ０；各 値 ： 数字 ならば ｛ もし 値 ％ ２ ＝＝ ０ ならば ｛ 続ける； ｝ 合計 ＝ 合計 ＋ 値； ｝返す 合計；";
        assert_eq!(run(src), Some(Value::Int(25)));
    }

    #[test]
    fn test_vm_break_inside_nested_if_inside_loop() {
        let src = "整数 カウンタ ＝ ０；間 真 ならば ｛ もし 真 ならば ｛ もし カウンタ ＝＝ ３ ならば ｛ 抜ける； ｝ ｝ カウンタ ＝ カウンタ ＋ １； ｝返す カウンタ；";
        assert_eq!(run(src), Some(Value::Int(3)));
    }

    #[test]
    fn test_vm_outer_loop_break_unaffected_by_inner_loop_break() {
        // Inner loop's 抜ける only exits the inner loop (after 2 inner runs
        // each outer iteration); outer loop runs its own 3 iterations.
        let src = "整数 外回数 ＝ ０；整数 内合計 ＝ ０；繰り返す 外 ＝ ０ から ３ ならば ｛ 外回数 ＝ 外回数 ＋ １；繰り返す 内 ＝ ０ から １０ ならば ｛ もし 内 ＝＝ ２ ならば ｛ 抜ける； ｝ 内合計 ＝ 内合計 ＋ １； ｝ ｝返す 外回数 ＊ １００ ＋ 内合計；";
        // Outer runs 3 times; inner runs 2 iterations (0,1) each time → 6.
        assert_eq!(run(src), Some(Value::Int(306)));
    }

    // ── 8c: bare return / void semantics ──────────────────────────────────

    #[test]
    fn test_vm_bare_return_in_void_function_keeps_stack_balanced() {
        let src =
            "関数 何もしない（）ー＞ 無 ｛ 返す； ｝何もしない（）；整数 結果 ＝ ４２；返す 結果；";
        assert_eq!(run(src), Some(Value::Int(42)));
    }

    // ── 9a: records ───────────────────────────────────────────────────────

    #[test]
    fn test_vm_record_construct_and_read_field() {
        let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝点 ｐ ＝ 点 ｛ ｘ：１、ｙ：２ ｝；返す ｐ：：ｘ ＋ ｐ：：ｙ；";
        assert_eq!(run(src), Some(Value::Int(3)));
    }

    #[test]
    fn test_vm_record_field_assign_mutates() {
        let src =
            "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；ｐ：：ｘ ＝ ９９；返す ｐ：：ｘ；";
        assert_eq!(run(src), Some(Value::Int(99)));
    }

    #[test]
    fn test_vm_record_aliasing_reference_semantics() {
        // Assigning Ａ to Ｂ shares the same underlying Rc<RefCell<>>, so
        // mutating a field through Ｂ must be visible through Ａ, mirroring
        // array aliasing.
        let src = "型 点 ｛ 整数 ｘ； ｝点 Ａ ＝ 点 ｛ ｘ：１ ｝；点 Ｂ ＝ Ａ；Ｂ：：ｘ ＝ ９９；返す Ａ：：ｘ；";
        assert_eq!(run(src), Some(Value::Int(99)));
    }

    #[test]
    fn test_vm_record_as_function_param_and_return() {
        let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝関数 ずらす（点 Ｐ）ー＞ 点 ｛ Ｐ：：ｘ ＝ Ｐ：：ｘ ＋ １；返す Ｐ； ｝点 ａ ＝ 点 ｛ ｘ：１、ｙ：２ ｝；点 ｂ ＝ ずらす（ａ）；返す ｂ：：ｘ；";
        assert_eq!(run(src), Some(Value::Int(2)));
    }

    #[test]
    fn test_vm_record_with_array_field() {
        let src = "型 箱 ｛ 整数列 数字； ｝箱 ｂ ＝ 箱 ｛ 数字：【１、２、３】 ｝；返す ｂ：：数字【１】；";
        assert_eq!(run(src), Some(Value::Int(2)));
    }

    // ── 9b: enums and pattern matching ──────────────────────────────────

    #[test]
    fn test_vm_construct_and_print_payload_and_payloadless_variants() {
        let src = "構造 結果 ｛ 成功（整数）、 異常 ｝印刷（成功（１２３））；印刷（異常（））；";
        let result = run(src);
        assert_eq!(result, None);
    }

    #[test]
    fn test_vm_match_dispatches_to_correct_arm_for_each_variant() {
        let src = "構造 信号 ｛ 赤、 黄、 青 ｝関数 名前（信号 値）ー＞ 整数 ｛ 照合 値 ｛ 赤（） ならば ｛ 返す １； ｝ 黄（） ならば ｛ 返す ２； ｝ 青（） ならば ｛ 返す ３； ｝ ｝返す ０； ｝返す 名前（赤（）） ＊ １００ ＋ 名前（黄（）） ＊ １０ ＋ 名前（青（））；";
        assert_eq!(run(src), Some(Value::Int(123)));
    }

    #[test]
    fn test_vm_match_binder_receives_correct_payload_values_in_order() {
        let src = "構造 結果 ｛ 点（整数、整数） ｝結果 値 ＝ 点（３、４）；照合 値 ｛ 点（ｘ、ｙ） ならば ｛ 返す ｘ ＊ １０ ＋ ｙ； ｝ ｝";
        assert_eq!(run(src), Some(Value::Int(34)));
    }

    #[test]
    fn test_vm_non_exhaustive_match_rejected_at_typecheck_time() {
        let src = "構造 信号 ｛ 赤、 青 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ ｝ ｝";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        let err = crate::typechecker::TypeChecker::new().check(&ast);
        assert!(err.is_err());
    }

    #[test]
    fn test_vm_build_sort_print_array_program() {
        let src = "取り込む 「配列」；整数列 数字 ＝ 新配列＜整数＞；追加（数字、５）；追加（数字、１）；追加（数字、３）；整列（数字）；返す 数字；";
        let result = run(src);
        match result {
            Some(Value::Array(a)) => {
                let v: Vec<i64> = a
                    .borrow()
                    .iter()
                    .map(|x| match x {
                        Value::Int(n) => *n,
                        _ => panic!("expected Int"),
                    })
                    .collect();
                assert_eq!(v, vec![1, 3, 5]);
            }
            other => panic!("expected Array, got {:?}", other),
        }
    }

    // ── 9c: maps ──────────────────────────────────────────────────────────

    #[test]
    fn test_vm_map_literal_creation_and_lookup() {
        let src = "取り込む 「辞書」；辞書＜文字列、整数＞ スコア ＝ ｛ 「アリス」：１００、「ボブ」：８５ ｝；返す スコア【「アリス」】；";
        assert_eq!(run(src), Some(Value::Int(100)));
    }

    #[test]
    fn test_vm_map_insert_and_contains() {
        let src = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛ 「あ」：１ ｝；m【「い」】 ＝ ２；真偽 結果 ＝ 含む（m、「い」）；返す 結果；";
        assert_eq!(run(src), Some(Value::Bool(true)));
    }

    #[test]
    fn test_vm_map_keys_and_values_builtins() {
        // 鍵一覧 returns an array of string keys sorted alphabetically.
        let src = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛ 「い」：２、「あ」：１ ｝；文字列列 ks ＝ 鍵一覧（m）；返す ks【０】；";
        assert_eq!(run(src), Some(Value::Str("あ".to_string())));

        // 値一覧 returns values in key-sorted order.
        let src2 = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛ 「い」：２、「あ」：１ ｝；整数列 vs ＝ 値一覧（m）；返す vs【０】；";
        assert_eq!(run(src2), Some(Value::Int(1)));
    }

    #[test]
    fn test_vm_map_delete_removes_key() {
        let src = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛ 「あ」：１、「い」：２ ｝；削除（m、「あ」）；返す 含む（m、「あ」）；";
        assert_eq!(run(src), Some(Value::Bool(false)));
    }

    #[test]
    fn test_vm_map_missing_key_returns_error() {
        let src =
            "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛ 「あ」：１ ｝；返す m【「い」】；";
        assert_eq!(
            run_result(src),
            Err(RuntimeError::KeyNotFound("い".to_string()))
        );
    }

    #[test]
    fn test_vm_empty_map_literal_and_insert() {
        let src = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛｝；m【「キー」】 ＝ ４２；返す m【「キー」】；";
        assert_eq!(run(src), Some(Value::Int(42)));
    }

    #[test]
    fn test_vm_lambda_creation_and_call() {
        // Lambda stored in var, then called through var.
        // The var decl ends with ；ー
        let src = "関数＜（整数） ー＞ 整数＞ f ＝ ｜ｎ：整数｜ ー＞ 整数 ｛ 返す ｎ ＊ ２； ｝；返す f（５）；";
        assert_eq!(run(src), Some(Value::Int(10)));
    }

    #[test]
    fn test_vm_named_function_as_value() {
        // Named function used as a first-class value.
        let src = "関数 二倍（整数 ｎ）ー＞ 整数 ｛ 返す ｎ ＊ ２； ｝関数＜（整数） ー＞ 整数＞ f ＝ 二倍；返す f（７）；";
        assert_eq!(run(src), Some(Value::Int(14)));
    }

    #[test]
    fn test_vm_map_array_with_named_function() {
        // マップ HOF with named function
        let src = "取り込む 「関数」；関数 二倍（整数 ｎ）ー＞ 整数 ｛ 返す ｎ ＊ ２； ｝整数列 nums ＝ 【１、２、３】；整数列 result ＝ マップ（nums、二倍）；返す result【２】；";
        assert_eq!(run(src), Some(Value::Int(6)));
    }

    #[test]
    fn test_vm_filter_array_with_lambda() {
        // 絞り込み HOF with lambda predicate (lambda is an argument, no extra ；)
        let src = "取り込む 「関数」；取り込む 「配列」；整数列 nums ＝ 【１、２、３、４、５】；整数列 evens ＝ 絞り込み（nums、｜ｎ：整数｜ ー＞ 真偽 ｛ 返す ｎ ％ ２ ＝＝ ０； ｝）；返す 要素数（evens）；";
        assert_eq!(run(src), Some(Value::Int(2)));
    }

    #[test]
    fn test_vm_fold_array() {
        // 畳み込み sum
        let src = "取り込む 「関数」；整数列 nums ＝ 【１、２、３、４、５】；整数 total ＝ 畳み込み（nums、０、｜acc：整数、ｎ：整数｜ ー＞ 整数 ｛ 返す acc ＋ ｎ； ｝）；返す total；";
        assert_eq!(run(src), Some(Value::Int(15)));
    }
}

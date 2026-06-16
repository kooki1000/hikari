use std::cell::RefCell;
use std::rc::Rc;

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
        // Implicit return when execution reaches the end of a chunk.
        if frame.ip >= frame.instructions.len() {
            self.frames.pop();
            return Ok(StepResult::Halt(None));
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
                        self.stack.push(arith(l, r, |a, b| a + b, |a, b| a + b)?);
                    }
                }
            }
            Instruction::Sub => {
                let (l, r) = self.pop2()?;
                self.stack.push(arith(l, r, |a, b| a - b, |a, b| a - b)?);
            }
            Instruction::Mul => {
                let (l, r) = self.pop2()?;
                self.stack.push(arith(l, r, |a, b| a * b, |a, b| a * b)?);
            }
            Instruction::Div => {
                let (l, r) = self.pop2()?;
                match &r {
                    Value::Int(0) => return Err(RuntimeError::DivisionByZero),
                    Value::Float(f) if *f == 0.0 => return Err(RuntimeError::DivisionByZero),
                    _ => {}
                }
                self.stack.push(arith(l, r, |a, b| a / b, |a, b| a / b)?);
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
                let stack_len = self.stack.len();
                let mut args = self.stack.split_off(stack_len - argc as usize);
                let result = call_builtin(builtin, &mut args)?;
                self.stack.push(result);
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
                    Value::Int(n) => Value::Int(-n),
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
            Instruction::MakeArray(n) => {
                let stack_len = self.stack.len();
                let elements = self.stack.split_off(stack_len - n as usize);
                self.stack
                    .push(Value::Array(Rc::new(RefCell::new(elements))));
            }
            Instruction::GetIndex => {
                let index = match self.stack.pop().ok_or(RuntimeError::StackUnderflow)? {
                    Value::Int(i) => i,
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                let arr = match self.stack.pop().ok_or(RuntimeError::StackUnderflow)? {
                    Value::Array(a) => a,
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
            Instruction::SetIndex => {
                let value = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let index = match self.stack.pop().ok_or(RuntimeError::StackUnderflow)? {
                    Value::Int(i) => i,
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                let arr = match self.stack.pop().ok_or(RuntimeError::StackUnderflow)? {
                    Value::Array(a) => a,
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
    }
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
    min + (seed as i64).rem_euclid(max - min + 1)
}

fn arith(
    lhs: Value,
    rhs: Value,
    int_op: impl Fn(i64, i64) -> i64,
    float_op: impl Fn(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(a, b))),
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
        assert_eq!(result, Some(Value::Int(0 + 1 + 2 + 3 + 4)));
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
}

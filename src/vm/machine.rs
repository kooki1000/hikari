use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::compiler::{BuiltinFn, Chunk, Instruction, Value};
use crate::lexer::Span;

use super::builtins::call_builtin;
use super::error::RuntimeError;
use super::frame::{Frame, INITIAL_LOCALS, TryHandler};
use super::value_ops::{arith, cmp_ge, cmp_gt, cmp_le, cmp_lt, display_value};

// ── VM ────────────────────────────────────────────────────────────────────────

pub struct Vm {
    constants: Vec<Value>,
    chunks: Vec<Chunk>,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    try_stack: Vec<TryHandler>,
    // Source span of the most recent uncaught runtime error, for diagnostics.
    error_span: Option<Span>,
    // CLI arguments passed to the running program, returned by the 引数 builtin.
    program_args: Vec<String>,
}

enum StepResult {
    Continue,
    Halt(Option<Value>),
}

// Maximum number of simultaneously active call frames. Exceeding this raises a
// clean `再帰が深すぎます` error instead of letting unbounded recursion grow the
// frame vector until the process is killed by the OS.
const MAX_FRAME_DEPTH: usize = 1024;

impl Vm {
    /// Construct from a script chunk (no named functions).
    #[allow(dead_code)] // used in low-level unit tests that bypass the compiler
    pub fn new(constants: Vec<Value>, instructions: Vec<Instruction>) -> Self {
        let script_chunk = Chunk {
            instructions,
            param_count: 0,
            spans: Vec::new(),
        };
        let frame = Frame::new(&script_chunk, vec![]);
        Self {
            constants,
            chunks: vec![script_chunk],
            stack: Vec::new(),
            frames: vec![frame],
            try_stack: Vec::new(),
            error_span: None,
            program_args: Vec::new(),
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
            spans: Vec::new(),
        };
        let frame = Frame::new(&script_chunk, vec![]);
        Self {
            constants,
            chunks,
            stack: Vec::new(),
            frames: vec![frame],
            try_stack: Vec::new(),
            error_span: None,
            program_args: Vec::new(),
        }
    }

    /// Install the top-level script's span checkpoints (from
    /// `Compiler::script_spans`) onto frame 0, so runtime errors in top-level
    /// code can report a source location.
    pub fn set_script_spans(&mut self, spans: Vec<(usize, Span)>) {
        if let Some(frame) = self.frames.first_mut() {
            frame.spans = spans;
        }
    }

    /// The source span of the most recent uncaught runtime error, if known.
    pub fn error_span(&self) -> Option<Span> {
        self.error_span
    }

    /// Set the CLI arguments the program sees via the 引数 builtin.
    pub fn set_program_args(&mut self, args: Vec<String>) {
        self.program_args = args;
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
                let val = self
                    .frames
                    .last()
                    .unwrap()
                    .get_local(slot)
                    .ok_or(RuntimeError::UninitializedLocal(slot))?;
                self.stack.push(val);
            }
            Instruction::StoreLocal(slot) => {
                let val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                self.frames.last_mut().unwrap().set_local(slot, val);
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
                self.push_frame(new_frame)?;
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
                        let (chunk_index, captured) = match fn_val {
                            Value::Function {
                                chunk_index,
                                captured,
                                ..
                            } => (chunk_index, captured),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let elements = match arr_val {
                            Value::Array(a) => a.borrow().clone(),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let mut results = Vec::new();
                        for elem in elements {
                            let result =
                                self.call_function(chunk_index, vec![elem], captured.clone())?;
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
                        let (chunk_index, captured) = match fn_val {
                            Value::Function {
                                chunk_index,
                                captured,
                                ..
                            } => (chunk_index, captured),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let elements = match arr_val {
                            Value::Array(a) => a.borrow().clone(),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let mut results = Vec::new();
                        for elem in elements {
                            let result = self.call_function(
                                chunk_index,
                                vec![elem.clone()],
                                captured.clone(),
                            )?;
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
                        let (chunk_index, captured) = match fn_val {
                            Value::Function {
                                chunk_index,
                                captured,
                                ..
                            } => (chunk_index, captured),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let elements = match arr_val {
                            Value::Array(a) => a.borrow().clone(),
                            _ => return Err(RuntimeError::TypeMismatch),
                        };
                        let mut acc = init;
                        for elem in elements {
                            acc =
                                self.call_function(chunk_index, vec![acc, elem], captured.clone())?;
                        }
                        self.stack.push(acc);
                    }
                    BuiltinFn::ProgramArgs => {
                        // Handled here (not call_builtin) because it reads the
                        // VM's stored program arguments.
                        let args: Vec<Value> = self
                            .program_args
                            .iter()
                            .map(|a| Value::Str(a.clone()))
                            .collect();
                        self.stack.push(Value::Array(Rc::new(RefCell::new(args))));
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
            Instruction::PrintLine(n) => {
                let stack_len = self.stack.len();
                if stack_len < n as usize {
                    return Err(RuntimeError::StackUnderflow);
                }
                let values = self.stack.split_off(stack_len - n as usize);
                // Values print space-separated, with a trailing newline.
                let line = values
                    .iter()
                    .map(display_value)
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("{}", line);
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
            // Push a (non-capturing) function value onto the stack.
            Instruction::LoadFn { chunk_index, arity } => {
                self.stack.push(Value::Function {
                    chunk_index,
                    arity,
                    captured: Vec::new(),
                });
            }
            // Pop `capture_count` captured values and push a closure over them.
            Instruction::MakeClosure {
                chunk_index,
                arity,
                capture_count,
            } => {
                let stack_len = self.stack.len();
                let captured = self.stack.split_off(stack_len - capture_count as usize);
                self.stack.push(Value::Function {
                    chunk_index,
                    arity,
                    captured,
                });
            }
            // Pop the function value and its args, then push a new frame.
            Instruction::CallValue(arg_count) => {
                let fn_val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                match fn_val {
                    Value::Function {
                        chunk_index,
                        arity,
                        captured,
                    } => {
                        if arg_count != arity {
                            return Err(RuntimeError::TypeMismatch);
                        }
                        let chunk = &self.chunks[chunk_index];
                        let stack_len = self.stack.len();
                        let mut seed = self.stack.split_off(stack_len - arg_count as usize);
                        // Captured values occupy locals right after the params.
                        seed.extend(captured);
                        let new_frame = Frame::new(chunk, seed);
                        self.push_frame(new_frame)?;
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
                        frame.set_local(handler.error_slot, Value::Str(e.to_string()));
                        frame.ip = handler.catch_target;
                    } else {
                        self.error_span = self.current_error_span();
                        return Err(e);
                    }
                }
            }
        }
    }

    pub fn run_repl_line(
        &mut self,
        new_instrs: Vec<Instruction>,
        new_spans: Vec<(usize, Span)>,
    ) -> Result<Option<Value>, RuntimeError> {
        let start_ip = self.frames[0].instructions.len();
        self.frames[0].instructions.extend(new_instrs);
        // Span checkpoints are emitted relative to this line's start; shift
        // them to frame 0's absolute instruction indices before appending.
        self.frames[0]
            .spans
            .extend(new_spans.into_iter().map(|(i, s)| (i + start_ip, s)));
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
                            locals: vec![None; INITIAL_LOCALS],
                            spans: Vec::new(),
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
                        frame.set_local(handler.error_slot, Value::Str(e.to_string()));
                        frame.ip = handler.catch_target;
                    } else {
                        self.error_span = self.current_error_span();
                        return Err(e);
                    }
                }
            }
        }
    }

    /// The source span of the instruction currently being executed in the
    /// top frame. `step` increments `ip` before dispatching, so the erroring
    /// instruction is at `ip - 1`.
    fn current_error_span(&self) -> Option<Span> {
        let frame = self.frames.last()?;
        frame.span_at(frame.ip.saturating_sub(1))
    }

    pub fn sync_program(&mut self, constants: Vec<Value>, chunks: Vec<Chunk>) {
        self.constants = constants;
        self.chunks = chunks;
    }

    /// Push a new call frame, enforcing the recursion-depth limit. Centralizes
    /// the guard so every call path (`Call`, `CallValue`, HOF `call_function`)
    /// gets the same clean overflow error.
    fn push_frame(&mut self, frame: Frame) -> Result<(), RuntimeError> {
        if self.frames.len() >= MAX_FRAME_DEPTH {
            return Err(RuntimeError::StackOverflow);
        }
        self.frames.push(frame);
        Ok(())
    }

    fn pop2(&mut self) -> Result<(Value, Value), RuntimeError> {
        let rhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
        let lhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
        Ok((lhs, rhs))
    }

    /// call a chunk by index with the given arguments and run it to
    /// completion, returning the produced value. Used by HOF builtins.
    /// `captured` holds any closure-captured values, seeded into the callee's
    /// locals right after the params (matching `CallValue`).
    fn call_function(
        &mut self,
        chunk_index: usize,
        mut args: Vec<Value>,
        captured: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let chunk = &self.chunks[chunk_index];
        args.extend(captured);
        let frame = Frame::new(chunk, args);
        let target_depth = self.frames.len(); // depth BEFORE pushing the new frame
        self.push_frame(frame)?;
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

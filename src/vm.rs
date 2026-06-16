use crate::compiler::{Chunk, Instruction, Value};

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum RuntimeError {
    StackUnderflow,
    UninitializedLocal(u16),
    DivisionByZero,
    TypeMismatch,
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

// ── VM ────────────────────────────────────────────────────────────────────────

pub struct Vm {
    constants: Vec<Value>,
    chunks: Vec<Chunk>,
    stack: Vec<Value>,
    frames: Vec<Frame>,
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
        }
    }

    pub fn run(&mut self) -> Result<Option<Value>, RuntimeError> {
        loop {
            let frame = self.frames.last_mut().expect("no active frame");
            // Implicit return when execution reaches the end of a chunk.
            if frame.ip >= frame.instructions.len() {
                self.frames.pop();
                return Ok(None);
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
                    self.stack.push(arith(l, r, |a, b| a + b, |a, b| a + b)?);
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
                        return Ok(return_val);
                    }
                    // Push return value back onto the caller's stack.
                    if let Some(val) = return_val {
                        self.stack.push(val);
                    }
                }
            }
        }
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
        let src = "関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝返す 加算（３、４）；";
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
    fn test_vm_uninitialized_local_returns_error() {
        // もし １ ＝＝ ２ ならば ｛ 整数 Ａ ＝ １； ｝ 返す Ａ；
        // The then-branch never runs, so Ａ's slot is never stored before the load.
        let ast = Parser::new(
            Lexer::new("もし １ ＝＝ ２ ならば ｛ 整数 Ａ ＝ １； ｝返す Ａ；").tokenize(),
        )
        .parse()
        .unwrap();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
        assert_eq!(result, Err(RuntimeError::UninitializedLocal(0)));
    }
}

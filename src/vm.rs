use crate::compiler::{Chunk, Instruction, Value};

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

    pub fn run(&mut self) -> Option<Value> {
        loop {
            let frame = self.frames.last_mut().expect("no active frame");
            // Implicit return when execution reaches the end of a chunk.
            if frame.ip >= frame.instructions.len() {
                self.frames.pop();
                return None;
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
                        .expect("load of uninitialised local");
                    self.stack.push(val);
                }
                Instruction::StoreLocal(slot) => {
                    let val = self.stack.pop().expect("stack underflow on StoreLocal");
                    self.frames.last_mut().unwrap().locals[slot as usize] = Some(val);
                }
                Instruction::Add => {
                    let (l, r) = self.pop2();
                    self.stack.push(arith(l, r, |a, b| a + b, |a, b| a + b));
                }
                Instruction::Sub => {
                    let (l, r) = self.pop2();
                    self.stack.push(arith(l, r, |a, b| a - b, |a, b| a - b));
                }
                Instruction::Mul => {
                    let (l, r) = self.pop2();
                    self.stack.push(arith(l, r, |a, b| a * b, |a, b| a * b));
                }
                Instruction::Div => {
                    let (l, r) = self.pop2();
                    self.stack.push(arith(l, r, |a, b| a / b, |a, b| a / b));
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
                Instruction::Print => {
                    let val = self.stack.pop().expect("stack underflow on Print");
                    println!("{}", display_value(&val));
                }
                Instruction::Return => {
                    let return_val = self.stack.pop();
                    self.frames.pop();
                    if self.frames.is_empty() {
                        return return_val;
                    }
                    // Push return value back onto the caller's stack.
                    if let Some(val) = return_val {
                        self.stack.push(val);
                    }
                }
            }
        }
    }

    fn pop2(&mut self) -> (Value, Value) {
        let rhs = self.stack.pop().expect("stack underflow");
        let lhs = self.stack.pop().expect("stack underflow");
        (lhs, rhs)
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

fn arith(
    lhs: Value,
    rhs: Value,
    int_op: impl Fn(i64, i64) -> i64,
    float_op: impl Fn(f64, f64) -> f64,
) -> Value {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Value::Int(int_op(a, b)),
        (Value::Float(a), Value::Float(b)) => Value::Float(float_op(a, b)),
        (l, r) => panic!("type error in arithmetic: {:?} and {:?}", l, r),
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
        let ast = Parser::new(Lexer::new(src).tokenize()).parse();
        let mut compiler = Compiler::new();
        let script = compiler.compile(&ast);
        Vm::with_chunks(compiler.constants, compiler.chunks, script).run()
    }

    #[test]
    fn test_vm_push_constant() {
        let constants = vec![Value::Int(42)];
        let instructions = vec![Instruction::LoadConst(0), Instruction::Return];
        let result = Vm::new(constants, instructions).run();
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
    fn test_vm_call_with_expression_arg() {
        // 関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝
        // 返す 二倍（３ ＋ ４）；  →  14
        let src = "関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝返す 二倍（３ ＋ ４）；";
        assert_eq!(run(src), Some(Value::Int(14)));
    }
}

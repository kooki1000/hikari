use crate::compiler::{Instruction, Value};

// ── VM ────────────────────────────────────────────────────────────────────────

pub struct Vm {
    constants: Vec<Value>,
    instructions: Vec<Instruction>,
    stack: Vec<Value>,
    locals: Vec<Option<Value>>,
    ip: usize,
}

impl Vm {
    pub fn new(constants: Vec<Value>, instructions: Vec<Instruction>) -> Self {
        Self {
            constants,
            instructions,
            stack: Vec::new(),
            locals: vec![None; 256],
            ip: 0,
        }
    }

    pub fn run(&mut self) -> Option<Value> {
        loop {
            let instr = self.instructions[self.ip].clone();
            self.ip += 1;

            match instr {
                Instruction::LoadConst(idx) => {
                    self.stack.push(self.constants[idx as usize].clone());
                }
                Instruction::LoadLocal(slot) => {
                    let val = self.locals[slot as usize]
                        .clone()
                        .expect("load of uninitialised local");
                    self.stack.push(val);
                }
                Instruction::StoreLocal(slot) => {
                    let val = self.stack.pop().expect("stack underflow on StoreLocal");
                    self.locals[slot as usize] = Some(val);
                }
                Instruction::Add => {
                    let (lhs, rhs) = self.pop2();
                    self.stack.push(arith(lhs, rhs, |a, b| a + b, |a, b| a + b));
                }
                Instruction::Sub => {
                    let (lhs, rhs) = self.pop2();
                    self.stack.push(arith(lhs, rhs, |a, b| a - b, |a, b| a - b));
                }
                Instruction::Mul => {
                    let (lhs, rhs) = self.pop2();
                    self.stack.push(arith(lhs, rhs, |a, b| a * b, |a, b| a * b));
                }
                Instruction::Div => {
                    let (lhs, rhs) = self.pop2();
                    self.stack.push(arith(lhs, rhs, |a, b| a / b, |a, b| a / b));
                }
                Instruction::Return => {
                    return self.stack.pop();
                }
                Instruction::Call(_, _) => {
                    // Full call-frame dispatch is a future enhancement.
                    // For now, a Call with no matching definition is a no-op.
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
        let instructions = compiler.compile(&ast);
        Vm::new(compiler.constants, instructions).run()
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
        // 整数 年齢 ＝ ２０；  then 返す 年齢；
        let result = run("整数 年齢 ＝ ２０；返す 年齢；");
        assert_eq!(result, Some(Value::Int(20)));
    }

    #[test]
    fn test_vm_addition() {
        // 整数 結果 ＝ ３ ＋ ４；  返す 結果；
        let result = run("整数 結果 ＝ ３ ＋ ４；返す 結果；");
        assert_eq!(result, Some(Value::Int(7)));
    }

    #[test]
    fn test_vm_operator_precedence() {
        // 整数 結果 ＝ ２ ＋ ３ ＊ ４；  返す 結果；  → 2 + 12 = 14
        let result = run("整数 結果 ＝ ２ ＋ ３ ＊ ４；返す 結果；");
        assert_eq!(result, Some(Value::Int(14)));
    }

    #[test]
    fn test_vm_function_body() {
        // 関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
        // The function body is compiled inline; param Ａ has no slot pre-loaded,
        // so we seed it manually via a preceding VarDecl.
        let result = run("整数 Ａ ＝ ９；関数 計算（整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝");
        assert_eq!(result, Some(Value::Int(10)));
    }
}

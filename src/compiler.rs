use std::collections::HashMap;

use crate::parser::{BinOpKind, Expr, Stmt};

// ── Value (constant pool entries) ─────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}

// ── Built-in functions ────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BuiltinFn {
    Len,        // 文字数
    Input,      // 入力
    ParseInt,   // 整数化
    ParseFloat, // 小数化
    ToStr,      // 文字列化
}

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
}

pub fn builtin_name(name: &str) -> Option<BuiltinFn> {
    match name {
        "文字数" => Some(BuiltinFn::Len),
        "入力" => Some(BuiltinFn::Input),
        "整数化" => Some(BuiltinFn::ParseInt),
        "小数化" => Some(BuiltinFn::ParseFloat),
        "文字列化" => Some(BuiltinFn::ToStr),
        _ => None,
    }
}

// ── Function chunk ────────────────────────────────────────────────────────────

// One compiled function: its instructions and the number of parameters
// (params occupy locals[0..param_count]).
#[derive(Debug, Clone)]
pub struct Chunk {
    pub instructions: Vec<Instruction>,
    #[allow(dead_code)] // reserved for arity checking in the type checker
    pub param_count: u8,
}

// ── Compiler ──────────────────────────────────────────────────────────────────

pub struct Compiler {
    pub constants: Vec<Value>,
    pub chunks: Vec<Chunk>,         // chunks[0] is the top-level script
    fn_index: HashMap<String, u16>, // function name → chunk index
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            chunks: Vec::new(),
            fn_index: HashMap::new(),
        }
    }

    pub fn compile(&mut self, stmts: &[Stmt]) -> Vec<Instruction> {
        // First pass: register all function names so forward calls resolve.
        for stmt in stmts {
            if let Stmt::FnDecl { name, params, .. } = stmt {
                // Reserve a chunk slot; the body is compiled below.
                let idx = self.chunks.len() as u16;
                self.fn_index.insert(name.clone(), idx);
                self.chunks.push(Chunk {
                    instructions: Vec::new(),
                    param_count: params.len() as u8,
                });
            }
        }

        // Second pass: compile function bodies and the top-level script.
        let mut script_instrs: Vec<Instruction> = Vec::new();
        let mut script_locals: HashMap<String, u16> = HashMap::new();

        for stmt in stmts {
            match stmt {
                Stmt::FnDecl {
                    name, params, body, ..
                } => {
                    let idx = self.fn_index[name] as usize;
                    let mut fn_locals: HashMap<String, u16> = HashMap::new();
                    // Parameters occupy the first local slots in order.
                    for (_, pname) in params {
                        let slot = fn_locals.len() as u16;
                        fn_locals.insert(pname.clone(), slot);
                    }
                    let mut fn_instrs = Vec::new();
                    for s in body {
                        self.emit_stmt(s, &mut fn_instrs, &mut fn_locals);
                    }
                    self.chunks[idx].instructions = fn_instrs;
                }
                other => {
                    self.emit_stmt(other, &mut script_instrs, &mut script_locals);
                }
            }
        }

        script_instrs
    }

    fn add_constant(&mut self, val: Value) -> u16 {
        if let Some(idx) = self.constants.iter().position(|c| c == &val) {
            return idx as u16;
        }
        let idx = self.constants.len() as u16;
        self.constants.push(val);
        idx
    }

    fn local_slot(locals: &mut HashMap<String, u16>, name: &str) -> u16 {
        if let Some(&slot) = locals.get(name) {
            return slot;
        }
        let slot = locals.len() as u16;
        locals.insert(name.to_string(), slot);
        slot
    }

    fn emit_stmt(
        &mut self,
        stmt: &Stmt,
        instrs: &mut Vec<Instruction>,
        locals: &mut HashMap<String, u16>,
    ) {
        match stmt {
            Stmt::VarDecl { name, value, .. } => {
                self.emit_expr(value, instrs, locals);
                let slot = Self::local_slot(locals, name);
                instrs.push(Instruction::StoreLocal(slot));
            }
            Stmt::FnDecl { .. } => {
                // Nested fn decls are not yet supported; top-level ones are
                // handled in compile() directly.
            }
            Stmt::Print(expr, _) => {
                self.emit_expr(expr, instrs, locals);
                instrs.push(Instruction::Print);
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.emit_expr(condition, instrs, locals);
                // Placeholder index; back-patched after then_body is emitted.
                let jump_if_false_idx = instrs.len();
                instrs.push(Instruction::JumpIfFalse(0));

                for s in then_body {
                    self.emit_stmt(s, instrs, locals);
                }

                if let Some(else_stmts) = else_body {
                    // Jump over else_body after then_body executes.
                    let jump_idx = instrs.len();
                    instrs.push(Instruction::Jump(0));
                    // Back-patch JumpIfFalse to land here (start of else).
                    let else_start = instrs.len() as u16;
                    instrs[jump_if_false_idx] = Instruction::JumpIfFalse(else_start);

                    for s in else_stmts {
                        self.emit_stmt(s, instrs, locals);
                    }
                    // Back-patch Jump to land after else_body.
                    let after_else = instrs.len() as u16;
                    instrs[jump_idx] = Instruction::Jump(after_else);
                } else {
                    // No else: back-patch JumpIfFalse to land after then_body.
                    let after_then = instrs.len() as u16;
                    instrs[jump_if_false_idx] = Instruction::JumpIfFalse(after_then);
                }
            }
            Stmt::While { condition, body, .. } => {
                let loop_start = instrs.len() as u16;
                self.emit_expr(condition, instrs, locals);
                let jump_if_false_idx = instrs.len();
                instrs.push(Instruction::JumpIfFalse(0));
                for s in body {
                    self.emit_stmt(s, instrs, locals);
                }
                instrs.push(Instruction::Jump(loop_start));
                let after_loop = instrs.len() as u16;
                instrs[jump_if_false_idx] = Instruction::JumpIfFalse(after_loop);
            }
            Stmt::Return(expr, _) => {
                self.emit_expr(expr, instrs, locals);
                instrs.push(Instruction::Return);
            }
            Stmt::ExprStmt(expr, _) => {
                self.emit_expr(expr, instrs, locals);
            }
            Stmt::Assign { name, value, .. } => {
                self.emit_expr(value, instrs, locals);
                let slot = Self::local_slot(locals, name);
                instrs.push(Instruction::StoreLocal(slot));
            }
        }
    }

    fn emit_expr(
        &mut self,
        expr: &Expr,
        instrs: &mut Vec<Instruction>,
        locals: &mut HashMap<String, u16>,
    ) {
        match expr {
            Expr::LitInt(n) => {
                let idx = self.add_constant(Value::Int(*n));
                instrs.push(Instruction::LoadConst(idx));
            }
            Expr::LitFloat(f) => {
                let idx = self.add_constant(Value::Float(*f));
                instrs.push(Instruction::LoadConst(idx));
            }
            Expr::LitString(s) => {
                let idx = self.add_constant(Value::Str(s.clone()));
                instrs.push(Instruction::LoadConst(idx));
            }
            Expr::LitBool(b) => {
                let idx = self.add_constant(Value::Bool(*b));
                instrs.push(Instruction::LoadConst(idx));
            }
            Expr::Ident(name) => {
                let slot = Self::local_slot(locals, name);
                instrs.push(Instruction::LoadLocal(slot));
            }
            Expr::BinOp {
                op: BinOpKind::And,
                lhs,
                rhs,
            } => {
                self.emit_expr(lhs, instrs, locals);
                let jump_if_false_idx = instrs.len();
                instrs.push(Instruction::JumpIfFalse(0));
                self.emit_expr(rhs, instrs, locals);
                let jump_end_idx = instrs.len();
                instrs.push(Instruction::Jump(0));
                let false_target = instrs.len() as u16;
                instrs[jump_if_false_idx] = Instruction::JumpIfFalse(false_target);
                let false_idx = self.add_constant(Value::Bool(false));
                instrs.push(Instruction::LoadConst(false_idx));
                let end = instrs.len() as u16;
                instrs[jump_end_idx] = Instruction::Jump(end);
            }
            Expr::BinOp {
                op: BinOpKind::Or,
                lhs,
                rhs,
            } => {
                self.emit_expr(lhs, instrs, locals);
                let jump_if_true_idx = instrs.len();
                instrs.push(Instruction::JumpIfTrue(0));
                self.emit_expr(rhs, instrs, locals);
                let jump_end_idx = instrs.len();
                instrs.push(Instruction::Jump(0));
                let true_target = instrs.len() as u16;
                instrs[jump_if_true_idx] = Instruction::JumpIfTrue(true_target);
                let true_idx = self.add_constant(Value::Bool(true));
                instrs.push(Instruction::LoadConst(true_idx));
                let end = instrs.len() as u16;
                instrs[jump_end_idx] = Instruction::Jump(end);
            }
            Expr::BinOp { op, lhs, rhs } => {
                self.emit_expr(lhs, instrs, locals);
                self.emit_expr(rhs, instrs, locals);
                let instr = match op {
                    BinOpKind::Add => Instruction::Add,
                    BinOpKind::Sub => Instruction::Sub,
                    BinOpKind::Mul => Instruction::Mul,
                    BinOpKind::Div => Instruction::Div,
                    BinOpKind::Eq => Instruction::Equal,
                    BinOpKind::Lt => Instruction::LessThan,
                    BinOpKind::Gt => Instruction::GreaterThan,
                    BinOpKind::LtEq => Instruction::LessEqual,
                    BinOpKind::GtEq => Instruction::GreaterEqual,
                    BinOpKind::NotEq => Instruction::NotEqual,
                    BinOpKind::And | BinOpKind::Or => unreachable!("handled above"),
                };
                instrs.push(instr);
            }
            Expr::UnaryMinus(inner) => {
                self.emit_expr(inner, instrs, locals);
                instrs.push(Instruction::Negate);
            }
            Expr::UnaryNot(inner) => {
                self.emit_expr(inner, instrs, locals);
                instrs.push(Instruction::Not);
            }
            Expr::Call { name, args } => {
                // Push arguments left-to-right; the VM seeds locals from them.
                for arg in args {
                    self.emit_expr(arg, instrs, locals);
                }
                if let Some(builtin) = builtin_name(name) {
                    instrs.push(Instruction::CallBuiltin(builtin, args.len() as u8));
                } else {
                    let fn_idx = self.fn_index[name];
                    instrs.push(Instruction::Call(fn_idx, args.len() as u8));
                }
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn compile(src: &str) -> (Vec<Instruction>, Vec<Value>) {
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        let mut c = Compiler::new();
        let instrs = c.compile(&ast);
        (instrs, c.constants)
    }

    #[test]
    fn test_compile_integer_literal() {
        let (instrs, constants) = compile("整数 年齢 ＝ ２０；");
        assert_eq!(instrs[0], Instruction::LoadConst(0));
        assert_eq!(instrs[1], Instruction::StoreLocal(0));
        assert_eq!(constants[0], Value::Int(20));
    }

    #[test]
    fn test_compile_bool_literal() {
        let (instrs, constants) = compile("真偽 フラグ ＝ 真；");
        assert_eq!(instrs[0], Instruction::LoadConst(0));
        assert_eq!(instrs[1], Instruction::StoreLocal(0));
        assert_eq!(constants[0], Value::Bool(true));
    }

    #[test]
    fn test_compile_binary_add() {
        let (instrs, constants) = compile("整数 結果 ＝ １ ＋ ２；");
        assert_eq!(instrs[0], Instruction::LoadConst(0));
        assert_eq!(instrs[1], Instruction::LoadConst(1));
        assert_eq!(instrs[2], Instruction::Add);
        assert_eq!(instrs[3], Instruction::StoreLocal(0));
        assert_eq!(constants, vec![Value::Int(1), Value::Int(2)]);
    }

    #[test]
    fn test_compile_constant_deduplication() {
        let (instrs, constants) = compile("整数 Ａ ＝ ５；整数 Ｂ ＝ ５；");
        assert_eq!(constants, vec![Value::Int(5)]);
        assert_eq!(instrs[0], Instruction::LoadConst(0));
        assert_eq!(instrs[2], Instruction::LoadConst(0));
    }

    #[test]
    fn test_compile_load_local() {
        let (instrs, _) = compile("整数 Ａ ＝ １０；整数 Ｂ ＝ Ａ；");
        assert_eq!(instrs[2], Instruction::LoadLocal(0));
        assert_eq!(instrs[3], Instruction::StoreLocal(1));
    }

    #[test]
    fn test_compile_return() {
        let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        let mut c = Compiler::new();
        c.compile(&ast);
        // The function chunk (index 0) must end with Return.
        assert_eq!(
            c.chunks[0].instructions.last().unwrap(),
            &Instruction::Return
        );
    }

    #[test]
    fn test_compile_while_loop() {
        // 整数 カウンタ ＝ ０；間 カウンタ ＜ ３ ならば ｛ 印刷（カウンタ）； ｝
        let src = "整数 カウンタ ＝ ０；間 カウンタ ＜ ３ ならば ｛ 印刷（カウンタ）； ｝";
        let (instrs, _) = compile(src);
        // layout: LoadConst(0), StoreLocal(0),        ← var decl
        //         [loop_start=2] LoadLocal(0), LoadConst(1), LessThan,  ← condition
        //         JumpIfFalse(after),                  ← idx 5
        //         LoadLocal(0), Print,                 ← body
        //         Jump(2),                             ← back-edge
        //         [after=9]
        assert!(matches!(instrs[5], Instruction::JumpIfFalse(9)));
        assert!(matches!(instrs[8], Instruction::Jump(2)));
    }

    #[test]
    fn test_compile_reassignment_reuses_slot() {
        let (instrs, _) = compile("整数 年齢 ＝ ２０；年齢 ＝ ３０；");
        assert_eq!(instrs[1], Instruction::StoreLocal(0));
        assert_eq!(instrs[3], Instruction::StoreLocal(0));
    }

    #[test]
    fn test_compile_unary_minus() {
        let (instrs, _) = compile("整数 結果 ＝ ー５；");
        assert_eq!(instrs[0], Instruction::LoadConst(0));
        assert_eq!(instrs[1], Instruction::Negate);
        assert_eq!(instrs[2], Instruction::StoreLocal(0));
    }

    #[test]
    fn test_compile_builtin_strlen_emits_call_builtin() {
        let (instrs, _) = compile("整数 結果 ＝ 文字数（「あ」）；");
        assert!(matches!(
            instrs[1],
            Instruction::CallBuiltin(BuiltinFn::Len, 1)
        ));
    }

    #[test]
    fn test_compile_builtin_input_emits_zero_args() {
        let (instrs, _) = compile("文字列 結果 ＝ 入力（）；");
        assert!(matches!(
            instrs[0],
            Instruction::CallBuiltin(BuiltinFn::Input, 0)
        ));
    }

    #[test]
    fn test_compile_builtin_to_str_emits_call_builtin() {
        let (instrs, _) = compile("文字列 結果 ＝ 文字列化（１）；");
        assert!(matches!(
            instrs[1],
            Instruction::CallBuiltin(BuiltinFn::ToStr, 1)
        ));
    }

    #[test]
    fn test_compile_call_emits_correct_fn_idx() {
        // 関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝ 返す 二倍（５）；
        let src = "関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝返す 二倍（５）；";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        let mut c = Compiler::new();
        let script = c.compile(&ast);
        // Script: LoadConst(5), Call(0, 1), Return
        assert!(matches!(script[1], Instruction::Call(0, 1)));
    }
}

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
    Call(u16, u8), // Call(fn_idx, arg_count) — future use
    Return,
}

// ── Compiler ──────────────────────────────────────────────────────────────────

pub struct Compiler {
    pub constants: Vec<Value>,
    pub instructions: Vec<Instruction>,
    locals: HashMap<String, u16>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            instructions: Vec::new(),
            locals: HashMap::new(),
        }
    }

    pub fn compile(&mut self, stmts: &[Stmt]) -> Vec<Instruction> {
        for stmt in stmts {
            self.emit_stmt(stmt);
        }
        self.instructions.clone()
    }

    fn add_constant(&mut self, val: Value) -> u16 {
        // Reuse existing constant if identical (simple deduplication).
        if let Some(idx) = self.constants.iter().position(|c| c == &val) {
            return idx as u16;
        }
        let idx = self.constants.len() as u16;
        self.constants.push(val);
        idx
    }

    fn local_slot(&mut self, name: &str) -> u16 {
        if let Some(&slot) = self.locals.get(name) {
            return slot;
        }
        let slot = self.locals.len() as u16;
        self.locals.insert(name.to_string(), slot);
        slot
    }

    fn emit(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }

    fn emit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl { name, value, .. } => {
                self.emit_expr(value);
                let slot = self.local_slot(name);
                self.emit(Instruction::StoreLocal(slot));
            }

            Stmt::FnDecl { body, .. } => {
                // For now compile the body inline (full call-frame support comes in the VM).
                for s in body {
                    self.emit_stmt(s);
                }
            }

            Stmt::Return(expr) => {
                self.emit_expr(expr);
                self.emit(Instruction::Return);
            }

            Stmt::ExprStmt(expr) => {
                self.emit_expr(expr);
            }
        }
    }

    fn emit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::LitInt(n) => {
                let idx = self.add_constant(Value::Int(*n));
                self.emit(Instruction::LoadConst(idx));
            }
            Expr::LitFloat(f) => {
                let idx = self.add_constant(Value::Float(*f));
                self.emit(Instruction::LoadConst(idx));
            }
            Expr::LitString(s) => {
                let idx = self.add_constant(Value::Str(s.clone()));
                self.emit(Instruction::LoadConst(idx));
            }
            Expr::LitBool(b) => {
                let idx = self.add_constant(Value::Bool(*b));
                self.emit(Instruction::LoadConst(idx));
            }
            Expr::Ident(name) => {
                let slot = self.local_slot(name);
                self.emit(Instruction::LoadLocal(slot));
            }
            Expr::BinOp { op, lhs, rhs } => {
                self.emit_expr(lhs);
                self.emit_expr(rhs);
                let instr = match op {
                    BinOpKind::Add => Instruction::Add,
                    BinOpKind::Sub => Instruction::Sub,
                    BinOpKind::Mul => Instruction::Mul,
                    BinOpKind::Div => Instruction::Div,
                };
                self.emit(instr);
            }
            Expr::Call { name: _, args } => {
                for arg in args {
                    self.emit_expr(arg);
                }
                // Placeholder: full function dispatch handled by the VM in Phase 5.
                self.emit(Instruction::Call(0, args.len() as u8));
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
        let ast = Parser::new(Lexer::new(src).tokenize()).parse();
        let mut c = Compiler::new();
        let instrs = c.compile(&ast);
        (instrs, c.constants)
    }

    #[test]
    fn test_compile_integer_literal() {
        // 整数 年齢 ＝ ２０；
        let (instrs, constants) = compile("整数 年齢 ＝ ２０；");
        assert_eq!(instrs[0], Instruction::LoadConst(0));
        assert_eq!(instrs[1], Instruction::StoreLocal(0));
        assert_eq!(constants[0], Value::Int(20));
    }

    #[test]
    fn test_compile_binary_add() {
        // 整数 結果 ＝ １ ＋ ２；
        let (instrs, constants) = compile("整数 結果 ＝ １ ＋ ２；");
        assert_eq!(instrs[0], Instruction::LoadConst(0)); // 1
        assert_eq!(instrs[1], Instruction::LoadConst(1)); // 2
        assert_eq!(instrs[2], Instruction::Add);
        assert_eq!(instrs[3], Instruction::StoreLocal(0));
        assert_eq!(constants, vec![Value::Int(1), Value::Int(2)]);
    }

    #[test]
    fn test_compile_constant_deduplication() {
        // The same literal appearing twice must reuse the same constant pool slot.
        let (instrs, constants) = compile("整数 Ａ ＝ ５；整数 Ｂ ＝ ５；");
        // Both VarDecls should LoadConst(0) — only one entry in the pool.
        assert_eq!(constants, vec![Value::Int(5)]);
        assert_eq!(instrs[0], Instruction::LoadConst(0));
        assert_eq!(instrs[2], Instruction::LoadConst(0));
    }

    #[test]
    fn test_compile_load_local() {
        // 整数 Ａ ＝ １０；  整数 Ｂ ＝ Ａ；
        // Second decl must emit LoadLocal(0) to read Ａ.
        let (instrs, _) = compile("整数 Ａ ＝ １０；整数 Ｂ ＝ Ａ；");
        assert_eq!(instrs[2], Instruction::LoadLocal(0)); // Ａ is slot 0
        assert_eq!(instrs[3], Instruction::StoreLocal(1)); // Ｂ is slot 1
    }

    #[test]
    fn test_compile_return() {
        // 関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
        let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝";
        let (instrs, _) = compile(src);
        // Last instruction must be Return.
        assert_eq!(instrs.last().unwrap(), &Instruction::Return);
    }
}

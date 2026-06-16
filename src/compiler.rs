use std::collections::HashMap;

use crate::parser::{BinOpKind, Expr, Stmt};

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
    MakeArray(u16),     // pop n values (in order), push a new Value::Array
    GetIndex,           // pop index, pop array, push the element at index
    SetIndex,           // pop value, pop index, pop array, mutate array in place
    ArrayLen,           // pop array, push its length as Value::Int
    TryStart(u16, u16), // TryStart(catch_target ip, error_var's local slot)
    TryEnd,             // marks successful completion of a try block
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
    synthetic_counter: u32,         // disambiguates ForEach's hidden locals
}

struct Scopes {
    frames: Vec<HashMap<String, u16>>,
    next_slot: u16,
}

impl Scopes {
    fn new() -> Self {
        Self {
            frames: vec![HashMap::new()],
            next_slot: 0,
        }
    }

    fn enter(&mut self) {
        self.frames.push(HashMap::new());
    }

    fn exit(&mut self) {
        self.frames.pop();
    }

    // Reuses the slot only on same-scope redeclaration; a name that exists
    // only in an outer scope gets a fresh slot, so the new binding shadows
    // the outer one without corrupting it.
    fn declare(&mut self, name: &str) -> u16 {
        if let Some(&slot) = self.frames.last().unwrap().get(name) {
            return slot;
        }
        let slot = self.next_slot;
        self.next_slot += 1;
        self.frames
            .last_mut()
            .unwrap()
            .insert(name.to_string(), slot);
        slot
    }

    fn lookup(&self, name: &str) -> Option<u16> {
        self.frames.iter().rev().find_map(|f| f.get(name).copied())
    }
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            chunks: Vec::new(),
            fn_index: HashMap::new(),
            synthetic_counter: 0,
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
        let mut script_scopes = Scopes::new();

        for stmt in stmts {
            match stmt {
                Stmt::FnDecl {
                    name, params, body, ..
                } => {
                    let idx = self.fn_index[name] as usize;
                    let mut fn_scopes = Scopes::new();
                    // Parameters occupy the first local slots in order.
                    for (_, pname) in params {
                        fn_scopes.declare(pname);
                    }
                    let mut fn_instrs = Vec::new();
                    for s in body {
                        self.emit_stmt(s, &mut fn_instrs, &mut fn_scopes);
                    }
                    self.chunks[idx].instructions = fn_instrs;
                }
                other => {
                    self.emit_stmt(other, &mut script_instrs, &mut script_scopes);
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

    fn emit_stmt(&mut self, stmt: &Stmt, instrs: &mut Vec<Instruction>, scopes: &mut Scopes) {
        match stmt {
            Stmt::VarDecl { name, value, .. } => {
                self.emit_expr(value, instrs, scopes);
                let slot = scopes.declare(name);
                instrs.push(Instruction::StoreLocal(slot));
            }
            Stmt::FnDecl { .. } => {
                // Nested fn decls are not yet supported; top-level ones are
                // handled in compile() directly.
            }
            Stmt::Print(expr, _) => {
                self.emit_expr(expr, instrs, scopes);
                instrs.push(Instruction::Print);
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.emit_expr(condition, instrs, scopes);
                // Placeholder index; back-patched after then_body is emitted.
                let jump_if_false_idx = instrs.len();
                instrs.push(Instruction::JumpIfFalse(0));

                scopes.enter();
                for s in then_body {
                    self.emit_stmt(s, instrs, scopes);
                }
                scopes.exit();

                if let Some(else_stmts) = else_body {
                    // Jump over else_body after then_body executes.
                    let jump_idx = instrs.len();
                    instrs.push(Instruction::Jump(0));
                    // Back-patch JumpIfFalse to land here (start of else).
                    let else_start = instrs.len() as u16;
                    instrs[jump_if_false_idx] = Instruction::JumpIfFalse(else_start);

                    scopes.enter();
                    for s in else_stmts {
                        self.emit_stmt(s, instrs, scopes);
                    }
                    scopes.exit();
                    // Back-patch Jump to land after else_body.
                    let after_else = instrs.len() as u16;
                    instrs[jump_idx] = Instruction::Jump(after_else);
                } else {
                    // No else: back-patch JumpIfFalse to land after then_body.
                    let after_then = instrs.len() as u16;
                    instrs[jump_if_false_idx] = Instruction::JumpIfFalse(after_then);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                let loop_start = instrs.len() as u16;
                self.emit_expr(condition, instrs, scopes);
                let jump_if_false_idx = instrs.len();
                instrs.push(Instruction::JumpIfFalse(0));
                scopes.enter();
                for s in body {
                    self.emit_stmt(s, instrs, scopes);
                }
                scopes.exit();
                instrs.push(Instruction::Jump(loop_start));
                let after_loop = instrs.len() as u16;
                instrs[jump_if_false_idx] = Instruction::JumpIfFalse(after_loop);
            }
            Stmt::Return(expr, _) => {
                self.emit_expr(expr, instrs, scopes);
                instrs.push(Instruction::Return);
            }
            Stmt::ExprStmt(expr, _) => {
                self.emit_expr(expr, instrs, scopes);
            }
            Stmt::Assign { name, value, .. } => {
                self.emit_expr(value, instrs, scopes);
                let slot = scopes
                    .lookup(name)
                    .expect("declared name must resolve to a slot (guaranteed by typechecker)");
                instrs.push(Instruction::StoreLocal(slot));
            }
            Stmt::IndexAssign {
                name, index, value, ..
            } => {
                let slot = scopes
                    .lookup(name)
                    .expect("declared name must resolve to a slot (guaranteed by typechecker)");
                instrs.push(Instruction::LoadLocal(slot));
                self.emit_expr(index, instrs, scopes);
                self.emit_expr(value, instrs, scopes);
                instrs.push(Instruction::SetIndex);
            }
            Stmt::ForRange {
                var,
                from,
                to,
                body,
                ..
            } => {
                self.emit_expr(from, instrs, scopes);
                scopes.enter();
                let slot = scopes.declare(var);
                instrs.push(Instruction::StoreLocal(slot));
                let loop_start = instrs.len() as u16;
                instrs.push(Instruction::LoadLocal(slot));
                self.emit_expr(to, instrs, scopes);
                instrs.push(Instruction::LessThan);
                let jif_idx = instrs.len();
                instrs.push(Instruction::JumpIfFalse(0));
                for s in body {
                    self.emit_stmt(s, instrs, scopes);
                }
                instrs.push(Instruction::LoadLocal(slot));
                let one_idx = self.add_constant(Value::Int(1));
                instrs.push(Instruction::LoadConst(one_idx));
                instrs.push(Instruction::Add);
                instrs.push(Instruction::StoreLocal(slot));
                instrs.push(Instruction::Jump(loop_start));
                let after_loop = instrs.len() as u16;
                instrs[jif_idx] = Instruction::JumpIfFalse(after_loop);
                scopes.exit();
            }
            Stmt::ForEach {
                var, array, body, ..
            } => {
                let id = self.synthetic_counter;
                self.synthetic_counter += 1;
                self.emit_expr(array, instrs, scopes);
                scopes.enter();
                let arr_slot = scopes.declare(&format!("__foreach_arr_{}", id));
                instrs.push(Instruction::StoreLocal(arr_slot));
                let idx_slot = scopes.declare(&format!("__foreach_idx_{}", id));
                let zero_idx = self.add_constant(Value::Int(0));
                instrs.push(Instruction::LoadConst(zero_idx));
                instrs.push(Instruction::StoreLocal(idx_slot));
                let loop_start = instrs.len() as u16;
                instrs.push(Instruction::LoadLocal(idx_slot));
                instrs.push(Instruction::LoadLocal(arr_slot));
                instrs.push(Instruction::ArrayLen);
                instrs.push(Instruction::LessThan);
                let jif_idx = instrs.len();
                instrs.push(Instruction::JumpIfFalse(0));
                instrs.push(Instruction::LoadLocal(arr_slot));
                instrs.push(Instruction::LoadLocal(idx_slot));
                instrs.push(Instruction::GetIndex);
                let var_slot = scopes.declare(var);
                instrs.push(Instruction::StoreLocal(var_slot));
                for s in body {
                    self.emit_stmt(s, instrs, scopes);
                }
                instrs.push(Instruction::LoadLocal(idx_slot));
                let one_idx = self.add_constant(Value::Int(1));
                instrs.push(Instruction::LoadConst(one_idx));
                instrs.push(Instruction::Add);
                instrs.push(Instruction::StoreLocal(idx_slot));
                instrs.push(Instruction::Jump(loop_start));
                let after_loop = instrs.len() as u16;
                instrs[jif_idx] = Instruction::JumpIfFalse(after_loop);
                scopes.exit();
            }
            Stmt::TryCatch {
                try_body,
                error_var,
                catch_body,
                ..
            } => {
                let try_start_idx = instrs.len();
                instrs.push(Instruction::TryStart(0, 0)); // placeholder, patched below
                scopes.enter();
                for s in try_body {
                    self.emit_stmt(s, instrs, scopes);
                }
                scopes.exit();
                instrs.push(Instruction::TryEnd);
                let jump_over_catch_idx = instrs.len();
                instrs.push(Instruction::Jump(0)); // placeholder
                let catch_target = instrs.len() as u16;
                scopes.enter();
                let error_slot = scopes.declare(error_var);
                instrs[try_start_idx] = Instruction::TryStart(catch_target, error_slot);
                for s in catch_body {
                    self.emit_stmt(s, instrs, scopes);
                }
                scopes.exit();
                let after_catch = instrs.len() as u16;
                instrs[jump_over_catch_idx] = Instruction::Jump(after_catch);
            }
        }
    }

    fn emit_expr(&mut self, expr: &Expr, instrs: &mut Vec<Instruction>, scopes: &mut Scopes) {
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
                let slot = scopes
                    .lookup(name)
                    .expect("declared name must resolve to a slot (guaranteed by typechecker)");
                instrs.push(Instruction::LoadLocal(slot));
            }
            Expr::BinOp {
                op: BinOpKind::And,
                lhs,
                rhs,
            } => {
                self.emit_expr(lhs, instrs, scopes);
                let jump_if_false_idx = instrs.len();
                instrs.push(Instruction::JumpIfFalse(0));
                self.emit_expr(rhs, instrs, scopes);
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
                self.emit_expr(lhs, instrs, scopes);
                let jump_if_true_idx = instrs.len();
                instrs.push(Instruction::JumpIfTrue(0));
                self.emit_expr(rhs, instrs, scopes);
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
                self.emit_expr(lhs, instrs, scopes);
                self.emit_expr(rhs, instrs, scopes);
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
                self.emit_expr(inner, instrs, scopes);
                instrs.push(Instruction::Negate);
            }
            Expr::UnaryNot(inner) => {
                self.emit_expr(inner, instrs, scopes);
                instrs.push(Instruction::Not);
            }
            Expr::Call { name, args } => {
                // Push arguments left-to-right; the VM seeds locals from them.
                for arg in args {
                    self.emit_expr(arg, instrs, scopes);
                }
                if let Some(builtin) = builtin_name(name) {
                    instrs.push(Instruction::CallBuiltin(builtin, args.len() as u8));
                } else {
                    let fn_idx = self.fn_index[name];
                    instrs.push(Instruction::Call(fn_idx, args.len() as u8));
                }
            }
            Expr::Array(elems) => {
                for elem in elems {
                    self.emit_expr(elem, instrs, scopes);
                }
                instrs.push(Instruction::MakeArray(elems.len() as u16));
            }
            Expr::Index { array, index } => {
                self.emit_expr(array, instrs, scopes);
                self.emit_expr(index, instrs, scopes);
                instrs.push(Instruction::GetIndex);
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

    #[test]
    fn test_compile_array_literal_emits_make_array() {
        let (instrs, constants) = compile("整数列 数字 ＝ 【１、２、３】；");
        assert_eq!(instrs[0], Instruction::LoadConst(0));
        assert_eq!(instrs[1], Instruction::LoadConst(1));
        assert_eq!(instrs[2], Instruction::LoadConst(2));
        assert_eq!(instrs[3], Instruction::MakeArray(3));
        assert_eq!(instrs[4], Instruction::StoreLocal(0));
        assert_eq!(constants, vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    }

    #[test]
    fn test_compile_index_expr_emits_get_index() {
        let (instrs, _) = compile("整数列 数字 ＝ 【１】；返す 数字【０】；");
        assert!(instrs.contains(&Instruction::GetIndex));
    }

    #[test]
    fn test_compile_index_assign_emits_set_index() {
        let (instrs, _) = compile("整数列 数字 ＝ 【１】；数字【０】＝ ２；");
        assert!(instrs.contains(&Instruction::SetIndex));
    }

    #[test]
    fn test_compile_for_range_loop() {
        let (instrs, _) =
            compile("繰り返す カウンタ ＝ ０ から ５ ならば ｛ 印刷（カウンタ）； ｝");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::Jump(_))));
        assert!(
            instrs
                .iter()
                .any(|i| matches!(i, Instruction::JumpIfFalse(_)))
        );
    }

    #[test]
    fn test_compile_for_each_loop_emits_array_len() {
        let src = "整数列 数字 ＝ 【１、２】；各 要素 ： 数字 ならば ｛ 印刷（要素）； ｝";
        let (instrs, _) = compile(src);
        assert!(instrs.contains(&Instruction::ArrayLen));
    }

    #[test]
    fn test_compile_nested_for_each_unique_synthetic_slots() {
        let src = "整数列 Ａ ＝ 【１】；整数列 Ｂ ＝ 【２】；各 要素 ： Ａ ならば ｛ 各 内側 ： Ｂ ならば ｛ 印刷（内側）； ｝ ｝";
        let (instrs, _) = compile(src);
        let array_len_count = instrs
            .iter()
            .filter(|i| matches!(i, Instruction::ArrayLen))
            .count();
        assert_eq!(array_len_count, 2);
    }

    #[test]
    fn test_compile_if_body_shadowing_gets_distinct_slot() {
        let (instrs, _) =
            compile("整数 Ｎ ＝ １０；もし 真 ならば ｛ 整数 Ｎ ＝ ５； ｝整数 結果 ＝ Ｎ；");
        // LoadConst(0)=10, StoreLocal(0)=outer Ｎ
        assert_eq!(instrs[0], Instruction::LoadConst(0));
        assert_eq!(instrs[1], Instruction::StoreLocal(0));
        // Inner Ｎ inside the if-block must get a distinct slot (1), not slot 0.
        assert!(
            instrs
                .iter()
                .any(|i| matches!(i, Instruction::StoreLocal(1)))
        );
        // Final read of outer Ｎ after the if-block must load slot 0, not 1.
        assert!(instrs.contains(&Instruction::LoadLocal(0)));
        assert!(!instrs.contains(&Instruction::LoadLocal(1)));
    }

    #[test]
    fn test_compile_try_catch_emits_try_start_end_and_jump() {
        // layout: [0] TryStart(catch_target, error_slot)
        //         [1] LoadConst(0)=1, [2] Print     ← try_body
        //         [3] TryEnd
        //         [4] Jump(after_catch)
        //         [catch_target=5] LoadConst(1), [6] Print   ← catch_body
        //         [after_catch=7]
        let src = "試す ｛ 印刷（１）； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝";
        let (instrs, _) = compile(src);
        assert!(matches!(instrs[0], Instruction::TryStart(5, 0)));
        assert_eq!(instrs[3], Instruction::TryEnd);
        assert!(matches!(instrs[4], Instruction::Jump(7)));
        assert_eq!(instrs.len(), 7);
    }
}

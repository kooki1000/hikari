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
    // first-class function pointer
    Function {
        chunk_index: usize,
        arity: u8,
    },
}

// ── Built-in functions ────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BuiltinFn {
    Len,           // 文字数
    Input,         // 入力
    ParseInt,      // 整数化
    ParseFloat,    // 小数化
    ToStr,         // 文字列化
    Abs,           // 絶対値
    Sqrt,          // 平方根
    Random,        // 乱数
    Max,           // 最大
    Min,           // 最小
    Split,         // 分割
    Join,          // 結合
    Contains,      // 含む
    Replace,       // 置換
    Pow,           // 累乗
    Floor,         // 切り捨て
    Ceil,          // 切り上げ
    Round,         // 四捨五入
    Rem,           // 余り
    ArrayLen,      // 要素数
    Push,          // 追加
    Pop,           // 取り出す
    ArrayContains, // 含む配列
    IndexOf,       // 位置
    Reverse,       // 逆順
    Sort,          // 整列
    Slice,         // 部分列
    MapKeys,       // 鍵一覧
    MapValues,     // 値一覧
    MapDelete,     // 削除
    // higher-order functions (these are special — they take a fn value)
    MapArray,    // マップ
    FilterArray, // 絞り込み
    FoldArray,   // 畳み込み
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
    Mod,
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
    MakeMap(u16),       // pop 2*n values (key,val pairs), push a new Value::Map
    MakeArray(u16),     // pop n values (in order), push a new Value::Array
    GetIndex,           // pop index, pop array, push the element at index
    SetIndex,           // pop value, pop index, pop array, mutate array in place
    ArrayLen,           // pop array, push its length as Value::Int
    TryStart(u16, u16), // TryStart(catch_target ip, error_var's local slot)
    TryEnd,             // marks successful completion of a try block
    // Field names in the SOURCE order their values were pushed (RecordLit's
    // parsed field order), not necessarily the type's declared field order.
    MakeRecord(Vec<String>),
    GetField(String),             // pop a record, push the named field's value
    SetField(String),             // pop value, pop record, set the named field in place
    MakeEnum(String, String, u8), // MakeEnum(enum_name, variant, payload_count)
    // Pops a Value::Enum, pushes Bool(its variant == the given name). Does
    // NOT consume the value for later payload extraction; callers reload
    // from a local slot if they need the payload after a successful check.
    TagEquals(String),
    GetPayload(u8), // pop a Value::Enum, push payload[index] (clone)
    // push a function value onto the stack
    LoadFn { chunk_index: usize, arity: u8 },
    // Pop function value and arg_count args off the stack, call the function
    CallValue(u8),
}

pub fn builtin_name(name: &str) -> Option<BuiltinFn> {
    match name {
        "文字数" => Some(BuiltinFn::Len),
        "入力" => Some(BuiltinFn::Input),
        "整数化" => Some(BuiltinFn::ParseInt),
        "小数化" => Some(BuiltinFn::ParseFloat),
        "文字列化" => Some(BuiltinFn::ToStr),
        "絶対値" => Some(BuiltinFn::Abs),
        "平方根" => Some(BuiltinFn::Sqrt),
        "乱数" => Some(BuiltinFn::Random),
        "最大" => Some(BuiltinFn::Max),
        "最小" => Some(BuiltinFn::Min),
        "分割" => Some(BuiltinFn::Split),
        "結合" => Some(BuiltinFn::Join),
        "含む" => Some(BuiltinFn::Contains),
        "置換" => Some(BuiltinFn::Replace),
        "累乗" => Some(BuiltinFn::Pow),
        "切り捨て" => Some(BuiltinFn::Floor),
        "切り上げ" => Some(BuiltinFn::Ceil),
        "四捨五入" => Some(BuiltinFn::Round),
        "余り" => Some(BuiltinFn::Rem),
        "要素数" => Some(BuiltinFn::ArrayLen),
        "追加" => Some(BuiltinFn::Push),
        "取り出す" => Some(BuiltinFn::Pop),
        "含む配列" => Some(BuiltinFn::ArrayContains),
        "位置" => Some(BuiltinFn::IndexOf),
        "逆順" => Some(BuiltinFn::Reverse),
        "整列" => Some(BuiltinFn::Sort),
        "部分列" => Some(BuiltinFn::Slice),
        "鍵一覧" => Some(BuiltinFn::MapKeys),
        "値一覧" => Some(BuiltinFn::MapValues),
        "削除" => Some(BuiltinFn::MapDelete),
        "マップ" => Some(BuiltinFn::MapArray),
        "絞り込み" => Some(BuiltinFn::FilterArray),
        "畳み込み" => Some(BuiltinFn::FoldArray),
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
    pub chunks: Vec<Chunk>,                // chunks[0] is the top-level script
    fn_index: HashMap<String, u16>,        // function name → chunk index
    synthetic_counter: u32,                // disambiguates ForEach's hidden locals
    script_scopes: Scopes,                 // persists slots across repeated compile() calls (REPL)
    loop_targets: Vec<LoopTarget>,         // enclosing-loop patch points for 抜ける／続ける
    variant_enum: HashMap<String, String>, // variant name → owning enum name
}

// For While, continue_target is known immediately (loop_start, where the
// condition is re-checked) so 続ける can emit its Jump directly. For
// ForRange/ForEach the increment step is compiled AFTER the body, so its
// offset isn't known while the body (and any 続ける within it) is being
// compiled; jumping to loop_start instead would skip the increment and loop
// forever, so those continues are deferred via continue_jump_idxs and
// back-patched once the increment's start index is known, the same way
// break_jump_idxs is back-patched once after_loop is known.
enum ContinueTarget {
    Known(usize),
    Deferred(Vec<usize>),
}

struct LoopTarget {
    continue_target: ContinueTarget,
    break_jump_idxs: Vec<usize>,
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
            script_scopes: Scopes::new(),
            loop_targets: Vec::new(),
            variant_enum: HashMap::new(),
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
        let mut script_scopes = std::mem::replace(&mut self.script_scopes, Scopes::new());

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

        self.script_scopes = script_scopes;
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
                self.loop_targets.push(LoopTarget {
                    continue_target: ContinueTarget::Known(loop_start as usize),
                    break_jump_idxs: Vec::new(),
                });
                for s in body {
                    self.emit_stmt(s, instrs, scopes);
                }
                scopes.exit();
                instrs.push(Instruction::Jump(loop_start));
                let after_loop = instrs.len() as u16;
                instrs[jump_if_false_idx] = Instruction::JumpIfFalse(after_loop);
                let target = self.loop_targets.pop().expect("pushed above");
                for idx in target.break_jump_idxs {
                    instrs[idx] = Instruction::Jump(after_loop);
                }
            }
            Stmt::Return(expr, _) => {
                if let Some(expr) = expr {
                    self.emit_expr(expr, instrs, scopes);
                }
                instrs.push(Instruction::Return);
            }
            Stmt::Break(_) => {
                let idx = instrs.len();
                instrs.push(Instruction::Jump(0));
                self.loop_targets
                    .last_mut()
                    .expect("guaranteed inside a loop by the typechecker")
                    .break_jump_idxs
                    .push(idx);
            }
            Stmt::Continue(_) => {
                let top = self
                    .loop_targets
                    .last_mut()
                    .expect("guaranteed inside a loop by the typechecker");
                match &mut top.continue_target {
                    ContinueTarget::Known(idx) => {
                        instrs.push(Instruction::Jump(*idx as u16));
                    }
                    ContinueTarget::Deferred(idxs) => {
                        let idx = instrs.len();
                        instrs.push(Instruction::Jump(0));
                        idxs.push(idx);
                    }
                }
            }
            Stmt::Expr(expr, _) => {
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
                self.loop_targets.push(LoopTarget {
                    continue_target: ContinueTarget::Deferred(Vec::new()),
                    break_jump_idxs: Vec::new(),
                });
                for s in body {
                    self.emit_stmt(s, instrs, scopes);
                }
                // 続ける must land HERE, at the increment, not at loop_start:
                // loop_start re-checks the condition (fine in principle), but
                // jumping there directly would also skip this increment, so
                // the loop variable would never advance and the loop would
                // never terminate.
                let increment_start = instrs.len() as u16;
                instrs.push(Instruction::LoadLocal(slot));
                let one_idx = self.add_constant(Value::Int(1));
                instrs.push(Instruction::LoadConst(one_idx));
                instrs.push(Instruction::Add);
                instrs.push(Instruction::StoreLocal(slot));
                instrs.push(Instruction::Jump(loop_start));
                let after_loop = instrs.len() as u16;
                instrs[jif_idx] = Instruction::JumpIfFalse(after_loop);
                let target = self.loop_targets.pop().expect("pushed above");
                for idx in target.break_jump_idxs {
                    instrs[idx] = Instruction::Jump(after_loop);
                }
                if let ContinueTarget::Deferred(idxs) = target.continue_target {
                    for idx in idxs {
                        instrs[idx] = Instruction::Jump(increment_start);
                    }
                }
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
                self.loop_targets.push(LoopTarget {
                    continue_target: ContinueTarget::Deferred(Vec::new()),
                    break_jump_idxs: Vec::new(),
                });
                for s in body {
                    self.emit_stmt(s, instrs, scopes);
                }
                // Same reasoning as ForRange: 続ける must land at the index
                // increment below, not at loop_start, or the index would
                // never advance and the loop would never terminate.
                let increment_start = instrs.len() as u16;
                instrs.push(Instruction::LoadLocal(idx_slot));
                let one_idx = self.add_constant(Value::Int(1));
                instrs.push(Instruction::LoadConst(one_idx));
                instrs.push(Instruction::Add);
                instrs.push(Instruction::StoreLocal(idx_slot));
                instrs.push(Instruction::Jump(loop_start));
                let after_loop = instrs.len() as u16;
                instrs[jif_idx] = Instruction::JumpIfFalse(after_loop);
                let target = self.loop_targets.pop().expect("pushed above");
                for idx in target.break_jump_idxs {
                    instrs[idx] = Instruction::Jump(after_loop);
                }
                if let ContinueTarget::Deferred(idxs) = target.continue_target {
                    for idx in idxs {
                        instrs[idx] = Instruction::Jump(increment_start);
                    }
                }
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
            Stmt::Import { .. } => {
                // No bytecode: 数学/文字列 gating is enforced by the
                // typechecker, and file-based imports are already
                // flattened away before compilation.
            }
            Stmt::TypeDecl { .. } => {
                // No bytecode: purely a typechecker-time declaration.
            }
            Stmt::FieldAssign {
                record,
                field,
                value,
                ..
            } => {
                self.emit_expr(record, instrs, scopes);
                self.emit_expr(value, instrs, scopes);
                instrs.push(Instruction::SetField(field.clone()));
            }
            Stmt::EnumDecl { name, variants, .. } => {
                // No bytecode: registers variant_enum for later codegen,
                // mirroring TypeDecl's "register during compile, no-op
                // codegen" pattern.
                for (variant_name, _) in variants {
                    self.variant_enum.insert(variant_name.clone(), name.clone());
                }
            }
            Stmt::Match {
                subject,
                arms,
                span: _,
            } => {
                // If the subject is a simple identifier, look up its slot and
                // reload it with LoadLocal before each arm — no synthetic needed.
                // Otherwise, emit the expression, store it in a synthetic local,
                // and reload from there (generic fallback).
                let subject_slot: u16 = if let Expr::Ident(name) = subject {
                    scopes.lookup(name).expect("match subject not in scope")
                } else {
                    self.emit_expr(subject, instrs, scopes);
                    let slot =
                        scopes.declare(&format!("__match_subject_{}", self.synthetic_counter));
                    self.synthetic_counter += 1;
                    instrs.push(Instruction::StoreLocal(slot));
                    slot
                };

                let mut end_jump_idxs = Vec::new();
                let mut prev_arm_jif_idx: Option<usize> = None;

                for arm in arms {
                    if let Some(idx) = prev_arm_jif_idx {
                        let here = instrs.len() as u16;
                        instrs[idx] = Instruction::JumpIfFalse(here);
                    }

                    instrs.push(Instruction::LoadLocal(subject_slot));
                    instrs.push(Instruction::TagEquals(arm.variant.clone()));
                    let jif_idx = instrs.len();
                    instrs.push(Instruction::JumpIfFalse(0)); // placeholder; patched by the next arm's start, or left defensively consistent for the last arm

                    scopes.enter();
                    for (i, binder) in arm.binders.iter().enumerate() {
                        instrs.push(Instruction::LoadLocal(subject_slot));
                        instrs.push(Instruction::GetPayload(i as u8));
                        let binder_slot = scopes.declare(binder);
                        instrs.push(Instruction::StoreLocal(binder_slot));
                    }
                    for s in &arm.body {
                        self.emit_stmt(s, instrs, scopes);
                    }
                    scopes.exit();

                    let end_jump_idx = instrs.len();
                    instrs.push(Instruction::Jump(0)); // placeholder, patched once after_match is known
                    end_jump_idxs.push(end_jump_idx);

                    prev_arm_jif_idx = Some(jif_idx);
                }

                let after_match = instrs.len() as u16;
                if let Some(idx) = prev_arm_jif_idx {
                    instrs[idx] = Instruction::JumpIfFalse(after_match);
                }
                for idx in end_jump_idxs {
                    instrs[idx] = Instruction::Jump(after_match);
                }
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
                // if the ident names a local variable, load it.
                // If it names a known function (used as a value), emit LoadFn.
                if let Some(slot) = scopes.lookup(name) {
                    instrs.push(Instruction::LoadLocal(slot));
                } else if let Some(&fn_idx) = self.fn_index.get(name.as_str()) {
                    let arity = self.chunks[fn_idx as usize].param_count;
                    instrs.push(Instruction::LoadFn {
                        chunk_index: fn_idx as usize,
                        arity,
                    });
                } else {
                    panic!(
                        "declared name must resolve to a slot or function (guaranteed by typechecker): {}",
                        name
                    );
                }
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
                    BinOpKind::Mod => Instruction::Mod,
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
                if let Some(owning_enum) = self.variant_enum.get(name).cloned() {
                    instrs.push(Instruction::MakeEnum(
                        owning_enum,
                        name.clone(),
                        args.len() as u8,
                    ));
                } else if let Some(builtin) = builtin_name(name) {
                    instrs.push(Instruction::CallBuiltin(builtin, args.len() as u8));
                } else if let Some(slot) = scopes.lookup(name) {
                    // calling a Fn-typed local variable.
                    instrs.push(Instruction::LoadLocal(slot));
                    instrs.push(Instruction::CallValue(args.len() as u8));
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
            Expr::MapLit(pairs) => {
                for (k, v) in pairs {
                    self.emit_expr(k, instrs, scopes);
                    self.emit_expr(v, instrs, scopes);
                }
                instrs.push(Instruction::MakeMap(pairs.len() as u16));
            }
            Expr::Index { array, index } => {
                self.emit_expr(array, instrs, scopes);
                self.emit_expr(index, instrs, scopes);
                instrs.push(Instruction::GetIndex);
            }
            Expr::NewArray(_) => {
                instrs.push(Instruction::MakeArray(0));
            }
            Expr::RecordLit { fields, .. } => {
                for (_, value) in fields {
                    self.emit_expr(value, instrs, scopes);
                }
                instrs.push(Instruction::MakeRecord(
                    fields.iter().map(|(n, _)| n.clone()).collect(),
                ));
            }
            Expr::FieldAccess { record, field } => {
                self.emit_expr(record, instrs, scopes);
                instrs.push(Instruction::GetField(field.clone()));
            }
            // compile a lambda into a new chunk, emit LoadFn.
            Expr::Lambda { params, body, .. } => {
                // Reserve a chunk slot.
                let chunk_index = self.chunks.len();
                let arity = params.len() as u8;
                self.chunks.push(Chunk {
                    instructions: Vec::new(),
                    param_count: arity,
                });
                // Compile body into a fresh scope.
                let mut fn_scopes = Scopes::new();
                for (pname, _) in params {
                    fn_scopes.declare(pname);
                }
                let mut fn_instrs = Vec::new();
                for s in body {
                    self.emit_stmt(s, &mut fn_instrs, &mut fn_scopes);
                }
                self.chunks[chunk_index].instructions = fn_instrs;
                instrs.push(Instruction::LoadFn { chunk_index, arity });
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
    fn test_compile_stdlib_builtin_emits_call_builtin() {
        let (instrs, _) = compile("取り込む 「数学」；整数 結果 ＝ 絶対値（ー５）；");
        assert!(matches!(
            instrs[2],
            Instruction::CallBuiltin(BuiltinFn::Abs, 1)
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
    fn test_compile_repl_persists_script_slots_across_calls() {
        let ast1 = Parser::new(Lexer::new("整数 値 ＝ １０；").tokenize())
            .parse()
            .unwrap();
        let mut c = Compiler::new();
        let instrs1 = c.compile(&ast1);
        assert_eq!(instrs1[1], Instruction::StoreLocal(0));

        let ast2 = Parser::new(Lexer::new("印刷（値）；").tokenize())
            .parse()
            .unwrap();
        let instrs2 = c.compile(&ast2);
        assert_eq!(instrs2[0], Instruction::LoadLocal(0));
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

    #[test]
    fn test_compile_modulo_emits_mod_instruction() {
        let (instrs, _) = compile("整数 結果 ＝ １０ ％ ３；");
        assert!(instrs.contains(&Instruction::Mod));
    }

    #[test]
    fn test_compile_array_len_builtin_emits_call_builtin() {
        let src = "取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 要素数（数字）；";
        let (instrs, _) = compile(src);
        assert!(instrs.contains(&Instruction::CallBuiltin(BuiltinFn::ArrayLen, 1)));
    }

    #[test]
    fn test_compile_new_array_emits_make_array_zero() {
        let (instrs, _) = compile("整数列 数字 ＝ 新配列＜整数＞；");
        assert_eq!(instrs[0], Instruction::MakeArray(0));
        assert_eq!(instrs[1], Instruction::StoreLocal(0));
    }

    #[test]
    fn test_compile_pow_builtin_emits_call_builtin() {
        let src = "取り込む 「数学」；整数 結果 ＝ 累乗（２、３）；";
        let (instrs, _) = compile(src);
        assert!(instrs.contains(&Instruction::CallBuiltin(BuiltinFn::Pow, 2)));
    }

    // ── 8b: break / continue ─────────────────────────────────────────────

    #[test]
    fn test_compile_while_break_jumps_to_after_loop() {
        // layout: [0] LoadConst(0)=真  [loop_start=1] ... condition is the
        // same constant reload, [jif] JumpIfFalse(after), body: Break, then
        // back-edge Jump(loop_start), [after]
        let src = "間 真 ならば ｛ 抜ける； ｝";
        let (instrs, _) = compile(src);
        let after_loop = instrs.len() as u16;
        let break_jump = instrs
            .iter()
            .find(|i| matches!(i, Instruction::Jump(n) if *n == after_loop));
        assert!(break_jump.is_some());
    }

    #[test]
    fn test_compile_while_continue_jumps_to_loop_start() {
        let src = "間 真 ならば ｛ 続ける； ｝";
        let (instrs, _) = compile(src);
        // loop_start is index 0 (condition re-check starts the loop).
        assert!(instrs.contains(&Instruction::Jump(0)));
    }

    // ── 9a: records ───────────────────────────────────────────────────────

    #[test]
    fn test_compile_record_lit_emits_make_record_in_source_order() {
        let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝点 ｐ ＝ 点 ｛ ｙ：２、ｘ：１ ｝；";
        let (instrs, _) = compile(src);
        assert!(matches!(
            &instrs[2],
            Instruction::MakeRecord(names) if names == &vec!["ｙ".to_string(), "ｘ".to_string()]
        ));
    }

    #[test]
    fn test_compile_field_access_emits_get_field() {
        let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；返す ｐ：：ｘ；";
        let (instrs, _) = compile(src);
        assert!(instrs.contains(&Instruction::GetField("ｘ".to_string())));
    }

    #[test]
    fn test_compile_field_assign_emits_set_field() {
        let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；ｐ：：ｘ ＝ ９；";
        let (instrs, _) = compile(src);
        assert!(instrs.contains(&Instruction::SetField("ｘ".to_string())));
    }

    #[test]
    fn test_compile_type_decl_emits_no_instructions() {
        let src = "型 点 ｛ 整数 ｘ； ｝";
        let (instrs, _) = compile(src);
        assert!(instrs.is_empty());
    }

    // ── 9b: enums and pattern matching ──────────────────────────────────

    #[test]
    fn test_compile_enum_decl_emits_no_instructions() {
        let src = "構造 結果 ｛ 成功（整数） ｝";
        let (instrs, _) = compile(src);
        assert!(instrs.is_empty());
    }

    #[test]
    fn test_compile_variant_construction_emits_make_enum() {
        let src = "構造 結果 ｛ 成功（整数） ｝結果 値 ＝ 成功（１）；";
        let (instrs, _) = compile(src);
        assert!(matches!(
            &instrs[1],
            Instruction::MakeEnum(enum_name, variant, 1)
            if enum_name == "結果" && variant == "成功"
        ));
    }

    #[test]
    fn test_compile_zero_payload_variant_construction_emits_make_enum_zero() {
        let src = "構造 信号 ｛ 赤 ｝信号 値 ＝ 赤（）；";
        let (instrs, _) = compile(src);
        assert!(matches!(
            &instrs[0],
            Instruction::MakeEnum(enum_name, variant, 0)
            if enum_name == "信号" && variant == "赤"
        ));
    }

    #[test]
    fn test_compile_match_two_arms_emits_tag_equals_and_correct_jump_targets() {
        let src = "構造 信号 ｛ 赤、 青 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ 印刷（１）； ｝ 青（） ならば ｛ 印刷（２）； ｝ ｝";
        let (instrs, _) = compile(src);

        let tag_equals_count = instrs
            .iter()
            .filter(|i| matches!(i, Instruction::TagEquals(_)))
            .count();
        assert_eq!(tag_equals_count, 2);
        assert!(matches!(&instrs[3], Instruction::TagEquals(v) if v == "赤"));

        let after_match = instrs.len() as u16;
        // The last arm's JumpIfFalse must land exactly at after_match.
        let last_jif = instrs
            .iter()
            .rev()
            .find(|i| matches!(i, Instruction::JumpIfFalse(_)))
            .unwrap();
        assert_eq!(last_jif, &Instruction::JumpIfFalse(after_match));

        // Every arm's trailing Jump (skip-to-end) must also land at after_match.
        let unconditional_jumps: Vec<&Instruction> = instrs
            .iter()
            .filter(|i| matches!(i, Instruction::Jump(n) if *n == after_match))
            .collect();
        assert_eq!(unconditional_jumps.len(), 2);
    }

    #[test]
    fn test_compile_for_range_continue_targets_increment_not_loop_start() {
        let src = "繰り返す ｉ ＝ ０ から ５ ならば ｛ 続ける； ｝";
        let (instrs, _) = compile(src);
        // The continue's Jump target must be the increment step's LoadLocal,
        // not loop_start (index 1, where the condition re-check begins).
        let continue_jump_target = instrs.iter().find_map(|i| match i {
            Instruction::Jump(n) if *n != 1 => Some(*n),
            _ => None,
        });
        assert!(continue_jump_target.is_some());
        let target = continue_jump_target.unwrap() as usize;
        assert!(matches!(instrs[target], Instruction::LoadLocal(_)));
    }
}

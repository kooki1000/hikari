use std::collections::{HashMap, HashSet};

use crate::lexer::Span;
use crate::parser::{BinOpKind, Expr, Stmt};

use super::builtins::builtin_name;
use super::bytecode::{Chunk, Instruction};
use super::value::Value;

// ── Compiler ──────────────────────────────────────────────────────────────────

pub struct Compiler {
    pub constants: Vec<Value>,
    pub chunks: Vec<Chunk>, // chunks[0] is the top-level script
    // Span checkpoints for the most recent compile()'s script instructions,
    // parallel to its returned Vec<Instruction>. The VM uses these to map a
    // runtime error in top-level code to its source line.
    pub script_spans: Vec<(usize, Span)>,
    fn_index: HashMap<String, u16>, // function name → chunk index
    synthetic_counter: u32,         // disambiguates ForEach's hidden locals
    script_scopes: Scopes,          // persists slots across repeated compile() calls (REPL)
    loop_targets: Vec<LoopTarget>,  // enclosing-loop patch points for 抜ける／続ける
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
            script_spans: Vec::new(),
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
                    spans: Vec::new(),
                });
            }
        }

        // Second pass: compile function bodies and the top-level script.
        let mut script_instrs: Vec<Instruction> = Vec::new();
        let mut script_spans: Vec<(usize, Span)> = Vec::new();
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
                    let mut fn_spans = Vec::new();
                    for s in body {
                        self.emit_stmt(s, &mut fn_instrs, &mut fn_spans, &mut fn_scopes);
                    }
                    self.chunks[idx].instructions = fn_instrs;
                    self.chunks[idx].spans = fn_spans;
                }
                other => {
                    self.emit_stmt(
                        other,
                        &mut script_instrs,
                        &mut script_spans,
                        &mut script_scopes,
                    );
                }
            }
        }

        self.script_scopes = script_scopes;
        self.script_spans = script_spans;
        script_instrs
    }

    /// The source span of a statement (every statement carries one).
    fn stmt_span(stmt: &Stmt) -> Span {
        match stmt {
            Stmt::VarDecl { span, .. }
            | Stmt::FnDecl { span, .. }
            | Stmt::Return(_, span)
            | Stmt::Print(_, span)
            | Stmt::If { span, .. }
            | Stmt::While { span, .. }
            | Stmt::Expr(_, span)
            | Stmt::Assign { span, .. }
            | Stmt::IndexAssign { span, .. }
            | Stmt::ForRange { span, .. }
            | Stmt::ForEach { span, .. }
            | Stmt::TryCatch { span, .. }
            | Stmt::Import { span, .. }
            | Stmt::Break(span)
            | Stmt::Continue(span)
            | Stmt::TypeDecl { span, .. }
            | Stmt::FieldAssign { span, .. }
            | Stmt::EnumDecl { span, .. }
            | Stmt::Match { span, .. } => *span,
        }
    }

    fn add_constant(&mut self, val: Value) -> u16 {
        if let Some(idx) = self.constants.iter().position(|c| c == &val) {
            return idx as u16;
        }
        let idx = self.constants.len() as u16;
        self.constants.push(val);
        idx
    }

    fn emit_stmt(
        &mut self,
        stmt: &Stmt,
        instrs: &mut Vec<Instruction>,
        spans: &mut Vec<(usize, Span)>,
        scopes: &mut Scopes,
    ) {
        // Record a checkpoint so any instruction emitted for this statement
        // (including its sub-expressions) maps back to this source span.
        spans.push((instrs.len(), Self::stmt_span(stmt)));
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
            Stmt::Print(exprs, _) => {
                for expr in exprs {
                    self.emit_expr(expr, instrs, scopes);
                }
                instrs.push(Instruction::PrintLine(exprs.len() as u16));
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
                    self.emit_stmt(s, instrs, spans, scopes);
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
                        self.emit_stmt(s, instrs, spans, scopes);
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
                    self.emit_stmt(s, instrs, spans, scopes);
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
                    self.emit_stmt(s, instrs, spans, scopes);
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
                    self.emit_stmt(s, instrs, spans, scopes);
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
                    self.emit_stmt(s, instrs, spans, scopes);
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
                    self.emit_stmt(s, instrs, spans, scopes);
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
                        self.emit_stmt(s, instrs, spans, scopes);
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
            // Compile a lambda into a new chunk. Variables it references from
            // enclosing scopes are captured by value: their current values are
            // pushed here and a MakeClosure bundles them into the function
            // value. Inside the body, captured names are ordinary locals seeded
            // right after the params, so they read/write via LoadLocal.
            Expr::Lambda { params, body, .. } => {
                // Free variables that resolve to an enclosing local become
                // captures (preserving first-reference order). Names that
                // resolve to a global function or builtin are not captured —
                // the body re-resolves them via LoadFn / CallBuiltin / Call.
                let captures: Vec<(String, u16)> = free_vars(params, body)
                    .into_iter()
                    .filter_map(|name| scopes.lookup(&name).map(|slot| (name, slot)))
                    .collect();

                let chunk_index = self.chunks.len();
                let arity = params.len() as u8;
                self.chunks.push(Chunk {
                    instructions: Vec::new(),
                    param_count: arity,
                    spans: Vec::new(),
                });
                // Params take slots 0..arity; captures take arity..arity+C, so
                // body references resolve as ordinary locals.
                let mut fn_scopes = Scopes::new();
                for (pname, _) in params {
                    fn_scopes.declare(pname);
                }
                for (name, _) in &captures {
                    fn_scopes.declare(name);
                }
                let mut fn_instrs = Vec::new();
                let mut fn_spans = Vec::new();
                for s in body {
                    self.emit_stmt(s, &mut fn_instrs, &mut fn_spans, &mut fn_scopes);
                }
                self.chunks[chunk_index].instructions = fn_instrs;
                self.chunks[chunk_index].spans = fn_spans;

                if captures.is_empty() {
                    instrs.push(Instruction::LoadFn { chunk_index, arity });
                } else {
                    // Push captured values (left-to-right = capture order) then
                    // bundle them into the closure.
                    for (_, slot) in &captures {
                        instrs.push(Instruction::LoadLocal(*slot));
                    }
                    instrs.push(Instruction::MakeClosure {
                        chunk_index,
                        arity,
                        capture_count: captures.len() as u8,
                    });
                }
            }
        }
    }
}

// ── Free-variable analysis (for closure capture) ───────────────────────────────

/// Names referenced inside a lambda but not bound within it (params or any
/// local declared in the body). The caller intersects these with the enclosing
/// scope to decide which to capture. References inside a nested lambda that the
/// nested lambda itself doesn't bind bubble up here, so multi-level capture
/// composes. First-reference order is preserved (it fixes capture-slot order).
fn free_vars(params: &[(String, crate::parser::HikariType)], body: &[Stmt]) -> Vec<String> {
    let mut referenced: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut bound: HashSet<String> = HashSet::new();
    for (p, _) in params {
        bound.insert(p.clone());
    }
    for s in body {
        collect_stmt(s, &mut referenced, &mut seen, &mut bound);
    }
    // `bound` is fully populated by the single pass, so filtering here excludes
    // any name declared anywhere in the body (conservative: a name both used
    // and locally declared is treated as the local, never captured).
    referenced
        .into_iter()
        .filter(|n| !bound.contains(n))
        .collect()
}

fn add_ref(name: &str, referenced: &mut Vec<String>, seen: &mut HashSet<String>) {
    if seen.insert(name.to_string()) {
        referenced.push(name.to_string());
    }
}

fn collect_stmt(
    stmt: &Stmt,
    referenced: &mut Vec<String>,
    seen: &mut HashSet<String>,
    bound: &mut HashSet<String>,
) {
    match stmt {
        Stmt::VarDecl { name, value, .. } => {
            collect_expr(value, referenced, seen, bound);
            bound.insert(name.clone());
        }
        // Named functions inside a lambda body are not compiled (nested fn
        // decls are unsupported); just treat the name as bound.
        Stmt::FnDecl { name, .. } => {
            bound.insert(name.clone());
        }
        Stmt::Return(Some(e), _) | Stmt::Expr(e, _) => collect_expr(e, referenced, seen, bound),
        Stmt::Print(exprs, _) => {
            for e in exprs {
                collect_expr(e, referenced, seen, bound);
            }
        }
        Stmt::Return(None, _) | Stmt::Import { .. } | Stmt::Break(_) | Stmt::Continue(_) => {}
        Stmt::TypeDecl { .. } | Stmt::EnumDecl { .. } => {}
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            collect_expr(condition, referenced, seen, bound);
            for s in then_body {
                collect_stmt(s, referenced, seen, bound);
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    collect_stmt(s, referenced, seen, bound);
                }
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            collect_expr(condition, referenced, seen, bound);
            for s in body {
                collect_stmt(s, referenced, seen, bound);
            }
        }
        Stmt::Assign { name, value, .. } => {
            add_ref(name, referenced, seen);
            collect_expr(value, referenced, seen, bound);
        }
        Stmt::IndexAssign {
            name, index, value, ..
        } => {
            add_ref(name, referenced, seen);
            collect_expr(index, referenced, seen, bound);
            collect_expr(value, referenced, seen, bound);
        }
        Stmt::ForRange {
            var,
            from,
            to,
            body,
            ..
        } => {
            collect_expr(from, referenced, seen, bound);
            collect_expr(to, referenced, seen, bound);
            bound.insert(var.clone());
            for s in body {
                collect_stmt(s, referenced, seen, bound);
            }
        }
        Stmt::ForEach {
            var, array, body, ..
        } => {
            collect_expr(array, referenced, seen, bound);
            bound.insert(var.clone());
            for s in body {
                collect_stmt(s, referenced, seen, bound);
            }
        }
        Stmt::TryCatch {
            try_body,
            error_var,
            catch_body,
            ..
        } => {
            for s in try_body {
                collect_stmt(s, referenced, seen, bound);
            }
            bound.insert(error_var.clone());
            for s in catch_body {
                collect_stmt(s, referenced, seen, bound);
            }
        }
        Stmt::FieldAssign { record, value, .. } => {
            collect_expr(record, referenced, seen, bound);
            collect_expr(value, referenced, seen, bound);
        }
        Stmt::Match { subject, arms, .. } => {
            collect_expr(subject, referenced, seen, bound);
            for arm in arms {
                for binder in &arm.binders {
                    bound.insert(binder.clone());
                }
                for s in &arm.body {
                    collect_stmt(s, referenced, seen, bound);
                }
            }
        }
    }
}

fn collect_expr(
    expr: &Expr,
    referenced: &mut Vec<String>,
    seen: &mut HashSet<String>,
    bound: &mut HashSet<String>,
) {
    match expr {
        Expr::LitInt(_)
        | Expr::LitFloat(_)
        | Expr::LitString(_)
        | Expr::LitBool(_)
        | Expr::NewArray(_) => {}
        Expr::Ident(name) => add_ref(name, referenced, seen),
        Expr::Call { name, args } => {
            // The callee name may be a captured fn-typed local; record it.
            add_ref(name, referenced, seen);
            for a in args {
                collect_expr(a, referenced, seen, bound);
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_expr(lhs, referenced, seen, bound);
            collect_expr(rhs, referenced, seen, bound);
        }
        Expr::UnaryMinus(e) | Expr::UnaryNot(e) | Expr::FieldAccess { record: e, .. } => {
            collect_expr(e, referenced, seen, bound)
        }
        Expr::Array(elems) => {
            for e in elems {
                collect_expr(e, referenced, seen, bound);
            }
        }
        Expr::Index { array, index } => {
            collect_expr(array, referenced, seen, bound);
            collect_expr(index, referenced, seen, bound);
        }
        Expr::MapLit(pairs) => {
            for (k, v) in pairs {
                collect_expr(k, referenced, seen, bound);
                collect_expr(v, referenced, seen, bound);
            }
        }
        Expr::RecordLit { fields, .. } => {
            for (_, v) in fields {
                collect_expr(v, referenced, seen, bound);
            }
        }
        // A nested lambda's own free variables become references of the
        // enclosing lambda (minus what the nested one binds), enabling
        // multi-level capture. We do NOT add the nested lambda's bound names.
        Expr::Lambda { params, body, .. } => {
            for name in free_vars(params, body) {
                add_ref(&name, referenced, seen);
            }
        }
    }
}

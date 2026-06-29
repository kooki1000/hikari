//! Beginner-friendly lint pass over the AST. These are non-fatal warnings
//! surfaced after type checking succeeds; they never reject a program.
//!
//! Lints:
//!   * unused local variable — a `型 名前 ＝ …；` whose value is never read in
//!     its scope (parameters, loop variables, match binders and the 失敗 error
//!     variable are exempt, since leaving those unused is common and benign).
//!   * unreachable code — statements following a `返す`／`抜ける`／`続ける`
//!     within the same block can never execute.
//!   * unused function — a top-level `関数` that is never called anywhere.
//!   * unused import — a `取り込む` stdlib module whose builtins are never used.

use std::collections::HashSet;

use crate::lexer::Span;
use crate::modules::STDLIB_MODULES;
use crate::parser::{Expr, Stmt};
use crate::typechecker::builtin_module;

pub struct Warning {
    pub span: Span,
    pub message: String,
}

/// Lint `stmts` (a whole program) and return any warnings, ordered by source
/// position so output reads top-to-bottom.
pub fn check(stmts: &[Stmt]) -> Vec<Warning> {
    let mut linter = Linter {
        scopes: Vec::new(),
        warnings: Vec::new(),
    };
    linter.body(stmts);
    global_lints(stmts, &mut linter.warnings);
    linter.warnings.sort_by_key(|w| (w.span.line, w.span.col));
    linter.warnings
}

// ── Global lints (require full-program view) ──────────────────────────────────

/// Returns true if `stmts` contains at least one top-level executable statement
/// (anything that is not a declaration). A file with only declarations is
/// treated as a library module, where unused-function warnings are suppressed.
fn has_executable_stmts(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| {
        !matches!(
            s,
            Stmt::FnDecl { .. }
                | Stmt::TypeDecl { .. }
                | Stmt::EnumDecl { .. }
                | Stmt::Import { .. }
        )
    })
}

fn global_lints(stmts: &[Stmt], warnings: &mut Vec<Warning>) {
    // Collect all function names declared at top-level, and all function names
    // that appear in any Call expression anywhere in the program.
    let mut declared_fns: Vec<(String, Span)> = Vec::new();
    let mut called_fns: HashSet<String> = HashSet::new();
    let mut imported_modules: Vec<(String, Span)> = Vec::new();
    let mut used_builtins: HashSet<String> = HashSet::new();

    for stmt in stmts {
        collect_declared(stmt, &mut declared_fns);
        collect_called(stmt, &mut called_fns, &mut used_builtins);
        if let Stmt::Import { name, span, .. } = stmt
            && STDLIB_MODULES.contains(&name.as_str())
        {
            imported_modules.push((name.clone(), *span));
        }
    }

    // Unused functions: declared but never called anywhere (including from other fns).
    // Only applies when the program has top-level executable statements — a file
    // consisting solely of declarations is a library module, where every function
    // is an export and can legitimately go uncalled within the same file.
    if has_executable_stmts(stmts) {
        for (name, span) in &declared_fns {
            // Module-namespaced functions (from aliased imports) are exempt —
            // the source file can't call them directly so we'd always false-positive.
            if !called_fns.contains(name.as_str()) && !name.contains('。') {
                warnings.push(Warning {
                    span: *span,
                    message: format!(
                        "関数「{}」は宣言されていますが、呼び出されていません。",
                        name
                    ),
                });
            }
        }
    }

    // Unused imports: imported stdlib module with no builtins from that module called.
    for (module, span) in &imported_modules {
        let any_used = used_builtins
            .iter()
            .any(|b| builtin_module(b) == Some(module.as_str()));
        if !any_used {
            warnings.push(Warning {
                span: *span,
                message: format!(
                    "「{}」モジュールを取り込んでいますが、そのモジュールの関数は使用されていません。",
                    module
                ),
            });
        }
    }
}

/// Collect every top-level `FnDecl` name (and its span) into `out`.
fn collect_declared(stmt: &Stmt, out: &mut Vec<(String, Span)>) {
    if let Stmt::FnDecl { name, span, .. } = stmt {
        out.push((name.clone(), *span));
    }
}

/// Walk all expressions in `stmt` (recursively into blocks) and collect:
/// - every `Call { name }` into `called`
/// - every `Call` whose name is a builtin into `builtins`
fn collect_called(stmt: &Stmt, called: &mut HashSet<String>, builtins: &mut HashSet<String>) {
    match stmt {
        Stmt::VarDecl { value, .. } | Stmt::Assign { value, .. } => {
            collect_called_expr(value, called, builtins);
        }
        Stmt::IndexAssign { index, value, .. } => {
            collect_called_expr(index, called, builtins);
            collect_called_expr(value, called, builtins);
        }
        Stmt::FieldAssign { record, value, .. } => {
            collect_called_expr(record, called, builtins);
            collect_called_expr(value, called, builtins);
        }
        Stmt::Return(Some(e), _) | Stmt::Expr(e, _) => {
            collect_called_expr(e, called, builtins);
        }
        Stmt::Return(None, _)
        | Stmt::Break(_)
        | Stmt::Continue(_)
        | Stmt::Import { .. }
        | Stmt::TypeDecl { .. }
        | Stmt::EnumDecl { .. } => {}
        Stmt::Print(exprs, _) => {
            for e in exprs {
                collect_called_expr(e, called, builtins);
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            collect_called_expr(condition, called, builtins);
            for s in then_body {
                collect_called(s, called, builtins);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    collect_called(s, called, builtins);
                }
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            collect_called_expr(condition, called, builtins);
            for s in body {
                collect_called(s, called, builtins);
            }
        }
        Stmt::ForRange { from, to, body, .. } => {
            collect_called_expr(from, called, builtins);
            collect_called_expr(to, called, builtins);
            for s in body {
                collect_called(s, called, builtins);
            }
        }
        Stmt::ForEach { array, body, .. } => {
            collect_called_expr(array, called, builtins);
            for s in body {
                collect_called(s, called, builtins);
            }
        }
        Stmt::TryCatch {
            try_body,
            catch_body,
            ..
        } => {
            for s in try_body {
                collect_called(s, called, builtins);
            }
            for s in catch_body {
                collect_called(s, called, builtins);
            }
        }
        Stmt::FnDecl { body, .. } => {
            for s in body {
                collect_called(s, called, builtins);
            }
        }
        Stmt::Match { subject, arms, .. } => {
            collect_called_expr(subject, called, builtins);
            for arm in arms {
                for s in &arm.body {
                    collect_called(s, called, builtins);
                }
            }
        }
    }
}

fn collect_called_expr(expr: &Expr, called: &mut HashSet<String>, builtins: &mut HashSet<String>) {
    match expr {
        Expr::Call { name, args, .. } => {
            called.insert(name.clone());
            if builtin_module(name).is_some() {
                builtins.insert(name.clone());
            }
            for a in args {
                collect_called_expr(a, called, builtins);
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_called_expr(lhs, called, builtins);
            collect_called_expr(rhs, called, builtins);
        }
        Expr::UnaryMinus(e) | Expr::UnaryNot(e) | Expr::FieldAccess { record: e, .. } => {
            collect_called_expr(e, called, builtins);
        }
        Expr::Array(elems) => {
            for e in elems {
                collect_called_expr(e, called, builtins);
            }
        }
        Expr::Index { array, index } => {
            collect_called_expr(array, called, builtins);
            collect_called_expr(index, called, builtins);
        }
        Expr::MapLit(pairs) => {
            for (k, v) in pairs {
                collect_called_expr(k, called, builtins);
                collect_called_expr(v, called, builtins);
            }
        }
        Expr::RecordLit { fields, .. } => {
            for (_, v) in fields {
                collect_called_expr(v, called, builtins);
            }
        }
        Expr::Lambda { body, .. } => {
            for s in body {
                collect_called(s, called, builtins);
            }
        }
        Expr::Question(inner, _) => collect_called_expr(inner, called, builtins),
        Expr::Ident(_)
        | Expr::LitInt(_)
        | Expr::LitFloat(_)
        | Expr::LitString(_)
        | Expr::LitBool(_)
        | Expr::NewArray(_) => {}
    }
}

struct VarInfo {
    name: String,
    span: Span,
    used: bool,
    // Parameters / loop vars / binders are tracked for shadowing resolution but
    // never warned about when unused.
    exempt: bool,
}

struct Linter {
    scopes: Vec<Vec<VarInfo>>,
    warnings: Vec<Warning>,
}

impl Linter {
    fn declare(&mut self, name: &str, span: Span, exempt: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.push(VarInfo {
                name: name.to_string(),
                span,
                used: false,
                exempt,
            });
        }
    }

    // Mark the nearest enclosing binding of `name` as read.
    fn mark_used(&mut self, name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(info) = scope.iter_mut().rev().find(|v| v.name == name) {
                info.used = true;
                return;
            }
        }
    }

    // Walk a block in its own scope: warn on unreachable code, then on any
    // unused (non-exempt) locals declared within.
    fn body(&mut self, stmts: &[Stmt]) {
        self.scoped_body(&[], stmts);
    }

    // Like `body`, but pre-declares `extra` names (exempt) in the new scope —
    // used for loop variables, match binders, and the 失敗 error variable.
    fn scoped_body(&mut self, extra: &[(&str, Span)], stmts: &[Stmt]) {
        self.report_unreachable(stmts);
        self.scopes.push(Vec::new());
        for (name, span) in extra {
            self.declare(name, *span, true);
        }
        for stmt in stmts {
            self.stmt(stmt);
        }
        let scope = self.scopes.pop().unwrap();
        for info in scope {
            if !info.used && !info.exempt {
                self.warnings.push(Warning {
                    span: info.span,
                    message: format!("未使用の変数「{}」です。", info.name),
                });
            }
        }
    }

    // Warn once, at the first statement that follows a terminating statement
    // (返す／抜ける／続ける) in this block.
    fn report_unreachable(&mut self, stmts: &[Stmt]) {
        if let Some(pos) = stmts.iter().position(is_terminator)
            && let Some(next) = stmts.get(pos + 1)
        {
            self.warnings.push(Warning {
                span: stmt_span(next),
                message: "この後の文には到達しません。".to_string(),
            });
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl {
                name, value, span, ..
            } => {
                self.expr(value);
                self.declare(name, *span, false);
            }
            // Reassignment is a write, not a read, so it does not count as use.
            Stmt::Assign { value, .. } => self.expr(value),
            Stmt::IndexAssign {
                name, index, value, ..
            } => {
                // Indexing into a collection reads it.
                self.mark_used(name);
                self.expr(index);
                self.expr(value);
            }
            Stmt::FieldAssign { record, value, .. } => {
                self.expr(record);
                self.expr(value);
            }
            Stmt::Return(Some(e), _) | Stmt::Expr(e, _) => self.expr(e),
            Stmt::Return(None, _)
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::Import { .. }
            | Stmt::TypeDecl { .. }
            | Stmt::EnumDecl { .. } => {}
            Stmt::Print(exprs, _) => {
                for e in exprs {
                    self.expr(e);
                }
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.expr(condition);
                self.body(then_body);
                if let Some(else_body) = else_body {
                    self.body(else_body);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.expr(condition);
                self.body(body);
            }
            Stmt::ForRange {
                var,
                from,
                to,
                body,
                span,
            } => {
                self.expr(from);
                self.expr(to);
                self.scoped_body(&[(var, *span)], body);
            }
            Stmt::ForEach {
                var,
                array,
                body,
                span,
            } => {
                self.expr(array);
                self.scoped_body(&[(var, *span)], body);
            }
            Stmt::TryCatch {
                try_body,
                error_var,
                catch_body,
                span,
            } => {
                self.body(try_body);
                self.scoped_body(&[(error_var, *span)], catch_body);
            }
            // Named function bodies are isolated; lint them in their own scope
            // with the parameters pre-declared (exempt).
            Stmt::FnDecl {
                params, body, span, ..
            } => {
                let extra: Vec<(&str, Span)> =
                    params.iter().map(|(_, n)| (n.as_str(), *span)).collect();
                self.scoped_body(&extra, body);
            }
            Stmt::Match {
                subject,
                arms,
                span,
            } => {
                self.expr(subject);
                for arm in arms {
                    let extra: Vec<(&str, Span)> =
                        arm.binders.iter().map(|n| (n.as_str(), *span)).collect();
                    self.scoped_body(&extra, &arm.body);
                }
            }
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(name) => self.mark_used(name),
            Expr::Call { name, args, .. } => {
                // The callee may be a function-typed local variable.
                self.mark_used(name);
                for a in args {
                    self.expr(a);
                }
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Expr::UnaryMinus(e) | Expr::UnaryNot(e) | Expr::FieldAccess { record: e, .. } => {
                self.expr(e)
            }
            Expr::Array(elems) => {
                for e in elems {
                    self.expr(e);
                }
            }
            Expr::Index { array, index } => {
                self.expr(array);
                self.expr(index);
            }
            Expr::MapLit(pairs) => {
                for (k, v) in pairs {
                    self.expr(k);
                    self.expr(v);
                }
            }
            Expr::RecordLit { fields, .. } => {
                for (_, v) in fields {
                    self.expr(v);
                }
            }
            // A lambda body is its own scope; its parameters are exempt, and
            // reads inside it mark enclosing (captured) variables as used.
            Expr::Lambda { params, body, .. } => {
                let extra: Vec<(&str, Span)> = params
                    .iter()
                    .map(|(n, _)| {
                        (
                            n.as_str(),
                            Span {
                                line: 0,
                                col: 0,
                                len: 0,
                            },
                        )
                    })
                    .collect();
                self.scoped_body(&extra, body);
            }
            Expr::Question(inner, _) => self.expr(inner),
            Expr::LitInt(_)
            | Expr::LitFloat(_)
            | Expr::LitString(_)
            | Expr::LitBool(_)
            | Expr::NewArray(_) => {}
        }
    }
}

fn is_terminator(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Return(..) | Stmt::Break(_) | Stmt::Continue(_))
}

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

#[cfg(test)]
mod tests;

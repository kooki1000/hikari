//! Beginner-friendly lint pass over the AST. These are non-fatal warnings
//! surfaced after type checking succeeds; they never reject a program.
//!
//! Two lints:
//!   * unused local variable — a `型 名前 ＝ …；` whose value is never read in
//!     its scope (parameters, loop variables, match binders and the 失敗 error
//!     variable are exempt, since leaving those unused is common and benign).
//!   * unreachable code — statements following a `返す`／`抜ける`／`続ける`
//!     within the same block can never execute.

use crate::lexer::Span;
use crate::parser::{Expr, Stmt};

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
    linter.warnings.sort_by_key(|w| (w.span.line, w.span.col));
    linter.warnings
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
        if let Some(pos) = stmts.iter().position(is_terminator) {
            if let Some(next) = stmts.get(pos + 1) {
                self.warnings.push(Warning {
                    span: stmt_span(next),
                    message: "この後の文には到達しません。".to_string(),
                });
            }
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
            Expr::Call { name, args } => {
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

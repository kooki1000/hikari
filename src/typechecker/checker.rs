use std::collections::{HashMap, HashSet};

use crate::lexer::Span;
use crate::modules::STDLIB_MODULES;
use crate::parser::{Expr, HikariType, Stmt};

use super::error::{NonExhaustiveMatchInfo, TypeError};
use super::symbols::{FnSig, always_returns};

// 無し() returns Option(Void) as an "unresolved None" sentinel.  It is
// assignment-compatible wherever any Option<T> is expected.
pub(super) fn option_none_compatible(declared: &HikariType, got: &HikariType) -> bool {
    matches!(
        (declared, got),
        (HikariType::Option(_), HikariType::Option(inner)) if inner.as_ref() == &HikariType::Void
    )
}

// Clone lets the REPL snapshot the checker before a line and roll back to it
// if the line fails, so a partially-checked line leaves no half-declared state.
#[derive(Clone)]
pub struct TypeChecker {
    pub(super) scopes: Vec<HashMap<String, HikariType>>,
    pub(super) fns: HashMap<String, FnSig>,
    // Return type expected by the function currently being checked.
    pub(super) current_return_ty: Option<HikariType>,
    pub(super) imported_modules: std::collections::HashSet<String>,
    // Number of enclosing 間／繰り返す／各 bodies; 抜ける／続ける require > 0.
    pub(super) loop_depth: u32,
    // Record type name → ordered (field name, field type) pairs.
    pub(super) records: HashMap<String, Vec<(String, HikariType)>>,
    // Enum name → ordered (variant name, payload types) pairs.
    pub(super) enums: HashMap<String, Vec<(String, Vec<HikariType>)>>,
    // Variant name → owning enum name (variant names are globally unique).
    pub(super) variant_owner: HashMap<String, String>,
    // Type variable names currently in scope (populated when checking a generic fn body).
    pub(super) type_var_names: HashSet<String>,
    // Mangled names of module-private functions (non-公開 functions from aliased imports).
    // Calls to these from outside their own module body are rejected.
    pub(super) private_fns: HashSet<String>,
    // The mangled name of the function whose body is currently being checked.
    // Used to allow intra-module private calls (alias。fn calling alias。helper).
    pub(super) current_fn_name: Option<String>,
    // Spans of 総和 calls whose array argument has a 小数 element type. The
    // compiler reads this immediately after checking the same AST to lower
    // those calls to a float-aware sum (so an empty 小数列 yields 0.0, not the
    // integer 0). Keyed by Expr::Call's own span; cleared at the start of every
    // check so a previous REPL line's sites can't leak into the next.
    //
    // Spans carry no file identity, so in principle two 総和 calls at the exact
    // same line:col:len in different imported files could collide after import
    // merging. That is harmless unless one sums 整数 and the other 小数; a future
    // file-aware Span would close the gap entirely.
    pub(super) float_sum_sites: HashSet<Span>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            fns: HashMap::new(),
            current_return_ty: None,
            imported_modules: std::collections::HashSet::new(),
            loop_depth: 0,
            records: HashMap::new(),
            enums: HashMap::new(),
            variant_owner: HashMap::new(),
            type_var_names: HashSet::new(),
            private_fns: HashSet::new(),
            current_fn_name: None,
            float_sum_sites: HashSet::new(),
        }
    }

    /// Take the set of float-element 総和 call-node identities collected during
    /// the most recent `check`/`check_all`, for the compiler to consult while
    /// lowering the *same* AST. Draining it transfers ownership and leaves the
    /// checker's set empty for the next line.
    pub fn take_float_sum_sites(&mut self) -> HashSet<Span> {
        std::mem::take(&mut self.float_sum_sites)
    }

    pub(super) fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn declare_var(&mut self, name: &str, ty: HikariType) {
        self.scopes.last_mut().unwrap().insert(name.to_string(), ty);
    }

    pub(super) fn lookup_var(&self, name: &str) -> Option<HikariType> {
        self.scopes.iter().rev().find_map(|s| s.get(name).cloned())
    }

    // parse_type's Ident arm accepts ANY identifier as a syntactically valid
    // type, even ones that aren't declared record types, so this is the
    // gatekeeper that rejects undeclared record names wherever a type is
    // actually used (VarDecl, function params/return type).
    pub(super) fn check_type_declared(&self, ty: &HikariType, span: Span) -> Result<(), TypeError> {
        match ty {
            HikariType::Record(name)
                if !self.records.contains_key(name)
                    && !self.enums.contains_key(name)
                    && !self.type_var_names.contains(name) =>
            {
                Err(TypeError::UndeclaredType(name.clone(), span))
            }
            HikariType::Map(k, v) => {
                self.check_type_declared(k, span)?;
                self.check_type_declared(v, span)
            }
            HikariType::Array(inner) => self.check_type_declared(inner, span),
            HikariType::Option(inner) => self.check_type_declared(inner, span),
            HikariType::Fn(params, ret) => {
                for p in params {
                    self.check_type_declared(p, span)?;
                }
                self.check_type_declared(ret, span)
            }
            _ => Ok(()),
        }
    }

    // Infer the type of an expression used in value position, rejecting 無.
    // A 無-returning call (or other 無-typed expression) produces no value, so
    // using it as an 印刷 argument, 追加 element, binary operand, 返す value,
    // condition, etc. is a TypeError rather than a runtime StackUnderflow.
    // Statement-position calls (Stmt::Expr) bypass this and stay legal.
    pub(super) fn infer_value_expr(
        &mut self,
        expr: &Expr,
        span: Span,
    ) -> Result<HikariType, TypeError> {
        let ty = self.infer_expr(expr, span)?;
        if ty == HikariType::Void {
            return Err(TypeError::VoidValueUsed { span });
        }
        Ok(ty)
    }

    pub fn check(&mut self, stmts: &[Stmt]) -> Result<(), TypeError> {
        self.float_sum_sites.clear();
        // Pre-pass: register all top-level FnDecl signatures so that
        // forward references and mutual recursion both type-check, and so
        // that private-fn guards know about every module-private function
        // before the first body is checked.
        for stmt in stmts {
            if let Stmt::FnDecl {
                name,
                type_params,
                params,
                return_ty,
                is_public,
                ..
            } = stmt
            {
                let sig = super::symbols::FnSig {
                    params: params.iter().map(|(t, _)| t.clone()).collect(),
                    return_ty: return_ty.clone(),
                    type_params: type_params.clone(),
                };
                self.fns.insert(name.clone(), sig);
                if !is_public && name.contains('。') {
                    self.private_fns.insert(name.clone());
                }
            }
        }
        for stmt in stmts {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    /// Like `check`, but collects **all** statement-level errors instead of
    /// stopping at the first. Best-effort: a failed statement may leave the
    /// checker in an inconsistent state, so some subsequent errors may be
    /// cascades. Returns the error list (empty = no errors).
    pub fn check_all(&mut self, stmts: &[Stmt]) -> Vec<TypeError> {
        self.float_sum_sites.clear();
        // Same pre-pass as `check`.
        for stmt in stmts {
            if let Stmt::FnDecl {
                name,
                type_params,
                params,
                return_ty,
                is_public,
                ..
            } = stmt
            {
                let sig = super::symbols::FnSig {
                    params: params.iter().map(|(t, _)| t.clone()).collect(),
                    return_ty: return_ty.clone(),
                    type_params: type_params.clone(),
                };
                self.fns.insert(name.clone(), sig);
                if !is_public && name.contains('。') {
                    self.private_fns.insert(name.clone());
                }
            }
        }
        let mut errors = Vec::new();
        for stmt in stmts {
            if let Err(e) = self.check_stmt(stmt) {
                errors.push(e);
            }
        }
        errors
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        match stmt {
            Stmt::VarDecl {
                ty,
                name,
                value,
                span,
            } => {
                self.check_type_declared(ty, *span)?;
                // Special case: an empty array literal 【】 is valid if the
                // declared type is an Array — infer_expr cannot deduce element
                // type from an empty literal, so trust the annotation.
                if matches!(value, Expr::Array(elems) if elems.is_empty()) {
                    if !matches!(ty, HikariType::Array(_)) {
                        return Err(TypeError::VarDeclMismatch {
                            name: name.clone(),
                            declared: ty.clone(),
                            got: HikariType::Array(Box::new(HikariType::Void)),
                            span: *span,
                        });
                    }
                    self.declare_var(name, ty.clone());
                    return Ok(());
                }
                // Special case: an empty map literal ｛｝ is valid if the declared
                // type is a Map — infer_expr can't deduce the element types from nothing,
                // so we skip the inference and trust the annotation.
                if matches!(value, Expr::MapLit(pairs) if pairs.is_empty()) {
                    if !matches!(ty, HikariType::Map(..)) {
                        return Err(TypeError::VarDeclMismatch {
                            name: name.clone(),
                            declared: ty.clone(),
                            got: HikariType::Map(
                                Box::new(HikariType::String),
                                Box::new(HikariType::Void),
                            ),
                            span: *span,
                        });
                    }
                    self.declare_var(name, ty.clone());
                    return Ok(());
                }
                let inferred = self.infer_value_expr(value, *span)?;
                if inferred != *ty && !option_none_compatible(ty, &inferred) {
                    return Err(TypeError::VarDeclMismatch {
                        name: name.clone(),
                        declared: ty.clone(),
                        got: inferred,
                        span: *span,
                    });
                }
                self.declare_var(name, ty.clone());
                Ok(())
            }

            Stmt::FnDecl {
                name,
                type_params,
                params,
                return_ty,
                body,
                is_public,
                span,
            } => {
                // Bring type-var names into scope before checking declared types
                // so that e.g. `配列＜Ｔ＞` in a param type doesn't fail.
                let outer_type_var_names = std::mem::replace(
                    &mut self.type_var_names,
                    type_params.iter().cloned().collect(),
                );

                for (ty, _) in params {
                    self.check_type_declared(ty, *span)?;
                }
                self.check_type_declared(return_ty, *span)?;
                let sig = FnSig {
                    params: params.iter().map(|(t, _)| t.clone()).collect(),
                    return_ty: return_ty.clone(),
                    type_params: type_params.clone(),
                };
                self.fns.insert(name.clone(), sig);

                // Functions from aliased imports that are not 公開 are private:
                // they can be called internally (their body references the mangled
                // names of other module functions) but not from outside the module.
                if !is_public && name.contains('。') {
                    self.private_fns.insert(name.clone());
                }

                // Function bodies are fully isolated: they get a brand new
                // scope stack, matching the VM's independent per-call Frame.
                let outer_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);
                let outer_return_ty = self.current_return_ty.take();
                let outer_fn_name = self.current_fn_name.replace(name.clone());
                // A 抜ける／続ける written inside a nested 関数 body must not
                // be considered "inside a loop" just because the call site
                // (or even the declaration site) happens to sit inside one.
                let outer_loop_depth = std::mem::take(&mut self.loop_depth);

                for (ty, pname) in params {
                    self.declare_var(pname, ty.clone());
                }
                self.current_return_ty = Some(return_ty.clone());

                self.check(body)?;

                if *return_ty != HikariType::Void && !always_returns(body) {
                    return Err(TypeError::MissingReturn {
                        name: name.clone(),
                        span: *span,
                    });
                }

                self.scopes = outer_scopes;
                self.current_return_ty = outer_return_ty;
                self.current_fn_name = outer_fn_name;
                self.loop_depth = outer_loop_depth;
                self.type_var_names = outer_type_var_names;
                Ok(())
            }

            Stmt::Return(expr, span) => {
                match expr {
                    Some(expr) => {
                        let got = self.infer_value_expr(expr, *span)?;
                        if let Some(expected) = &self.current_return_ty
                            && got != *expected
                            && !option_none_compatible(expected, &got)
                        {
                            return Err(TypeError::ReturnTypeMismatch {
                                expected: expected.clone(),
                                got,
                                span: *span,
                            });
                        }
                    }
                    None => {
                        if let Some(expected) = &self.current_return_ty
                            && *expected != HikariType::Void
                        {
                            return Err(TypeError::ReturnTypeMismatch {
                                expected: expected.clone(),
                                got: HikariType::Void,
                                span: *span,
                            });
                        }
                    }
                }
                Ok(())
            }

            Stmt::Break(span) => {
                if self.loop_depth == 0 {
                    return Err(TypeError::ControlFlowOutsideLoop {
                        keyword: "抜ける".to_string(),
                        span: *span,
                    });
                }
                Ok(())
            }

            Stmt::Continue(span) => {
                if self.loop_depth == 0 {
                    return Err(TypeError::ControlFlowOutsideLoop {
                        keyword: "続ける".to_string(),
                        span: *span,
                    });
                }
                Ok(())
            }

            Stmt::Print(exprs, span) => {
                // Each value may be of any (non-無) type; infer_value_expr
                // rejects 無-typed values used in value position.
                for expr in exprs {
                    self.infer_value_expr(expr, *span)?;
                }
                Ok(())
            }

            Stmt::If {
                condition,
                then_body,
                else_body,
                span,
            } => {
                let cond_ty = self.infer_value_expr(condition, *span)?;
                if cond_ty != HikariType::Bool {
                    return Err(TypeError::ConditionNotBool(cond_ty, *span));
                }
                self.enter_scope();
                self.check(then_body)?;
                self.exit_scope();
                if let Some(body) = else_body {
                    self.enter_scope();
                    self.check(body)?;
                    self.exit_scope();
                }
                Ok(())
            }

            Stmt::While {
                condition,
                body,
                span,
            } => {
                let cond_ty = self.infer_value_expr(condition, *span)?;
                if cond_ty != HikariType::Bool {
                    return Err(TypeError::ConditionNotBool(cond_ty, *span));
                }
                self.enter_scope();
                self.loop_depth += 1;
                self.check(body)?;
                self.loop_depth -= 1;
                self.exit_scope();
                Ok(())
            }

            Stmt::Expr(expr, span) => {
                // Statement-position expressions may be 無-typed: calling a
                // 無-returning function purely for its side effects (印刷,
                // 追加, etc.) is legal, so this site uses infer_expr directly
                // rather than infer_value_expr.
                self.infer_expr(expr, *span)?;
                Ok(())
            }

            Stmt::Assign { name, value, span } => {
                let declared = self
                    .lookup_var(name)
                    .ok_or_else(|| TypeError::UndeclaredVariable(name.clone(), *span))?;
                let got = self.infer_value_expr(value, *span)?;
                if got != declared {
                    return Err(TypeError::VarDeclMismatch {
                        name: name.clone(),
                        declared,
                        got,
                        span: *span,
                    });
                }
                Ok(())
            }

            Stmt::IndexAssign {
                name,
                index,
                value,
                span,
            } => {
                let var_ty = self
                    .lookup_var(name)
                    .ok_or_else(|| TypeError::UndeclaredVariable(name.clone(), *span))?;
                match var_ty {
                    HikariType::Array(elem_ty) => {
                        let index_ty = self.infer_value_expr(index, *span)?;
                        if index_ty != HikariType::Int {
                            return Err(TypeError::IndexNotInt {
                                got: index_ty,
                                span: *span,
                            });
                        }
                        let value_ty = self.infer_value_expr(value, *span)?;
                        if value_ty != *elem_ty {
                            return Err(TypeError::ArrayElementTypeMismatch {
                                expected: *elem_ty,
                                got: value_ty,
                                span: *span,
                            });
                        }
                    }
                    HikariType::Map(key_ty, val_ty) => {
                        let index_ty = self.infer_value_expr(index, *span)?;
                        if index_ty != *key_ty {
                            return Err(TypeError::IndexNotInt {
                                got: index_ty,
                                span: *span,
                            });
                        }
                        let value_ty = self.infer_value_expr(value, *span)?;
                        if value_ty != *val_ty {
                            return Err(TypeError::ArrayElementTypeMismatch {
                                expected: *val_ty,
                                got: value_ty,
                                span: *span,
                            });
                        }
                    }
                    other => {
                        return Err(TypeError::NotIndexable {
                            got: other,
                            span: *span,
                        });
                    }
                }
                Ok(())
            }

            Stmt::ForRange {
                var,
                from,
                to,
                body,
                span,
            } => {
                let from_ty = self.infer_value_expr(from, *span)?;
                let to_ty = self.infer_value_expr(to, *span)?;
                if from_ty != HikariType::Int {
                    return Err(TypeError::ArgTypeMismatch {
                        name: var.clone(),
                        param: HikariType::Int,
                        got: from_ty,
                        span: *span,
                    });
                }
                if to_ty != HikariType::Int {
                    return Err(TypeError::ArgTypeMismatch {
                        name: var.clone(),
                        param: HikariType::Int,
                        got: to_ty,
                        span: *span,
                    });
                }
                self.enter_scope();
                self.declare_var(var, HikariType::Int);
                self.loop_depth += 1;
                self.check(body)?;
                self.loop_depth -= 1;
                self.exit_scope();
                Ok(())
            }

            Stmt::ForEach {
                var,
                array,
                body,
                span,
            } => {
                let array_ty = self.infer_value_expr(array, *span)?;
                let elem_ty = match array_ty {
                    HikariType::Array(inner) => *inner,
                    other => {
                        return Err(TypeError::NotIndexable {
                            got: other,
                            span: *span,
                        });
                    }
                };
                self.enter_scope();
                self.declare_var(var, elem_ty);
                self.loop_depth += 1;
                self.check(body)?;
                self.loop_depth -= 1;
                self.exit_scope();
                Ok(())
            }

            Stmt::TryCatch {
                try_body,
                error_var,
                catch_body,
                ..
            } => {
                self.enter_scope();
                self.check(try_body)?;
                self.exit_scope();

                self.enter_scope();
                self.declare_var(error_var, HikariType::String);
                self.check(catch_body)?;
                self.exit_scope();
                Ok(())
            }

            Stmt::Import { name, .. } => {
                if STDLIB_MODULES.contains(&name.as_str()) {
                    self.imported_modules.insert(name.clone());
                }
                Ok(())
            }

            Stmt::TypeDecl { name, fields, .. } => {
                let entry = fields.iter().map(|(t, n)| (n.clone(), t.clone())).collect();
                self.records.insert(name.clone(), entry);
                Ok(())
            }

            Stmt::FieldAssign {
                record,
                field,
                value,
                span,
            } => {
                let record_ty = self.infer_value_expr(record, *span)?;
                let type_name = match record_ty {
                    HikariType::Record(name) => name,
                    other => {
                        return Err(TypeError::NotARecord {
                            got: other,
                            span: *span,
                        });
                    }
                };
                let field_ty = self
                    .records
                    .get(&type_name)
                    .and_then(|fs| fs.iter().find(|(n, _)| n == field).map(|(_, t)| t.clone()))
                    .ok_or_else(|| TypeError::UnknownField {
                        type_name: type_name.clone(),
                        field: field.clone(),
                        span: *span,
                    })?;
                let value_ty = self.infer_value_expr(value, *span)?;
                if value_ty != field_ty {
                    return Err(TypeError::FieldTypeMismatch {
                        type_name,
                        field: field.clone(),
                        expected: Box::new(field_ty),
                        got: Box::new(value_ty),
                        span: *span,
                    });
                }
                Ok(())
            }

            Stmt::EnumDecl {
                name,
                variants,
                span,
            } => {
                for (variant_name, _) in variants {
                    if self.variant_owner.contains_key(variant_name) {
                        return Err(TypeError::DuplicateEnumVariant {
                            variant: variant_name.clone(),
                            span: *span,
                        });
                    }
                    self.variant_owner
                        .insert(variant_name.clone(), name.clone());
                }
                self.enums.insert(name.clone(), variants.clone());
                Ok(())
            }

            Stmt::Match {
                subject,
                arms,
                span,
            } => {
                let subject_ty = self.infer_value_expr(subject, *span)?;

                // Built-in 省略可＜T＞ match: 有る(binder) and 無し().
                if let HikariType::Option(inner_ty) = &subject_ty {
                    let inner_ty = *inner_ty.clone();
                    let mut covered: HashSet<String> = HashSet::new();
                    for arm in arms {
                        if !covered.insert(arm.variant.clone()) {
                            return Err(TypeError::DuplicateMatchArm {
                                variant: arm.variant.clone(),
                                span: *span,
                            });
                        }
                        match arm.variant.as_str() {
                            "有る" => {
                                if arm.binders.len() != 1 {
                                    return Err(TypeError::ArgCountMismatch {
                                        name: "有る".to_string(),
                                        expected: 1,
                                        got: arm.binders.len(),
                                        span: *span,
                                    });
                                }
                                self.enter_scope();
                                self.declare_var(&arm.binders[0], inner_ty.clone());
                                self.check(&arm.body)?;
                                self.exit_scope();
                            }
                            "無し" => {
                                if !arm.binders.is_empty() {
                                    return Err(TypeError::ArgCountMismatch {
                                        name: "無し".to_string(),
                                        expected: 0,
                                        got: arm.binders.len(),
                                        span: *span,
                                    });
                                }
                                self.enter_scope();
                                self.check(&arm.body)?;
                                self.exit_scope();
                            }
                            other => {
                                return Err(TypeError::UndeclaredEnumVariant {
                                    enum_name: "省略可".to_string(),
                                    variant: other.to_string(),
                                    span: *span,
                                });
                            }
                        }
                    }
                    let missing: Vec<String> = ["有る", "無し"]
                        .iter()
                        .filter(|v| !covered.contains(**v))
                        .map(|v| v.to_string())
                        .collect();
                    if !missing.is_empty() {
                        return Err(TypeError::NonExhaustiveMatch(Box::new(
                            NonExhaustiveMatchInfo {
                                enum_name: "省略可".to_string(),
                                missing,
                                span: *span,
                            },
                        )));
                    }
                    return Ok(());
                }

                // Enum-typed variables are stored as Record(enum_name) in the
                // type system (parse_type maps any bare Ident to Record), so
                // we accept Record(name) when name is a registered enum, as
                // well as the explicit Enum(name) form.
                let enum_name = match subject_ty {
                    HikariType::Record(name) if self.enums.contains_key(&name) => name,
                    other => {
                        return Err(TypeError::NotAnEnum {
                            got: other,
                            span: *span,
                        });
                    }
                };
                // Guaranteed present: subject already typechecked to
                // Enum(enum_name) or Record(enum_name), so enum_name must be registered.
                let declared_variants = self
                    .enums
                    .get(&enum_name)
                    .expect("enum registered by EnumDecl before any value of its type exists")
                    .clone();

                let mut covered: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for arm in arms {
                    if !covered.insert(arm.variant.clone()) {
                        return Err(TypeError::DuplicateMatchArm {
                            variant: arm.variant.clone(),
                            span: *span,
                        });
                    }

                    let payload_types = declared_variants
                        .iter()
                        .find(|(n, _)| n == &arm.variant)
                        .map(|(_, tys)| tys.clone())
                        .ok_or_else(|| TypeError::UndeclaredEnumVariant {
                            enum_name: enum_name.clone(),
                            variant: arm.variant.clone(),
                            span: *span,
                        })?;

                    if arm.binders.len() != payload_types.len() {
                        return Err(TypeError::ArgCountMismatch {
                            name: arm.variant.clone(),
                            expected: payload_types.len(),
                            got: arm.binders.len(),
                            span: *span,
                        });
                    }

                    self.enter_scope();
                    for (binder, ty) in arm.binders.iter().zip(payload_types.iter()) {
                        self.declare_var(binder, ty.clone());
                    }
                    self.check(&arm.body)?;
                    self.exit_scope();
                }

                let missing: Vec<String> = declared_variants
                    .iter()
                    .map(|(n, _)| n.clone())
                    .filter(|n| !covered.contains(n))
                    .collect();
                if !missing.is_empty() {
                    return Err(TypeError::NonExhaustiveMatch(Box::new(
                        NonExhaustiveMatchInfo {
                            enum_name,
                            missing,
                            span: *span,
                        },
                    )));
                }

                Ok(())
            }
        }
    }
}

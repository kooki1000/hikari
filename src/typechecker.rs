use std::collections::HashMap;

use crate::lexer::Span;
use crate::parser::{BinOpKind, Expr, HikariType, Stmt, hikari_type_japanese};

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum TypeError {
    // Declared type does not match the inferred type of the initialiser.
    VarDeclMismatch {
        name: String,
        declared: HikariType,
        got: HikariType,
        span: Span,
    },
    // Both sides of a binary operator must share a type.
    BinOpMismatch {
        op: BinOpKind,
        lhs: HikariType,
        rhs: HikariType,
        span: Span,
    },
    // Variable referenced before declaration.
    UndeclaredVariable(String, Span),
    // Return expression type differs from the function's declared return type.
    ReturnTypeMismatch {
        expected: HikariType,
        got: HikariType,
        span: Span,
    },
    // Call to an undeclared function.
    UndeclaredFunction(String, Span),
    // Wrong number of arguments at a call site.
    ArgCountMismatch {
        name: String,
        expected: usize,
        got: usize,
        span: Span,
    },
    // Argument type does not match the parameter type.
    ArgTypeMismatch {
        name: String,
        param: HikariType,
        got: HikariType,
        span: Span,
    },
    // Condition in もし/間 is not Bool.
    ConditionNotBool(HikariType, Span),
}

impl TypeError {
    pub fn span(&self) -> Span {
        match self {
            TypeError::VarDeclMismatch { span, .. } => *span,
            TypeError::BinOpMismatch { span, .. } => *span,
            TypeError::UndeclaredVariable(_, span) => *span,
            TypeError::ReturnTypeMismatch { span, .. } => *span,
            TypeError::UndeclaredFunction(_, span) => *span,
            TypeError::ArgCountMismatch { span, .. } => *span,
            TypeError::ArgTypeMismatch { span, .. } => *span,
            TypeError::ConditionNotBool(_, span) => *span,
        }
    }
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::VarDeclMismatch { name, declared, got, .. } => write!(
                f,
                "変数「{}」の型が一致しません: 「{}」として宣言されましたが、「{}」の値が代入されました。",
                name,
                hikari_type_japanese(declared),
                hikari_type_japanese(got)
            ),
            TypeError::BinOpMismatch { lhs, rhs, .. } => write!(
                f,
                "演算子の両辺の型が一致しません: 「{}」と「{}」は一緒に演算できません。",
                hikari_type_japanese(lhs),
                hikari_type_japanese(rhs)
            ),
            TypeError::UndeclaredVariable(name, _) => write!(
                f,
                "変数「{}」は宣言されていません。（ヒント: 使用する前に型と一緒に宣言してください）",
                name
            ),
            TypeError::ReturnTypeMismatch { expected, got, .. } => write!(
                f,
                "戻り値の型が一致しません: 「{}」を返す必要がありますが、「{}」が返されました。",
                hikari_type_japanese(expected),
                hikari_type_japanese(got)
            ),
            TypeError::UndeclaredFunction(name, _) => {
                write!(f, "関数「{}」は宣言されていません。", name)
            }
            TypeError::ArgCountMismatch { name, expected, got, .. } => write!(
                f,
                "関数「{}」の引数の数が一致しません: {}個必要ですが、{}個指定されました。",
                name, expected, got
            ),
            TypeError::ArgTypeMismatch { name, param, got, .. } => write!(
                f,
                "関数「{}」の引数の型が一致しません: 「{}」が必要ですが、「{}」が渡されました。",
                name,
                hikari_type_japanese(param),
                hikari_type_japanese(got)
            ),
            TypeError::ConditionNotBool(got, _) => write!(
                f,
                "条件式は「真偽」型である必要がありますが、「{}」が指定されました。",
                hikari_type_japanese(got)
            ),
        }
    }
}

// ── Symbol tables ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct FnSig {
    params: Vec<HikariType>,
    return_ty: HikariType,
}

pub struct TypeChecker {
    vars: HashMap<String, HikariType>,
    fns: HashMap<String, FnSig>,
    // Return type expected by the function currently being checked.
    current_return_ty: Option<HikariType>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            fns: HashMap::new(),
            current_return_ty: None,
        }
    }

    pub fn check(&mut self, stmts: &[Stmt]) -> Result<(), TypeError> {
        for stmt in stmts {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        match stmt {
            Stmt::VarDecl { ty, name, value, span } => {
                let inferred = self.infer_expr(value, *span)?;
                if inferred != *ty {
                    return Err(TypeError::VarDeclMismatch {
                        name: name.clone(),
                        declared: ty.clone(),
                        got: inferred,
                        span: *span,
                    });
                }
                self.vars.insert(name.clone(), ty.clone());
                Ok(())
            }

            Stmt::FnDecl {
                name,
                params,
                return_ty,
                body,
                ..
            } => {
                let sig = FnSig {
                    params: params.iter().map(|(t, _)| t.clone()).collect(),
                    return_ty: return_ty.clone(),
                };
                self.fns.insert(name.clone(), sig);

                // Enter function scope: save outer state.
                let outer_vars = self.vars.clone();
                let outer_return_ty = self.current_return_ty.take();

                for (ty, pname) in params {
                    self.vars.insert(pname.clone(), ty.clone());
                }
                self.current_return_ty = Some(return_ty.clone());

                self.check(body)?;

                // Restore outer scope.
                self.vars = outer_vars;
                self.current_return_ty = outer_return_ty;
                Ok(())
            }

            Stmt::Return(expr, span) => {
                let got = self.infer_expr(expr, *span)?;
                if let Some(expected) = &self.current_return_ty {
                    if got != *expected {
                        return Err(TypeError::ReturnTypeMismatch {
                            expected: expected.clone(),
                            got,
                            span: *span,
                        });
                    }
                }
                Ok(())
            }

            Stmt::Print(expr, span) => {
                self.infer_expr(expr, *span)?;
                Ok(())
            }

            Stmt::If {
                condition,
                then_body,
                else_body,
                span,
            } => {
                let cond_ty = self.infer_expr(condition, *span)?;
                if cond_ty != HikariType::Bool {
                    return Err(TypeError::ConditionNotBool(cond_ty, *span));
                }
                self.check(then_body)?;
                if let Some(body) = else_body {
                    self.check(body)?;
                }
                Ok(())
            }

            Stmt::While { condition, body, span } => {
                let cond_ty = self.infer_expr(condition, *span)?;
                if cond_ty != HikariType::Bool {
                    return Err(TypeError::ConditionNotBool(cond_ty, *span));
                }
                self.check(body)?;
                Ok(())
            }

            Stmt::ExprStmt(expr, span) => {
                self.infer_expr(expr, *span)?;
                Ok(())
            }
        }
    }

    fn infer_expr(&self, expr: &Expr, span: Span) -> Result<HikariType, TypeError> {
        match expr {
            Expr::LitInt(_) => Ok(HikariType::Int),
            Expr::LitFloat(_) => Ok(HikariType::Float),
            Expr::LitString(_) => Ok(HikariType::String),
            Expr::LitBool(_) => Ok(HikariType::Bool),

            Expr::Ident(name) => self
                .vars
                .get(name)
                .cloned()
                .ok_or_else(|| TypeError::UndeclaredVariable(name.clone(), span)),

            Expr::BinOp { op, lhs, rhs } => {
                let lty = self.infer_expr(lhs, span)?;
                let rty = self.infer_expr(rhs, span)?;
                if lty != rty {
                    return Err(TypeError::BinOpMismatch {
                        op: op.clone(),
                        lhs: lty,
                        rhs: rty,
                        span,
                    });
                }
                match op {
                    BinOpKind::Eq | BinOpKind::Lt | BinOpKind::Gt => Ok(HikariType::Bool),
                    _ => Ok(lty),
                }
            }

            Expr::Call { name, args } => {
                let sig = self
                    .fns
                    .get(name)
                    .cloned()
                    .ok_or_else(|| TypeError::UndeclaredFunction(name.clone(), span))?;
                if args.len() != sig.params.len() {
                    return Err(TypeError::ArgCountMismatch {
                        name: name.clone(),
                        expected: sig.params.len(),
                        got: args.len(),
                        span,
                    });
                }
                for (arg, param_ty) in args.iter().zip(sig.params.iter()) {
                    let arg_ty = self.infer_expr(arg, span)?;
                    if arg_ty != *param_ty {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: param_ty.clone(),
                            got: arg_ty,
                            span,
                        });
                    }
                }
                Ok(sig.return_ty)
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::{Parser, Stmt};

    fn parse(src: &str) -> Vec<Stmt> {
        Parser::new(Lexer::new(src).tokenize()).parse().unwrap()
    }

    #[test]
    fn test_typecheck_valid_var_decl() {
        // 整数 年齢 ＝ ２０；  — declared Int, assigned Int literal: OK
        let ast = parse("整数 年齢 ＝ ２０；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_type_mismatch_var_decl() {
        // 整数 名前 ＝ 「太郎」；  — declared Int, assigned String: must fail
        let ast = parse("整数 名前 ＝ 「太郎」；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::VarDeclMismatch {
                declared: HikariType::Int,
                got: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_binop_type_mismatch() {
        // 整数 結果 ＝ １ ＋ 「文字」；  — Int + String: must fail
        let ast = parse("整数 結果 ＝ １ ＋ 「文字」；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::BinOpMismatch {
                lhs: HikariType::Int,
                rhs: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_undeclared_variable() {
        // 返す 年齢；  — 年齢 never declared
        let ast = parse("返す 年齢；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "年齢"));
    }

    #[test]
    fn test_typecheck_valid_function() {
        // 関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
        let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_bool_literal_as_if_condition() {
        // 真偽 フラグ ＝ 真；もし フラグ ならば ｛ 印刷（１）； ｝
        let ast = parse("真偽 フラグ ＝ 真；もし フラグ ならば ｛ 印刷（１）； ｝");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_while_valid() {
        let src = "整数 Ｎ ＝ ０；間 Ｎ ＜ ３ ならば ｛ 整数 Ｎ ＝ Ｎ ＋ １； ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_while_non_bool_condition() {
        let src = "整数 Ｎ ＝ ０；間 Ｎ ならば ｛ 整数 Ｎ ＝ Ｎ ＋ １； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ConditionNotBool(HikariType::Int, _)));
    }

    #[test]
    fn test_typecheck_if_non_bool_condition() {
        let src = "整数 Ｎ ＝ ０；もし Ｎ ならば ｛ 印刷（Ｎ）； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ConditionNotBool(HikariType::Int, _)));
    }

    #[test]
    fn test_typecheck_return_type_mismatch() {
        // Function declared ー＞ 整数 but returns a 文字列 literal: must fail
        let src = "関数 誤り（）ー＞ 整数 ｛ 返す 「間違い」； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ReturnTypeMismatch {
                expected: HikariType::Int,
                got: HikariType::String,
                ..
            }
        ));
    }
}

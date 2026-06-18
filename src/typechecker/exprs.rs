use std::collections::HashMap;

use crate::lexer::Span;
use crate::parser::{BinOpKind, Expr, HikariType};

use super::error::TypeError;
use super::symbols::{always_returns, builtin_module, builtin_sig};

impl super::TypeChecker {
    pub(super) fn infer_expr(&mut self, expr: &Expr, span: Span) -> Result<HikariType, TypeError> {
        match expr {
            Expr::LitInt(_) => Ok(HikariType::Int),
            Expr::LitFloat(_) => Ok(HikariType::Float),
            Expr::LitString(_) => Ok(HikariType::String),
            Expr::LitBool(_) => Ok(HikariType::Bool),

            Expr::Ident(name) => {
                // First look up in local variable scope.
                if let Some(ty) = self.lookup_var(name) {
                    return Ok(ty);
                }
                // a bare identifier that names a known function can
                // be used as a first-class function value.
                if let Some(sig) = self.fns.get(name).cloned() {
                    return Ok(HikariType::Fn(sig.params, Box::new(sig.return_ty)));
                }
                Err(TypeError::UndeclaredVariable(name.clone(), span))
            }

            Expr::BinOp { op, lhs, rhs } => {
                let lty = self.infer_expr(lhs, span)?;
                let rty = self.infer_expr(rhs, span)?;
                if matches!(op, BinOpKind::And | BinOpKind::Or) {
                    if lty != HikariType::Bool {
                        return Err(TypeError::BinOpMismatch {
                            op: op.clone(),
                            lhs: lty,
                            rhs: rty,
                            span,
                        });
                    }
                    if rty != HikariType::Bool {
                        return Err(TypeError::BinOpMismatch {
                            op: op.clone(),
                            lhs: lty,
                            rhs: rty,
                            span,
                        });
                    }
                    return Ok(HikariType::Bool);
                }
                if lty != rty {
                    return Err(TypeError::BinOpMismatch {
                        op: op.clone(),
                        lhs: lty,
                        rhs: rty,
                        span,
                    });
                }
                let numeric = matches!(lty, HikariType::Int | HikariType::Float);
                let mismatch = || TypeError::BinOpMismatch {
                    op: op.clone(),
                    lhs: lty.clone(),
                    rhs: rty.clone(),
                    span,
                };
                match op {
                    // Equality works for any two values of the same type.
                    BinOpKind::Eq | BinOpKind::NotEq => Ok(HikariType::Bool),
                    // Ordering is only defined for numbers (the VM has no
                    // ordering for 文字列/真偽/配列).
                    BinOpKind::Lt | BinOpKind::Gt | BinOpKind::LtEq | BinOpKind::GtEq => {
                        if numeric {
                            Ok(HikariType::Bool)
                        } else {
                            Err(mismatch())
                        }
                    }
                    // ＋ also concatenates strings; the rest are numbers only.
                    BinOpKind::Add => {
                        if numeric || lty == HikariType::String {
                            Ok(lty)
                        } else {
                            Err(mismatch())
                        }
                    }
                    BinOpKind::Sub | BinOpKind::Mul | BinOpKind::Div | BinOpKind::Mod => {
                        if numeric { Ok(lty) } else { Err(mismatch()) }
                    }
                    BinOpKind::And | BinOpKind::Or => unreachable!("handled above"),
                }
            }

            Expr::UnaryMinus(inner) => {
                let ity = self.infer_expr(inner, span)?;
                match ity {
                    HikariType::Int | HikariType::Float => Ok(ity),
                    other => Err(TypeError::UnaryOpMismatch { got: other, span }),
                }
            }

            Expr::UnaryNot(inner) => {
                let ity = self.infer_expr(inner, span)?;
                match ity {
                    HikariType::Bool => Ok(HikariType::Bool),
                    other => Err(TypeError::UnaryOpMismatch { got: other, span }),
                }
            }

            Expr::Call { name, args } => {
                if let Some(owning_enum) = self.variant_owner.get(name).cloned() {
                    let payload_types = self
                        .enums
                        .get(&owning_enum)
                        .and_then(|vs| vs.iter().find(|(n, _)| n == name))
                        .map(|(_, tys)| tys.clone())
                        .expect("variant_owner entry implies a registered enum/variant");
                    if args.len() != payload_types.len() {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: payload_types.len(),
                            got: args.len(),
                            span,
                        });
                    }
                    for (arg, param_ty) in args.iter().zip(payload_types.iter()) {
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
                    // Return as Record so the type matches a VarDecl whose
                    // declared type was parsed as HikariType::Record(enum_name).
                    return Ok(HikariType::Record(owning_enum));
                }

                if let Some(module) = builtin_module(name)
                    && !self.imported_modules.contains(module)
                {
                    return Err(TypeError::ModuleNotImported {
                        name: name.clone(),
                        module: module.to_string(),
                        span,
                    });
                }

                if name == "絶対値" || name == "平方根" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arg_ty = self.infer_expr(&args[0], span)?;
                    if !matches!(arg_ty, HikariType::Int | HikariType::Float) {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Int,
                            got: arg_ty,
                            span,
                        });
                    }
                    return Ok(if name == "平方根" {
                        HikariType::Float
                    } else {
                        arg_ty
                    });
                }

                if name == "最大" || name == "最小" || name == "累乗" || name == "余り" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let a_ty = self.infer_expr(&args[0], span)?;
                    let b_ty = self.infer_expr(&args[1], span)?;
                    if !matches!(a_ty, HikariType::Int | HikariType::Float) {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Int,
                            got: a_ty,
                            span,
                        });
                    }
                    if a_ty != b_ty {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: a_ty,
                            got: b_ty,
                            span,
                        });
                    }
                    return Ok(a_ty);
                }

                if name == "切り捨て" || name == "切り上げ" || name == "四捨五入" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arg_ty = self.infer_expr(&args[0], span)?;
                    if arg_ty != HikariType::Float {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Float,
                            got: arg_ty,
                            span,
                        });
                    }
                    return Ok(HikariType::Int);
                }

                if name == "要素数" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arg_ty = self.infer_expr(&args[0], span)?;
                    if !matches!(arg_ty, HikariType::Array(_)) {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Array(Box::new(HikariType::Int)),
                            got: arg_ty,
                            span,
                        });
                    }
                    return Ok(HikariType::Int);
                }

                if name == "追加" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    let val_ty = self.infer_expr(&args[1], span)?;
                    if val_ty != elem_ty {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: elem_ty,
                            got: val_ty,
                            span,
                        });
                    }
                    return Ok(HikariType::Void);
                }

                if name == "取り出す" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    return Ok(elem_ty);
                }

                if name == "含む配列" || name == "位置" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    let val_ty = self.infer_expr(&args[1], span)?;
                    if val_ty != elem_ty {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: elem_ty,
                            got: val_ty,
                            span,
                        });
                    }
                    return Ok(if name == "位置" {
                        HikariType::Int
                    } else {
                        HikariType::Bool
                    });
                }

                if name == "逆順" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    match arr_ty {
                        HikariType::Array(inner) => return Ok(HikariType::Array(inner)),
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    }
                }

                if name == "整列" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    match &arr_ty {
                        HikariType::Array(inner)
                            if matches!(
                                inner.as_ref(),
                                HikariType::Int | HikariType::Float | HikariType::String
                            ) =>
                        {
                            return Ok(arr_ty);
                        }
                        _ => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: arr_ty,
                                span,
                            });
                        }
                    }
                }

                if name == "部分列" {
                    if args.len() != 3 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 3,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    if !matches!(arr_ty, HikariType::Array(_)) {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Array(Box::new(HikariType::Int)),
                            got: arr_ty,
                            span,
                        });
                    }
                    let start_ty = self.infer_expr(&args[1], span)?;
                    if start_ty != HikariType::Int {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Int,
                            got: start_ty,
                            span,
                        });
                    }
                    let end_ty = self.infer_expr(&args[2], span)?;
                    if end_ty != HikariType::Int {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Int,
                            got: end_ty,
                            span,
                        });
                    }
                    return Ok(arr_ty);
                }

                // Map builtins (require 辞書 module import).
                if name == "鍵一覧" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arg_ty = self.infer_expr(&args[0], span)?;
                    match arg_ty {
                        HikariType::Map(k, _) => {
                            return Ok(HikariType::Array(k));
                        }
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Map(
                                    Box::new(HikariType::String),
                                    Box::new(HikariType::Void),
                                ),
                                got: other,
                                span,
                            });
                        }
                    }
                }

                if name == "値一覧" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arg_ty = self.infer_expr(&args[0], span)?;
                    match arg_ty {
                        HikariType::Map(_, v) => {
                            return Ok(HikariType::Array(v));
                        }
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Map(
                                    Box::new(HikariType::String),
                                    Box::new(HikariType::Void),
                                ),
                                got: other,
                                span,
                            });
                        }
                    }
                }

                if name == "削除" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let map_ty = self.infer_expr(&args[0], span)?;
                    let key_ty = match map_ty {
                        HikariType::Map(k, _) => *k,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Map(
                                    Box::new(HikariType::String),
                                    Box::new(HikariType::Void),
                                ),
                                got: other,
                                span,
                            });
                        }
                    };
                    let arg_key_ty = self.infer_expr(&args[1], span)?;
                    if arg_key_ty != key_ty {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: key_ty,
                            got: arg_key_ty,
                            span,
                        });
                    }
                    return Ok(HikariType::Void);
                }

                // 含む is polymorphic: String × String → Bool (文字列 module)
                // or Map × Key → Bool (辞書 module).
                if name == "含む" && args.len() == 2 {
                    let first_ty = self.infer_expr(&args[0], span)?;
                    if let HikariType::Map(k, _) = &first_ty {
                        // Map membership check — requires 辞書 import.
                        if !self.imported_modules.contains("辞書") {
                            return Err(TypeError::ModuleNotImported {
                                name: name.clone(),
                                module: "辞書".to_string(),
                                span,
                            });
                        }
                        let key_arg = self.infer_expr(&args[1], span)?;
                        if key_arg != **k {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: (**k).clone(),
                                got: key_arg,
                                span,
                            });
                        }
                        return Ok(HikariType::Bool);
                    }
                    // Fall through to 文字列-module 含む (String × String).
                    if !self.imported_modules.contains("文字列") {
                        return Err(TypeError::ModuleNotImported {
                            name: name.clone(),
                            module: "文字列".to_string(),
                            span,
                        });
                    }
                    let second_ty = self.infer_expr(&args[1], span)?;
                    if first_ty != HikariType::String || second_ty != HikariType::String {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::String,
                            got: if first_ty != HikariType::String {
                                first_ty
                            } else {
                                second_ty
                            },
                            span,
                        });
                    }
                    return Ok(HikariType::Bool);
                }

                // Higher-order function builtins.
                if name == "マップ" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    let fn_ty = self.infer_expr(&args[1], span)?;
                    let ret_ty = match fn_ty {
                        HikariType::Fn(params, ret)
                            if params.len() == 1 && params[0] == elem_ty =>
                        {
                            *ret
                        }
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Fn(vec![elem_ty], Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    return Ok(HikariType::Array(Box::new(ret_ty)));
                }

                if name == "絞り込み" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty.clone() {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    let fn_ty = self.infer_expr(&args[1], span)?;
                    match fn_ty {
                        HikariType::Fn(params, ret)
                            if params.len() == 1
                                && params[0] == elem_ty
                                && *ret == HikariType::Bool => {}
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Fn(
                                    vec![elem_ty.clone()],
                                    Box::new(HikariType::Bool),
                                ),
                                got: other,
                                span,
                            });
                        }
                    }
                    return Ok(arr_ty);
                }

                if name == "畳み込み" {
                    if args.len() != 3 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 3,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    let acc_ty = self.infer_expr(&args[1], span)?;
                    let fn_ty = self.infer_expr(&args[2], span)?;
                    match &fn_ty {
                        HikariType::Fn(params, ret)
                            if params.len() == 2
                                && params[0] == acc_ty
                                && params[1] == elem_ty
                                && **ret == acc_ty => {}
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Fn(
                                    vec![acc_ty.clone(), elem_ty],
                                    Box::new(acc_ty.clone()),
                                ),
                                got: other.clone(),
                                span,
                            });
                        }
                    }
                    return Ok(acc_ty);
                }

                // 印字 prints any single value with no trailing newline, like
                // 印刷 but newline-less; it accepts any type and returns 無.
                if name == "印字" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    self.infer_expr(&args[0], span)?;
                    return Ok(HikariType::Void);
                }

                if let Some(sig) = builtin_sig(name) {
                    if args.len() != sig.params.len() {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: sig.params.len(),
                            got: args.len(),
                            span,
                        });
                    }
                    if name == "文字列化" {
                        let arg_ty = self.infer_expr(&args[0], span)?;
                        if !matches!(
                            arg_ty,
                            HikariType::Int | HikariType::Float | HikariType::Bool
                        ) {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Int,
                                got: arg_ty,
                                span,
                            });
                        }
                    } else {
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
                    }
                    return Ok(sig.return_ty);
                }

                // Check if name is a Fn-typed local variable.
                if let Some(var_ty) = self.lookup_var(name) {
                    match var_ty {
                        HikariType::Fn(params, ret) => {
                            if args.len() != params.len() {
                                return Err(TypeError::ArgCountMismatch {
                                    name: name.clone(),
                                    expected: params.len(),
                                    got: args.len(),
                                    span,
                                });
                            }
                            for (arg, param_ty) in args.iter().zip(params.iter()) {
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
                            return Ok(*ret);
                        }
                        _ => {
                            return Err(TypeError::UndeclaredFunction(name.clone(), span));
                        }
                    }
                }

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

            Expr::Array(elems) => {
                let Some(first) = elems.first() else {
                    return Err(TypeError::EmptyArrayLiteral(span));
                };
                let expected = self.infer_expr(first, span)?;
                for elem in &elems[1..] {
                    let got = self.infer_expr(elem, span)?;
                    if got != expected {
                        return Err(TypeError::ArrayElementTypeMismatch {
                            expected,
                            got,
                            span,
                        });
                    }
                }
                Ok(HikariType::Array(Box::new(expected)))
            }

            Expr::Index { array, index } => {
                let array_ty = self.infer_expr(array, span)?;
                match array_ty {
                    HikariType::Array(inner) => {
                        let index_ty = self.infer_expr(index, span)?;
                        if index_ty != HikariType::Int {
                            return Err(TypeError::IndexNotInt {
                                got: index_ty,
                                span,
                            });
                        }
                        Ok(*inner)
                    }
                    HikariType::Map(key_ty, val_ty) => {
                        let index_ty = self.infer_expr(index, span)?;
                        if index_ty != *key_ty {
                            return Err(TypeError::IndexNotInt {
                                got: index_ty,
                                span,
                            });
                        }
                        Ok(*val_ty)
                    }
                    other => Err(TypeError::NotIndexable { got: other, span }),
                }
            }

            Expr::MapLit(pairs) => {
                // Empty map literal ｛｝ is handled by the VarDecl special-case
                // above; reaching here with an empty literal is an error.
                let Some((first_k, first_v)) = pairs.first() else {
                    return Err(TypeError::EmptyArrayLiteral(span));
                };
                let key_ty = self.infer_expr(first_k, span)?;
                if key_ty != HikariType::String {
                    return Err(TypeError::IndexNotInt { got: key_ty, span });
                }
                let val_ty = self.infer_expr(first_v, span)?;
                for (k, v) in &pairs[1..] {
                    let kt = self.infer_expr(k, span)?;
                    if kt != HikariType::String {
                        return Err(TypeError::IndexNotInt { got: kt, span });
                    }
                    let vt = self.infer_expr(v, span)?;
                    if vt != val_ty {
                        return Err(TypeError::ArrayElementTypeMismatch {
                            expected: val_ty.clone(),
                            got: vt,
                            span,
                        });
                    }
                }
                Ok(HikariType::Map(
                    Box::new(HikariType::String),
                    Box::new(val_ty),
                ))
            }

            Expr::NewArray(ty) => Ok(HikariType::Array(Box::new(ty.clone()))),

            Expr::RecordLit { type_name, fields } => {
                let declared = self
                    .records
                    .get(type_name)
                    .ok_or_else(|| TypeError::UndeclaredType(type_name.clone(), span))?
                    .clone();

                let provided: std::collections::HashSet<&str> =
                    fields.iter().map(|(n, _)| n.as_str()).collect();
                let required: std::collections::HashSet<&str> =
                    declared.iter().map(|(n, _)| n.as_str()).collect();

                if let Some(missing) = required.difference(&provided).next() {
                    return Err(TypeError::MissingField {
                        type_name: type_name.clone(),
                        field: missing.to_string(),
                        span,
                    });
                }
                if let Some(extra) = provided.difference(&required).next() {
                    return Err(TypeError::UnknownField {
                        type_name: type_name.clone(),
                        field: extra.to_string(),
                        span,
                    });
                }

                for (fname, fexpr) in fields {
                    let expected = declared
                        .iter()
                        .find(|(n, _)| n == fname)
                        .map(|(_, t)| t.clone())
                        .expect("field presence already validated above");
                    let got = self.infer_expr(fexpr, span)?;
                    if got != expected {
                        return Err(TypeError::FieldTypeMismatch {
                            type_name: type_name.clone(),
                            field: fname.clone(),
                            expected: Box::new(expected),
                            got: Box::new(got),
                            span,
                        });
                    }
                }

                Ok(HikariType::Record(type_name.clone()))
            }

            Expr::FieldAccess { record, field } => {
                let record_ty = self.infer_expr(record, span)?;
                let type_name = match record_ty {
                    HikariType::Record(name) => name,
                    other => return Err(TypeError::NotARecord { got: other, span }),
                };
                self.records
                    .get(&type_name)
                    .and_then(|fs| fs.iter().find(|(n, _)| n == field).map(|(_, t)| t.clone()))
                    .ok_or_else(|| TypeError::UnknownField {
                        type_name,
                        field: field.clone(),
                        span,
                    })
            }

            // Anonymous function (lambda).
            Expr::Lambda {
                params,
                return_ty,
                body,
            } => {
                // Unlike a named 関数 (whose body is isolated), a lambda is
                // lexically scoped: it may reference variables from enclosing
                // scopes, which the compiler captures by value. So push a fresh
                // frame for the lambda's params/locals but keep the outer
                // frames visible for lookup. 返す/loop context is still reset:
                // a 返す returns from the lambda and 抜ける can't escape it.
                self.scopes.push(HashMap::new());
                let lambda_depth = self.scopes.len();
                let outer_return_ty = self.current_return_ty.take();
                let outer_loop_depth = std::mem::take(&mut self.loop_depth);

                for (pname, pty) in params {
                    self.check_type_declared(pty, span)?;
                    self.declare_var(pname, pty.clone());
                }
                self.check_type_declared(return_ty, span)?;
                self.current_return_ty = Some(return_ty.clone());

                self.check(body)?;

                // A non-無 lambda must guarantee a return on every path, just
                // like a named 関数; otherwise the VM falls off the end of its
                // chunk with no value to return and underflows the stack.
                if *return_ty != HikariType::Void && !always_returns(body) {
                    return Err(TypeError::MissingReturn {
                        name: "＜無名関数＞".to_string(),
                        span,
                    });
                }

                // Restore outer context. check() balances its own enter/exit,
                // so the lambda's frame is still on top; truncate back to it.
                self.scopes.truncate(lambda_depth - 1);
                self.current_return_ty = outer_return_ty;
                self.loop_depth = outer_loop_depth;

                let param_types: Vec<HikariType> = params.iter().map(|(_, t)| t.clone()).collect();
                Ok(HikariType::Fn(param_types, Box::new(return_ty.clone())))
            }
        }
    }
}

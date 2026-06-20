//! Generic (parametric) signatures for the polymorphic stdlib builtins.
//!
//! Many builtins — the array, map, and higher-order-function helpers — are the
//! same shape over any element type: `要素数` works on a `配列＜Ｔ＞` for any `Ｔ`,
//! `マップ` turns a `配列＜Ｔ＞` and a `Ｔ→Ｕ` into a `配列＜Ｕ＞`, and so on. Rather
//! than hand-write the element-type extraction and matching for each (which is
//! what the type checker used to do), they share one set of signatures written
//! with type variables plus a small unifier that binds those variables against
//! the actual argument types.

use std::collections::HashMap;

use crate::parser::HikariType;

/// A type in a builtin signature: either concrete, or a type variable (`Var`)
/// to be unified against the actual argument types at the call site.
pub(super) enum SigType {
    Int,
    Bool,
    Void,
    Var(u8),
    Array(Box<SigType>),
    Map(Box<SigType>, Box<SigType>),
    Fn(Vec<SigType>, Box<SigType>),
}

pub(super) struct GenericSig {
    pub(super) params: Vec<SigType>,
    pub(super) ret: SigType,
}

// Small constructors keep the signature table readable.
fn var(n: u8) -> SigType {
    SigType::Var(n)
}
fn arr(t: SigType) -> SigType {
    SigType::Array(Box::new(t))
}
fn map(k: SigType, v: SigType) -> SigType {
    SigType::Map(Box::new(k), Box::new(v))
}
fn func(params: Vec<SigType>, ret: SigType) -> SigType {
    SigType::Fn(params, Box::new(ret))
}

/// The generic signature of a polymorphic builtin, or `None` if the builtin is
/// monomorphic / specially constrained (those stay hand-checked: math numerics,
/// `整列`'s orderable constraint, `含む`'s overload, `文字列化`'s union).
pub(super) fn generic_builtin_sig(name: &str) -> Option<GenericSig> {
    // Type variables: 0 = T (element), 1 = U (result/second), etc.
    let sig = match name {
        // ── 配列 module ──
        "要素数" => GenericSig {
            params: vec![arr(var(0))],
            ret: SigType::Int,
        },
        "追加" => GenericSig {
            params: vec![arr(var(0)), var(0)],
            ret: SigType::Void,
        },
        "取り出す" => GenericSig {
            params: vec![arr(var(0))],
            ret: var(0),
        },
        "含む配列" => GenericSig {
            params: vec![arr(var(0)), var(0)],
            ret: SigType::Bool,
        },
        "位置" => GenericSig {
            params: vec![arr(var(0)), var(0)],
            ret: SigType::Int,
        },
        "逆順" => GenericSig {
            params: vec![arr(var(0))],
            ret: arr(var(0)),
        },
        "部分列" => GenericSig {
            params: vec![arr(var(0)), SigType::Int, SigType::Int],
            ret: arr(var(0)),
        },
        // ── 辞書 module ──
        "鍵一覧" => GenericSig {
            params: vec![map(var(0), var(1))],
            ret: arr(var(0)),
        },
        "値一覧" => GenericSig {
            params: vec![map(var(0), var(1))],
            ret: arr(var(1)),
        },
        "削除" => GenericSig {
            params: vec![map(var(0), var(1)), var(0)],
            ret: SigType::Void,
        },
        // ── 関数 module (HOFs) ──
        "マップ" => GenericSig {
            params: vec![arr(var(0)), func(vec![var(0)], var(1))],
            ret: arr(var(1)),
        },
        "絞り込み" => GenericSig {
            params: vec![arr(var(0)), func(vec![var(0)], SigType::Bool)],
            ret: arr(var(0)),
        },
        "畳み込み" => GenericSig {
            params: vec![arr(var(0)), var(1), func(vec![var(1), var(0)], var(1))],
            ret: var(1),
        },
        // ── 入出力 module ──
        // 印字 prints any single value with no trailing newline.
        "印字" => GenericSig {
            params: vec![var(0)],
            ret: SigType::Void,
        },
        _ => return None,
    };
    Some(sig)
}

/// Unify a signature type against an actual argument type, recording variable
/// bindings in `subst`. Returns `Err(())` on any mismatch (the caller turns
/// that into a `TypeError::ArgTypeMismatch`).
pub(super) fn unify(
    sig: &SigType,
    actual: &HikariType,
    subst: &mut HashMap<u8, HikariType>,
) -> Result<(), ()> {
    match sig {
        SigType::Int => (*actual == HikariType::Int).then_some(()).ok_or(()),
        SigType::Bool => (*actual == HikariType::Bool).then_some(()).ok_or(()),
        SigType::Void => (*actual == HikariType::Void).then_some(()).ok_or(()),
        SigType::Var(v) => match subst.get(v) {
            // Already bound: the actual type must match the earlier binding.
            Some(bound) => (bound == actual).then_some(()).ok_or(()),
            None => {
                subst.insert(*v, actual.clone());
                Ok(())
            }
        },
        SigType::Array(inner) => match actual {
            HikariType::Array(a) => unify(inner, a, subst),
            _ => Err(()),
        },
        SigType::Map(k, v) => match actual {
            HikariType::Map(ka, va) => {
                unify(k, ka, subst)?;
                unify(v, va, subst)
            }
            _ => Err(()),
        },
        SigType::Fn(params, ret) => match actual {
            HikariType::Fn(aps, ar) if aps.len() == params.len() => {
                for (p, a) in params.iter().zip(aps.iter()) {
                    unify(p, a, subst)?;
                }
                unify(ret, ar, subst)
            }
            _ => Err(()),
        },
    }
}

/// Resolve a signature type to a concrete `HikariType` using the current
/// substitution. Still-unbound variables are filled with `整数` — this only
/// affects the "expected type" shown in an error message (matching the
/// placeholder the hand-written checks used to report).
pub(super) fn instantiate(sig: &SigType, subst: &HashMap<u8, HikariType>) -> HikariType {
    match sig {
        SigType::Int => HikariType::Int,
        SigType::Bool => HikariType::Bool,
        SigType::Void => HikariType::Void,
        SigType::Var(v) => subst.get(v).cloned().unwrap_or(HikariType::Int),
        SigType::Array(inner) => HikariType::Array(Box::new(instantiate(inner, subst))),
        SigType::Map(k, v) => HikariType::Map(
            Box::new(instantiate(k, subst)),
            Box::new(instantiate(v, subst)),
        ),
        SigType::Fn(params, ret) => HikariType::Fn(
            params.iter().map(|p| instantiate(p, subst)).collect(),
            Box::new(instantiate(ret, subst)),
        ),
    }
}

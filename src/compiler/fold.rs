//! Compile-time constant evaluation.
//!
//! `try_const_eval` returns `Some(value)` when an expression is fully constant
//! (literals and operators only; no variables, calls, or side effects). The
//! result is identical to what the VM would produce at runtime, including the
//! same overflow / division-by-zero behaviour: any expression that would trap
//! at runtime returns `None` here so the VM can still raise the error.

use crate::parser::{BinOpKind, Expr};

use super::value::Value;

/// Try to evaluate `expr` at compile time.
/// Returns `None` if the expression is not fully constant or if evaluation
/// would trap at runtime (division by zero, integer overflow).
pub fn try_const_eval(expr: &Expr) -> Option<Value> {
    match expr {
        Expr::LitInt(n) => Some(Value::Int(*n)),
        Expr::LitFloat(f) => Some(Value::Float(*f)),
        Expr::LitString(s) => Some(Value::Str(s.clone())),
        Expr::LitBool(b) => Some(Value::Bool(*b)),
        Expr::UnaryMinus(inner) => match try_const_eval(inner)? {
            Value::Int(n) => Some(Value::Int(n.checked_neg()?)),
            Value::Float(f) => Some(Value::Float(-f)),
            _ => None,
        },
        Expr::UnaryNot(inner) => match try_const_eval(inner)? {
            Value::Bool(b) => Some(Value::Bool(!b)),
            _ => None,
        },
        Expr::BinOp { op, lhs, rhs } => {
            let l = try_const_eval(lhs)?;
            let r = try_const_eval(rhs)?;
            eval_binop(op, l, r)
        }
        _ => None,
    }
}

fn eval_binop(op: &BinOpKind, l: Value, r: Value) -> Option<Value> {
    match (op, l, r) {
        // Integer arithmetic — uses checked ops to match the VM's behaviour.
        (BinOpKind::Add, Value::Int(a), Value::Int(b)) => Some(Value::Int(a.checked_add(b)?)),
        (BinOpKind::Sub, Value::Int(a), Value::Int(b)) => Some(Value::Int(a.checked_sub(b)?)),
        (BinOpKind::Mul, Value::Int(a), Value::Int(b)) => Some(Value::Int(a.checked_mul(b)?)),
        (BinOpKind::Div, Value::Int(a), Value::Int(b)) if b != 0 => {
            Some(Value::Int(a.checked_div(b)?))
        }
        (BinOpKind::Mod, Value::Int(a), Value::Int(b)) if b != 0 => {
            Some(Value::Int(a.checked_rem(b)?))
        }
        // Float arithmetic. Skip division by zero so the VM raises the error.
        (BinOpKind::Add, Value::Float(a), Value::Float(b)) => Some(Value::Float(a + b)),
        (BinOpKind::Sub, Value::Float(a), Value::Float(b)) => Some(Value::Float(a - b)),
        (BinOpKind::Mul, Value::Float(a), Value::Float(b)) => Some(Value::Float(a * b)),
        (BinOpKind::Div, Value::Float(a), Value::Float(b)) if b != 0.0 => Some(Value::Float(a / b)),
        // String concatenation.
        (BinOpKind::Add, Value::Str(a), Value::Str(b)) => Some(Value::Str(a + &b)),
        // Integer comparisons.
        (BinOpKind::Eq, Value::Int(a), Value::Int(b)) => Some(Value::Bool(a == b)),
        (BinOpKind::NotEq, Value::Int(a), Value::Int(b)) => Some(Value::Bool(a != b)),
        (BinOpKind::Lt, Value::Int(a), Value::Int(b)) => Some(Value::Bool(a < b)),
        (BinOpKind::Gt, Value::Int(a), Value::Int(b)) => Some(Value::Bool(a > b)),
        (BinOpKind::LtEq, Value::Int(a), Value::Int(b)) => Some(Value::Bool(a <= b)),
        (BinOpKind::GtEq, Value::Int(a), Value::Int(b)) => Some(Value::Bool(a >= b)),
        // Float comparisons.
        (BinOpKind::Eq, Value::Float(a), Value::Float(b)) => Some(Value::Bool(a == b)),
        (BinOpKind::NotEq, Value::Float(a), Value::Float(b)) => Some(Value::Bool(a != b)),
        (BinOpKind::Lt, Value::Float(a), Value::Float(b)) => Some(Value::Bool(a < b)),
        (BinOpKind::Gt, Value::Float(a), Value::Float(b)) => Some(Value::Bool(a > b)),
        (BinOpKind::LtEq, Value::Float(a), Value::Float(b)) => Some(Value::Bool(a <= b)),
        (BinOpKind::GtEq, Value::Float(a), Value::Float(b)) => Some(Value::Bool(a >= b)),
        // Bool comparisons and logic.
        (BinOpKind::Eq, Value::Bool(a), Value::Bool(b)) => Some(Value::Bool(a == b)),
        (BinOpKind::NotEq, Value::Bool(a), Value::Bool(b)) => Some(Value::Bool(a != b)),
        (BinOpKind::And, Value::Bool(a), Value::Bool(b)) => Some(Value::Bool(a && b)),
        (BinOpKind::Or, Value::Bool(a), Value::Bool(b)) => Some(Value::Bool(a || b)),
        // String equality.
        (BinOpKind::Eq, Value::Str(a), Value::Str(b)) => Some(Value::Bool(a == b)),
        (BinOpKind::NotEq, Value::Str(a), Value::Str(b)) => Some(Value::Bool(a != b)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{BinOpKind, Expr};

    fn int(n: i64) -> Expr {
        Expr::LitInt(n)
    }
    fn float(f: f64) -> Expr {
        Expr::LitFloat(f)
    }
    fn bool_lit(b: bool) -> Expr {
        Expr::LitBool(b)
    }
    fn str_lit(s: &str) -> Expr {
        Expr::LitString(s.to_string())
    }
    fn binop(op: BinOpKind, l: Expr, r: Expr) -> Expr {
        Expr::BinOp {
            op,
            lhs: Box::new(l),
            rhs: Box::new(r),
        }
    }

    #[test]
    fn test_fold_int_add() {
        assert_eq!(
            try_const_eval(&binop(BinOpKind::Add, int(3), int(4))),
            Some(Value::Int(7))
        );
    }

    #[test]
    fn test_fold_int_sub() {
        assert_eq!(
            try_const_eval(&binop(BinOpKind::Sub, int(10), int(3))),
            Some(Value::Int(7))
        );
    }

    #[test]
    fn test_fold_int_mul() {
        assert_eq!(
            try_const_eval(&binop(BinOpKind::Mul, int(6), int(7))),
            Some(Value::Int(42))
        );
    }

    #[test]
    fn test_fold_int_div() {
        assert_eq!(
            try_const_eval(&binop(BinOpKind::Div, int(10), int(2))),
            Some(Value::Int(5))
        );
    }

    #[test]
    fn test_fold_int_div_by_zero_returns_none() {
        assert_eq!(try_const_eval(&binop(BinOpKind::Div, int(1), int(0))), None);
    }

    #[test]
    fn test_fold_int_mod_by_zero_returns_none() {
        assert_eq!(try_const_eval(&binop(BinOpKind::Mod, int(5), int(0))), None);
    }

    #[test]
    fn test_fold_overflow_returns_none() {
        assert_eq!(
            try_const_eval(&binop(BinOpKind::Add, int(i64::MAX), int(1))),
            None
        );
    }

    #[test]
    fn test_fold_negate() {
        assert_eq!(
            try_const_eval(&Expr::UnaryMinus(Box::new(int(5)))),
            Some(Value::Int(-5))
        );
    }

    #[test]
    fn test_fold_negate_overflow_returns_none() {
        assert_eq!(
            try_const_eval(&Expr::UnaryMinus(Box::new(int(i64::MIN)))),
            None
        );
    }

    #[test]
    fn test_fold_not() {
        assert_eq!(
            try_const_eval(&Expr::UnaryNot(Box::new(bool_lit(true)))),
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn test_fold_float_add() {
        let result = try_const_eval(&binop(BinOpKind::Add, float(1.5), float(2.5)));
        assert_eq!(result, Some(Value::Float(4.0)));
    }

    #[test]
    fn test_fold_string_concat() {
        assert_eq!(
            try_const_eval(&binop(
                BinOpKind::Add,
                str_lit("こんにちは"),
                str_lit("世界")
            )),
            Some(Value::Str("こんにちは世界".to_string()))
        );
    }

    #[test]
    fn test_fold_comparison() {
        assert_eq!(
            try_const_eval(&binop(BinOpKind::Lt, int(3), int(5))),
            Some(Value::Bool(true))
        );
        assert_eq!(
            try_const_eval(&binop(BinOpKind::Gt, int(3), int(5))),
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn test_fold_nested() {
        // (2 + 3) * (10 - 4) → 30
        let expr = binop(
            BinOpKind::Mul,
            binop(BinOpKind::Add, int(2), int(3)),
            binop(BinOpKind::Sub, int(10), int(4)),
        );
        assert_eq!(try_const_eval(&expr), Some(Value::Int(30)));
    }

    #[test]
    fn test_no_fold_with_variable() {
        // A BinOp that references a variable cannot be folded.
        let expr = binop(BinOpKind::Add, Expr::Ident("ｘ".to_string()), int(1));
        assert_eq!(try_const_eval(&expr), None);
    }

    #[test]
    fn test_fold_bool_and() {
        assert_eq!(
            try_const_eval(&binop(BinOpKind::And, bool_lit(true), bool_lit(false))),
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn test_fold_bool_or() {
        assert_eq!(
            try_const_eval(&binop(BinOpKind::Or, bool_lit(false), bool_lit(true))),
            Some(Value::Bool(true))
        );
    }
}

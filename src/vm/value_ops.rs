use crate::compiler::Value;

use super::error::RuntimeError;

pub fn display_value(val: &Value) -> String {
    match val {
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Str(s) => s.clone(),
        Value::Bool(b) => if *b { "真" } else { "偽" }.to_string(),
        Value::Array(arr) => format!(
            "【{}】",
            arr.borrow()
                .iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join("、")
        ),
        Value::Record(rec) => {
            let borrowed = rec.borrow();
            // HashMap iteration order is unspecified; sort keys for
            // deterministic display/test output rather than building
            // ordered-map machinery just for this.
            let mut keys: Vec<&String> = borrowed.keys().collect();
            keys.sort();
            format!(
                "｛{}｝",
                keys.iter()
                    .map(|k| format!("{}：{}", k, display_value(&borrowed[*k])))
                    .collect::<Vec<_>>()
                    .join("、")
            )
        }
        Value::Map(map) => {
            let borrowed = map.borrow();
            let mut keys: Vec<&String> = borrowed.keys().collect();
            keys.sort();
            if keys.is_empty() {
                "｛｝".to_string()
            } else {
                format!(
                    "｛{}｝",
                    keys.iter()
                        .map(|k| format!("「{}」：{}", k, display_value(&borrowed[*k])))
                        .collect::<Vec<_>>()
                        .join("、")
                )
            }
        }
        Value::Enum {
            variant, payload, ..
        } => {
            if payload.is_empty() {
                variant.clone()
            } else {
                format!(
                    "{}（{}）",
                    variant,
                    payload
                        .iter()
                        .map(display_value)
                        .collect::<Vec<_>>()
                        .join("、")
                )
            }
        }
        Value::Function {
            chunk_index, arity, ..
        } => {
            format!("関数＜チャンク{}、引数{}＞", chunk_index, arity)
        }
    }
}

pub(super) fn cmp_lt(lhs: Value, rhs: Value) -> Result<bool, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(a < b),
        (Value::Float(a), Value::Float(b)) => Ok(a < b),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

pub(super) fn cmp_gt(lhs: Value, rhs: Value) -> Result<bool, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(a > b),
        (Value::Float(a), Value::Float(b)) => Ok(a > b),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

pub(super) fn cmp_le(lhs: Value, rhs: Value) -> Result<bool, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(a <= b),
        (Value::Float(a), Value::Float(b)) => Ok(a <= b),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

pub(super) fn cmp_ge(lhs: Value, rhs: Value) -> Result<bool, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(a >= b),
        (Value::Float(a), Value::Float(b)) => Ok(a >= b),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

// 整数化/小数化 accept strings users naturally type with full-width digits
// (e.g. from 入力), so normalize before handing off to Rust's ASCII parser.
pub(super) fn normalize_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{FF10}'..='\u{FF19}' => char::from_u32(c as u32 - 0xFF10 + '0' as u32).unwrap_or(c),
            '．' => '.',
            'ー' => '-',
            _ => c,
        })
        .collect()
}

pub(super) fn float_to_int(f: f64) -> Result<Value, RuntimeError> {
    if f.is_finite() && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
        Ok(Value::Int(f as i64))
    } else {
        Err(RuntimeError::InvalidConversion(
            "結果が整数の範囲に収まりません。".to_string(),
        ))
    }
}

pub(super) fn sort_values(values: &mut [Value]) -> Result<(), RuntimeError> {
    if values
        .iter()
        .any(|v| !matches!(v, Value::Int(_) | Value::Float(_) | Value::Str(_)))
    {
        return Err(RuntimeError::TypeMismatch);
    }
    values.sort_by(|a, b| match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Str(x), Value::Str(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    Ok(())
}

// Each call combines the current time with a process-local counter so two

pub(super) fn arith(
    lhs: Value,
    rhs: Value,
    int_op: impl Fn(i64, i64) -> Option<i64>,
    float_op: impl Fn(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => int_op(a, b)
            .map(Value::Int)
            .ok_or(RuntimeError::IntegerOverflow),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(a, b))),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

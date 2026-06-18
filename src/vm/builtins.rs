use std::cell::RefCell;
use std::rc::Rc;

use crate::compiler::{BuiltinFn, Value};

use super::error::RuntimeError;
use super::value_ops::{display_value, float_to_int, normalize_digits, sort_values};

static RANDOM_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_random_i64(min: i64, max: i64) -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let counter = RANDOM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut seed = nanos ^ counter;
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    // Compute the span in i128 so a wide [min, max] range can't overflow i64.
    let span = (max as i128) - (min as i128) + 1;
    let offset = (seed as u128 % span as u128) as i64;
    min + offset
}

// Integer ops use checked arithmetic so an overflow surfaces as a catchable
// RuntimeError instead of panicking the interpreter.

pub(super) fn call_builtin(
    builtin: BuiltinFn,
    args: &mut Vec<Value>,
) -> Result<Value, RuntimeError> {
    match builtin {
        BuiltinFn::Len => match args.pop() {
            Some(Value::Str(s)) => Ok(Value::Int(s.chars().count() as i64)),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Input => {
            let mut line = String::new();
            std::io::stdin()
                .read_line(&mut line)
                .map_err(|e| RuntimeError::InvalidConversion(e.to_string()))?;
            let trimmed = line.trim_end_matches(['\n', '\r']);
            Ok(Value::Str(trimmed.to_string()))
        }
        BuiltinFn::ParseInt => match args.pop() {
            Some(Value::Str(s)) => {
                normalize_digits(&s)
                    .parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| {
                        RuntimeError::InvalidConversion(format!(
                            "「{}」は整数に変換できません。",
                            s
                        ))
                    })
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::ParseFloat => match args.pop() {
            Some(Value::Str(s)) => normalize_digits(&s)
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| {
                    RuntimeError::InvalidConversion(format!("「{}」は小数に変換できません。", s))
                }),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::ToStr => match args.pop() {
            Some(val) => Ok(Value::Str(display_value(&val))),
            None => Err(RuntimeError::StackUnderflow),
        },
        BuiltinFn::Abs => match args.pop() {
            Some(Value::Int(n)) => Ok(Value::Int(n.wrapping_abs())),
            Some(Value::Float(f)) => Ok(Value::Float(f.abs())),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Sqrt => match args.pop() {
            Some(Value::Int(n)) => {
                if n < 0 {
                    Err(RuntimeError::InvalidConversion(
                        "負の数の平方根は計算できません。".to_string(),
                    ))
                } else {
                    Ok(Value::Float((n as f64).sqrt()))
                }
            }
            Some(Value::Float(f)) => {
                if f < 0.0 {
                    Err(RuntimeError::InvalidConversion(
                        "負の数の平方根は計算できません。".to_string(),
                    ))
                } else {
                    Ok(Value::Float(f.sqrt()))
                }
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Random => {
            let (min, max) = match (args.first().cloned(), args.get(1).cloned()) {
                (Some(Value::Int(min)), Some(Value::Int(max))) => (min, max),
                _ => return Err(RuntimeError::TypeMismatch),
            };
            if min > max {
                return Err(RuntimeError::InvalidConversion(
                    "乱数の範囲が無効です（最小値が最大値より大きいです）。".to_string(),
                ));
            }
            Ok(Value::Int(next_random_i64(min, max)))
        }
        BuiltinFn::Max => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Int(a)), Some(Value::Int(b))) => Ok(Value::Int(a.max(b))),
            (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.max(b))),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Min => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Int(a)), Some(Value::Int(b))) => Ok(Value::Int(a.min(b))),
            (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.min(b))),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Split => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Str(s)), Some(Value::Str(sep))) => {
                let parts: Vec<Value> = s
                    .split(sep.as_str())
                    .map(|p| Value::Str(p.to_string()))
                    .collect();
                Ok(Value::Array(Rc::new(RefCell::new(parts))))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Join => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Array(arr)), Some(Value::Str(sep))) => {
                let joined = arr
                    .borrow()
                    .iter()
                    .map(display_value)
                    .collect::<Vec<_>>()
                    .join(&sep);
                Ok(Value::Str(joined))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Contains => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Str(s)), Some(Value::Str(needle))) => Ok(Value::Bool(s.contains(&needle))),
            (Some(Value::Map(m)), Some(Value::Str(key))) => {
                Ok(Value::Bool(m.borrow().contains_key(&key)))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Replace => {
            match (
                args.first().cloned(),
                args.get(1).cloned(),
                args.get(2).cloned(),
            ) {
                (Some(Value::Str(s)), Some(Value::Str(old)), Some(Value::Str(new))) => {
                    Ok(Value::Str(s.replace(&old, &new)))
                }
                _ => Err(RuntimeError::TypeMismatch),
            }
        }
        BuiltinFn::Pow => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Int(base)), Some(Value::Int(exp))) => {
                // Negative exponents have no integer result; checked_pow also
                // needs a u32, so both bounds are validated up front.
                let exp_u32 = u32::try_from(exp).map_err(|_| {
                    RuntimeError::InvalidConversion(
                        "整数の累乗では指数は０以上である必要があります。".to_string(),
                    )
                })?;
                base.checked_pow(exp_u32)
                    .map(Value::Int)
                    .ok_or(RuntimeError::IntegerOverflow)
            }
            (Some(Value::Float(base)), Some(Value::Float(exp))) => Ok(Value::Float(base.powf(exp))),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Floor => match args.pop() {
            Some(Value::Float(f)) => float_to_int(f.floor()),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Ceil => match args.pop() {
            Some(Value::Float(f)) => float_to_int(f.ceil()),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Round => match args.pop() {
            Some(Value::Float(f)) => float_to_int(f.round()),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Rem => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Int(_)), Some(Value::Int(0))) => Err(RuntimeError::DivisionByZero),
            (Some(Value::Int(a)), Some(Value::Int(b))) => Ok(Value::Int(a % b)),
            (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a % b)),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::ArrayLen => match args.pop() {
            Some(Value::Array(a)) => Ok(Value::Int(a.borrow().len() as i64)),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Push => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Array(a)), Some(val)) => {
                a.borrow_mut().push(val);
                // 追加 is 無-typed; there's no Value::Void, so CallBuiltin
                // (which always pushes one result) gets a placeholder that's
                // never observed since the typechecker forbids using a 無
                // call's result as a value.
                Ok(Value::Int(0))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Pop => match args.pop() {
            Some(Value::Array(a)) => a.borrow_mut().pop().ok_or(RuntimeError::EmptyArray),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::ArrayContains => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Array(a)), Some(val)) => Ok(Value::Bool(a.borrow().contains(&val))),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::IndexOf => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Array(a)), Some(val)) => {
                let pos = a.borrow().iter().position(|v| v == &val);
                Ok(Value::Int(pos.map(|p| p as i64).unwrap_or(-1)))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Reverse => match args.pop() {
            Some(Value::Array(a)) => {
                a.borrow_mut().reverse();
                Ok(Value::Array(a))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::Sort => match args.pop() {
            Some(Value::Array(a)) => {
                {
                    let mut borrowed = a.borrow_mut();
                    sort_values(&mut borrowed)?;
                }
                Ok(Value::Array(a))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        // Unlike the other array builtins, 部分列 returns a NEW array rather
        // than mutating the original — slicing-as-copy matches the common
        // convention in other languages, even though 逆順/整列/追加/取り出す
        // here all mutate in place.
        BuiltinFn::Slice => {
            match (
                args.first().cloned(),
                args.get(1).cloned(),
                args.get(2).cloned(),
            ) {
                (Some(Value::Array(a)), Some(Value::Int(start)), Some(Value::Int(end))) => {
                    let borrowed = a.borrow();
                    let len = borrowed.len();
                    if start < 0 || end < 0 || start > end || end as usize > len {
                        return Err(RuntimeError::IndexOutOfBounds {
                            index: if start < 0 || start as usize > len {
                                start
                            } else {
                                end
                            },
                            len,
                        });
                    }
                    let slice = borrowed[start as usize..end as usize].to_vec();
                    Ok(Value::Array(Rc::new(RefCell::new(slice))))
                }
                _ => Err(RuntimeError::TypeMismatch),
            }
        }
        BuiltinFn::MapKeys => match args.pop() {
            Some(Value::Map(m)) => {
                let borrowed = m.borrow();
                let mut keys: Vec<Value> = borrowed.keys().map(|k| Value::Str(k.clone())).collect();
                // Sort for deterministic ordering.
                keys.sort_by(|a, b| match (a, b) {
                    (Value::Str(x), Value::Str(y)) => x.cmp(y),
                    _ => std::cmp::Ordering::Equal,
                });
                Ok(Value::Array(Rc::new(RefCell::new(keys))))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::MapValues => match args.pop() {
            Some(Value::Map(m)) => {
                let borrowed = m.borrow();
                // Sort by key for deterministic ordering.
                let mut entries: Vec<(&String, &Value)> = borrowed.iter().collect();
                entries.sort_by_key(|(k, _)| k.as_str());
                let vals: Vec<Value> = entries.into_iter().map(|(_, v)| v.clone()).collect();
                Ok(Value::Array(Rc::new(RefCell::new(vals))))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::MapDelete => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Map(m)), Some(Value::Str(key))) => {
                m.borrow_mut().remove(&key);
                // 削除 is 無-typed; return a placeholder like Push does.
                Ok(Value::Int(0))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::ReadFile => match args.pop() {
            Some(Value::Str(path)) => std::fs::read_to_string(&path)
                .map(Value::Str)
                .map_err(|e| RuntimeError::IoError(format!("「{}」を読み込めません: {}", path, e))),
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::WriteFile => match (args.first().cloned(), args.get(1).cloned()) {
            (Some(Value::Str(path)), Some(Value::Str(contents))) => {
                std::fs::write(&path, contents).map_err(|e| {
                    RuntimeError::IoError(format!("「{}」に書き込めません: {}", path, e))
                })?;
                // ファイル書く is 無-typed; return a placeholder like Push does.
                Ok(Value::Int(0))
            }
            _ => Err(RuntimeError::TypeMismatch),
        },
        BuiltinFn::PrintNoNewline => match args.pop() {
            Some(val) => {
                use std::io::Write;
                print!("{}", display_value(&val));
                // Flush so output ordering is correct when not newline-terminated.
                let _ = std::io::stdout().flush();
                // 表示 is 無-typed; return a placeholder like Push does.
                Ok(Value::Int(0))
            }
            None => Err(RuntimeError::StackUnderflow),
        },
        // HOF builtins are handled directly in step() since they
        // need access to the frame machinery; they never reach call_builtin.
        BuiltinFn::MapArray | BuiltinFn::FilterArray | BuiltinFn::FoldArray => {
            unreachable!("HOF builtins are handled in step()")
        }
    }
}

use super::*;

// ── 7a: modulo ───────────────────────────────────────────────────────

#[test]
fn test_vm_modulo_int() {
    assert_eq!(run("返す １０ ％ ３；"), Some(Value::Int(1)));
}

#[test]
fn test_vm_modulo_float() {
    assert_eq!(run("返す １０．０ ％ ３．０；"), Some(Value::Float(1.0)));
}

#[test]
fn test_vm_modulo_by_zero_returns_error() {
    assert_eq!(
        run_result("返す １０ ％ ０；"),
        Err(RuntimeError::DivisionByZero)
    );
}

#[test]
fn test_vm_fizzbuzz_with_modulo() {
    let src =
        "整数 Ｎ ＝ １５；もし Ｎ ％ １５ ＝＝ ０ ならば ｛ 返す ０； ｝違えば｛ 返す Ｎ； ｝";
    assert_eq!(run(src), Some(Value::Int(0)));
}

// ── 7b: array builtins ──────────────────────────────────────────────

#[test]
fn test_vm_array_len_builtin() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；返す 要素数（数字）；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_push_pop_round_trip() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 新配列＜整数＞；追加（数字、１）；追加（数字、２）；整数 最後 ＝ 取り出す（数字）；もし 最後 ＝＝ ２ かつ 要素数（数字） ＝＝ １ ならば ｛ 返す １； ｝違えば｛ 返す ０； ｝";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_pop_empty_array_returns_error() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 新配列＜整数＞；返す 取り出す（数字）；";
    assert_eq!(run_result(src), Err(RuntimeError::EmptyArray));
}

#[test]
fn test_vm_contains_array_found_and_not_found() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；返す 含む配列（数字、２）；";
    assert_eq!(run(src), Some(Value::Bool(true)));

    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；返す 含む配列（数字、９）；";
    assert_eq!(run(src), Some(Value::Bool(false)));
}

#[test]
fn test_vm_index_of_found_and_not_found() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１０、２０、３０】；返す 位置（数字、２０）；";
    assert_eq!(run(src), Some(Value::Int(1)));

    let src = "取り込む 「配列」；整数列 数字 ＝ 【１０、２０、３０】；返す 位置（数字、９９）；";
    assert_eq!(run(src), Some(Value::Int(-1)));
}

#[test]
fn test_vm_reverse_mutates_in_place_and_returns_array() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；整数列 同じ ＝ 逆順（数字）；返す 数字【０】；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_sort_int_array() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【３、１、２】；整列（数字）；返す 数字【０】；";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_sort_float_array() {
    let src = "取り込む 「配列」；小数列 数字 ＝ 【３．０、１．０、２．０】；整列（数字）；返す 数字【０】；";
    assert_eq!(run(src), Some(Value::Float(1.0)));
}

#[test]
fn test_vm_sort_string_array() {
    let src = "取り込む 「配列」；文字列列 文字 ＝ 【「う」、「あ」、「い」】；整列（文字）；返す 文字【０】；";
    assert_eq!(run(src), Some(Value::Str("あ".to_string())));
}

#[test]
fn test_vm_slice_returns_new_array_without_mutating_original() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３、４】；整数列 部分 ＝ 部分列（数字、１、３）；もし 部分【０】 ＝＝ ２ かつ 部分【１】 ＝＝ ３ かつ 要素数（数字） ＝＝ ４ ならば ｛ 返す １； ｝違えば｛ 返す ０； ｝";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_slice_out_of_bounds_returns_error() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２】；返す 部分列（数字、０、５）；";
    assert!(matches!(
        run_result(src),
        Err(RuntimeError::IndexOutOfBounds { .. })
    ));
}

#[test]
fn test_vm_new_array_then_build_and_check_len() {
    let src = "取り込む 「配列」；文字列列 単語 ＝ 新配列＜文字列＞；追加（単語、「あ」）；追加（単語、「い」）；返す 要素数（単語）；";
    assert_eq!(run(src), Some(Value::Int(2)));
}

// ── 7c: more math builtins ─────────────────────────────────────────

#[test]
fn test_vm_pow_int_and_float() {
    let src = "取り込む 「数学」；返す 累乗（２、１０）；";
    assert_eq!(run(src), Some(Value::Int(1024)));

    let src = "取り込む 「数学」；返す 累乗（２．０、０．５）；";
    match run(src) {
        Some(Value::Float(f)) => assert!((f - std::f64::consts::SQRT_2).abs() < 1e-9),
        other => panic!("expected Float, got {:?}", other),
    }
}

#[test]
fn test_vm_pow_negative_exponent_returns_error() {
    let src = "取り込む 「数学」；返す 累乗（２、ー１）；";
    assert!(matches!(
        run_result(src),
        Err(RuntimeError::InvalidConversion(_))
    ));
}

#[test]
fn test_vm_floor_ceil_round() {
    let src = "取り込む 「数学」；返す 切り捨て（３．７）；";
    assert_eq!(run(src), Some(Value::Int(3)));

    let src = "取り込む 「数学」；返す 切り上げ（３．２）；";
    assert_eq!(run(src), Some(Value::Int(4)));

    let src = "取り込む 「数学」；返す 四捨五入（３．５）；";
    assert_eq!(run(src), Some(Value::Int(4)));
}

#[test]
fn test_vm_remainder_function_form() {
    let src = "取り込む 「数学」；返す 余り（１０、３）；";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_remainder_by_zero_returns_error() {
    let src = "取り込む 「数学」；返す 余り（１０、０）；";
    assert_eq!(run_result(src), Err(RuntimeError::DivisionByZero));
}

// ── 11a: file I/O ────────────────────────────────────────────────────

#[test]
fn test_vm_file_write_then_read_round_trips() {
    // Use a unique temp path so the test is self-contained and parallel-safe.
    let path = std::env::temp_dir().join(format!("hikari_io_{}.txt", std::process::id()));
    let path_str = path.to_string_lossy().replace('\\', "/");
    let src = format!(
        "取り込む 「入出力」；ファイル書く（「{p}」、「こんにちは」）；返す ファイル読む（「{p}」）；",
        p = path_str
    );
    assert_eq!(run(&src), Some(Value::Str("こんにちは".to_string())));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_vm_read_missing_file_is_io_error() {
    let src = "取り込む 「入出力」；返す ファイル読む（「/no/such/hikari/file.txt」）；";
    assert!(matches!(run_result(src), Err(RuntimeError::IoError(_))));
}

// ── 11c: program args & environment ──────────────────────────────────

#[test]
fn test_vm_program_args_returns_supplied_args() {
    let src = "取り込む 「環境」；取り込む 「配列」；文字列列 ａ ＝ 引数（）；返す 要素数（ａ）；";
    assert_eq!(run_with_args(src, &["x", "y", "z"]), Some(Value::Int(3)));
}

#[test]
fn test_vm_program_args_indexing() {
    let src = "取り込む 「環境」；文字列列 ａ ＝ 引数（）；返す ａ【１】；";
    assert_eq!(
        run_with_args(src, &["first", "second"]),
        Some(Value::Str("second".to_string()))
    );
}

#[test]
fn test_vm_program_args_empty_when_none_supplied() {
    let src = "取り込む 「環境」；取り込む 「配列」；返す 要素数（引数（））；";
    assert_eq!(run_with_args(src, &[]), Some(Value::Int(0)));
}

#[test]
fn test_vm_env_var_present_and_missing() {
    // SAFETY: single-threaded test process; set then read our own variable.
    unsafe { std::env::set_var("HIKARI_TEST_VAR", "ありがとう") };
    let src = "取り込む 「環境」；返す 環境変数（「HIKARI_TEST_VAR」）；";
    assert_eq!(run(src), Some(Value::Str("ありがとう".to_string())));

    // A missing variable reads as the empty string.
    let src = "取り込む 「環境」；返す 環境変数（「HIKARI_DEFINITELY_MISSING_VAR」）；";
    assert_eq!(run(src), Some(Value::Str(String::new())));
}

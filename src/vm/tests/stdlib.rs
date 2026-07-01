use super::*;

// ── 17a: richer strings ───────────────────────────────────────────────

#[test]
fn test_vm_str_uppercase() {
    let src = r#"取り込む 「文字列」；返す 大文字（「hello」）；"#;
    assert_eq!(run(src), Some(Value::Str("HELLO".to_string())));
}

#[test]
fn test_vm_str_lowercase() {
    let src = r#"取り込む 「文字列」；返す 小文字（「WORLD」）；"#;
    assert_eq!(run(src), Some(Value::Str("world".to_string())));
}

#[test]
fn test_vm_str_trim() {
    let src = "取り込む 「文字列」；返す 整形（「  こんにちは  」）；";
    assert_eq!(run(src), Some(Value::Str("こんにちは".to_string())));
}

#[test]
fn test_vm_str_starts_with_true() {
    let src = r#"取り込む 「文字列」；返す 先頭一致（「hello world」、「hello」）；"#;
    assert_eq!(run(src), Some(Value::Bool(true)));
}

#[test]
fn test_vm_str_starts_with_false() {
    let src = r#"取り込む 「文字列」；返す 先頭一致（「hello world」、「world」）；"#;
    assert_eq!(run(src), Some(Value::Bool(false)));
}

#[test]
fn test_vm_str_ends_with_true() {
    let src = r#"取り込む 「文字列」；返す 末尾一致（「hello world」、「world」）；"#;
    assert_eq!(run(src), Some(Value::Bool(true)));
}

#[test]
fn test_vm_str_ends_with_false() {
    let src = r#"取り込む 「文字列」；返す 末尾一致（「hello world」、「hello」）；"#;
    assert_eq!(run(src), Some(Value::Bool(false)));
}

#[test]
fn test_vm_str_substring() {
    let src = r#"取り込む 「文字列」；返す 部分文字列（「abcdef」、２、５）；"#;
    assert_eq!(run(src), Some(Value::Str("cde".to_string())));
}

#[test]
fn test_vm_str_substring_clamps_end() {
    let src = r#"取り込む 「文字列」；返す 部分文字列（「abc」、０、１００）；"#;
    assert_eq!(run(src), Some(Value::Str("abc".to_string())));
}

#[test]
fn test_vm_str_find_found() {
    let src = r#"取り込む 「文字列」；
               照合 文字列位置（「hello world」、「world」） ｛
                 有る（ｉ） ならば ｛ 返す ｉ； ｝
                 無し（） ならば ｛ 返す ー１； ｝
               ｝"#;
    assert_eq!(run(src), Some(Value::Int(6)));
}

#[test]
fn test_vm_str_find_not_found() {
    let src = r#"取り込む 「文字列」；
               照合 文字列位置（「hello」、「xyz」） ｛
                 有る（ｉ） ならば ｛ 返す ｉ； ｝
                 無し（） ならば ｛ 返す ー１； ｝
               ｝"#;
    assert_eq!(run(src), Some(Value::Int(-1)));
}

#[test]
fn test_vm_str_repeat() {
    let src = r#"取り込む 「文字列」；返す 繰り返し文字列（「ab」、３）；"#;
    assert_eq!(run(src), Some(Value::Str("ababab".to_string())));
}

#[test]
fn test_vm_str_repeat_zero() {
    let src = r#"取り込む 「文字列」；返す 繰り返し文字列（「ab」、０）；"#;
    assert_eq!(run(src), Some(Value::Str(String::new())));
}

// ── 17b: more numerics ───────────────────────────────────────────────

#[test]
fn test_vm_sign_positive() {
    let src = "取り込む 「数学」；返す 符号（５）；";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_sign_negative() {
    let src = "取り込む 「数学」；返す 符号（ー７）；";
    assert_eq!(run(src), Some(Value::Int(-1)));
}

#[test]
fn test_vm_sign_zero() {
    let src = "取り込む 「数学」；返す 符号（０）；";
    assert_eq!(run(src), Some(Value::Int(0)));
}

#[test]
fn test_vm_clamp_within_range() {
    let src = "取り込む 「数学」；返す 挟む（５、０、１０）；";
    assert_eq!(run(src), Some(Value::Int(5)));
}

#[test]
fn test_vm_clamp_below_min() {
    let src = "取り込む 「数学」；返す 挟む（ー５、０、１０）；";
    assert_eq!(run(src), Some(Value::Int(0)));
}

#[test]
fn test_vm_clamp_above_max() {
    let src = "取り込む 「数学」；返す 挟む（１５、０、１０）；";
    assert_eq!(run(src), Some(Value::Int(10)));
}

#[test]
fn test_vm_sum_int_array() {
    let src =
        "取り込む 「数学」；取り込む 「配列」；整数列 ａ ＝ 【１、２、３、４】；返す 総和（ａ）；";
    assert_eq!(run(src), Some(Value::Int(10)));
}

#[test]
fn test_vm_sum_empty_float_array_is_float_zero() {
    // Regression: an empty 小数列 must sum to the float identity 0.0, not the
    // integer 0. The compiler selects SumFloat from the type checker's
    // element-type info; without it, the result would be Int(0) and any
    // downstream float use would fail with a type mismatch at runtime.
    let src = "取り込む 「数学」；小数列 ｘ ＝ 新配列＜小数＞；返す 総和（ｘ）；";
    assert_eq!(run_typed(src), Some(Value::Float(0.0)));
}

#[test]
fn test_vm_sum_empty_int_array_is_int_zero() {
    let src = "取り込む 「数学」；整数列 ｘ ＝ 新配列＜整数＞；返す 総和（ｘ）；";
    assert_eq!(run_typed(src), Some(Value::Int(0)));
}

#[test]
fn test_vm_sum_float_array_nonempty() {
    let src = "取り込む 「数学」；小数列 ｘ ＝ 【１．５、２．５】；返す 総和（ｘ）；";
    assert_eq!(run_typed(src), Some(Value::Float(4.0)));
}

#[test]
fn test_vm_average_int_array() {
    let src =
        "取り込む 「数学」；取り込む 「配列」；整数列 ａ ＝ 【１、２、３、４】；返す 平均（ａ）；";
    assert_eq!(run(src), Some(Value::Float(2.5)));
}

#[test]
fn test_vm_array_max_int() {
    let src = "取り込む 「数学」；取り込む 「配列」；整数列 ａ ＝ 【３、１、４、１、５、９】；返す 最大値（ａ）；";
    assert_eq!(run(src), Some(Value::Int(9)));
}

#[test]
fn test_vm_array_min_int() {
    let src = "取り込む 「数学」；取り込む 「配列」；整数列 ａ ＝ 【３、１、４、１、５、９】；返す 最小値（ａ）；";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_sin_zero() {
    let src = "取り込む 「数学」；返す 正弦（０．０）；";
    // sin(0) = 0.0
    assert_eq!(run(src), Some(Value::Float(0.0)));
}

#[test]
fn test_vm_cos_zero() {
    let src = "取り込む 「数学」；返す 余弦（０．０）；";
    // cos(0) = 1.0
    assert_eq!(run(src), Some(Value::Float(1.0)));
}

#[test]
fn test_vm_exp_zero() {
    let src = "取り込む 「数学」；返す 指数（０．０）；";
    // exp(0) = 1.0
    assert_eq!(run(src), Some(Value::Float(1.0)));
}

#[test]
fn test_vm_ln_one() {
    let src = "取り込む 「数学」；返す 対数（１．０）；";
    // ln(1) = 0.0
    assert_eq!(run(src), Some(Value::Float(0.0)));
}

// ── 17c: more array ops ───────────────────────────────────────────────

#[test]
fn test_vm_concat_two_arrays() {
    let src = "取り込む 「配列」；整数列 ａ ＝ 【１、２】；整数列 ｂ ＝ 【３、４】；整数列 ｃ ＝ 連結（ａ、ｂ）；返す ｃ【２】；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_concat_preserves_originals() {
    let src = "取り込む 「配列」；整数列 ａ ＝ 【１、２】；整数列 ｂ ＝ 【３、４】；整数列 ｃ ＝ 連結（ａ、ｂ）；返す 要素数（ａ）；";
    assert_eq!(run(src), Some(Value::Int(2)));
}

#[test]
fn test_vm_flatten() {
    // 平坦化([[1,2],[3,4]]) = [1,2,3,4]
    let src = "取り込む 「配列」；\
               整数列 ａ ＝ 【１、２】；\
               整数列 ｂ ＝ 【３、４】；\
               整数列列 ｍ ＝ 【ａ、ｂ】；\
               整数列 ｒ ＝ 平坦化（ｍ）；\
               返す 要素数（ｒ）；";
    assert_eq!(run(src), Some(Value::Int(4)));
}

#[test]
fn test_vm_any_true() {
    let src = "取り込む 「配列」；取り込む 「関数」；\
               整数列 ａ ＝ 【１、２、３】；\
               真偽 ｒ ＝ どれか（ａ、｜ｘ：整数｜ ー＞ 真偽 ｛ 返す ｘ ＞ ２； ｝）；\
               返す ｒ；";
    assert_eq!(run(src), Some(Value::Bool(true)));
}

#[test]
fn test_vm_any_false() {
    let src = "取り込む 「配列」；取り込む 「関数」；\
               整数列 ａ ＝ 【１、２、３】；\
               真偽 ｒ ＝ どれか（ａ、｜ｘ：整数｜ ー＞ 真偽 ｛ 返す ｘ ＞ １０； ｝）；\
               返す ｒ；";
    assert_eq!(run(src), Some(Value::Bool(false)));
}

#[test]
fn test_vm_all_true() {
    let src = "取り込む 「配列」；取り込む 「関数」；\
               整数列 ａ ＝ 【２、４、６】；\
               真偽 ｒ ＝ すべて（ａ、｜ｘ：整数｜ ー＞ 真偽 ｛ 返す ｘ ％ ２ ＝＝ ０； ｝）；\
               返す ｒ；";
    assert_eq!(run(src), Some(Value::Bool(true)));
}

#[test]
fn test_vm_all_false() {
    let src = "取り込む 「配列」；取り込む 「関数」；\
               整数列 ａ ＝ 【２、３、６】；\
               真偽 ｒ ＝ すべて（ａ、｜ｘ：整数｜ ー＞ 真偽 ｛ 返す ｘ ％ ２ ＝＝ ０； ｝）；\
               返す ｒ；";
    assert_eq!(run(src), Some(Value::Bool(false)));
}

#[test]
fn test_vm_count_array() {
    let src = "取り込む 「配列」；取り込む 「関数」；\
               整数列 ａ ＝ 【１、２、３、４、５】；\
               整数 ｒ ＝ 数える（ａ、｜ｘ：整数｜ ー＞ 真偽 ｛ 返す ｘ ％ ２ ＝＝ ０； ｝）；\
               返す ｒ；";
    assert_eq!(run(src), Some(Value::Int(2)));
}

// ── 17d: more map ops ─────────────────────────────────────────────────

#[test]
fn test_vm_map_merge() {
    let src = "取り込む 「辞書」；\
               辞書＜文字列、整数＞ ａ ＝ ｛「ｘ」：１｝；\
               辞書＜文字列、整数＞ ｂ ＝ ｛「ｙ」：２｝；\
               辞書＜文字列、整数＞ ｍ ＝ 併合（ａ、ｂ）；\
               返す ｍ【「ｙ」】；";
    assert_eq!(run(src), Some(Value::Int(2)));
}

#[test]
fn test_vm_map_merge_b_overwrites_a() {
    let src = "取り込む 「辞書」；\
               辞書＜文字列、整数＞ ａ ＝ ｛「ｋ」：１｝；\
               辞書＜文字列、整数＞ ｂ ＝ ｛「ｋ」：２｝；\
               辞書＜文字列、整数＞ ｍ ＝ 併合（ａ、ｂ）；\
               返す ｍ【「ｋ」】；";
    assert_eq!(run(src), Some(Value::Int(2)));
}

#[test]
fn test_vm_map_size() {
    let src = "取り込む 「辞書」；辞書＜文字列、整数＞ ｍ ＝ ｛「ａ」：１、「ｂ」：２、「ｃ」：３｝；返す 数（ｍ）；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_map_get_or_default_found() {
    let src = "取り込む 「辞書」；辞書＜文字列、整数＞ ｍ ＝ ｛「ｋ」：４２｝；返す 取得既定（ｍ、「ｋ」、０）；";
    assert_eq!(run(src), Some(Value::Int(42)));
}

#[test]
fn test_vm_map_get_or_default_missing() {
    let src = "取り込む 「辞書」；辞書＜文字列、整数＞ ｍ ＝ ｛「ｋ」：４２｝；返す 取得既定（ｍ、「ｘ」、０）；";
    assert_eq!(run(src), Some(Value::Int(0)));
}

// ── 17e: time ─────────────────────────────────────────────────────────

#[test]
fn test_vm_now_millis_returns_positive_int() {
    let src = "取り込む 「時間」；整数 ｔ ＝ 現在時刻（）；返す ｔ ＞ ０；";
    assert_eq!(run(src), Some(Value::Bool(true)));
}

#[test]
fn test_vm_elapsed_nonnegative() {
    let src = "取り込む 「時間」；整数 ｔ ＝ 現在時刻（）；整数 ｅ ＝ 経過（ｔ）；返す ｅ ≧ ０；";
    assert_eq!(run(src), Some(Value::Bool(true)));
}

#[test]
fn test_vm_sleep_short_succeeds() {
    // 眠る is void; just ensure it runs without error.
    let src = "取り込む 「時間」；眠る（１）；";
    assert_eq!(run(src), None);
}

// ── Phase 23b: dedup, chunk, fold-right ───────────────────────────────

#[test]
fn test_vm_dedup_removes_duplicates() {
    let src = "取り込む 「配列」；整数列 ａ ＝ 【１、２、１、３、２】；整数列 ｂ ＝ 重複除去（ａ）；返す 要素数（ｂ）；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_dedup_preserves_order() {
    let src = "取り込む 「配列」；整数列 ａ ＝ 【３、１、２、１、３】；整数列 ｂ ＝ 重複除去（ａ）；返す ｂ【０】；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_chunk_basic() {
    let src = "取り込む 「配列」；整数列 ａ ＝ 【１、２、３、４、５】；整数列列 ｂ ＝ 分割列（ａ、２）；返す 要素数（ｂ）；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_fold_right_sum() {
    // 畳み込み右 over 【１、２、３】 with add fn and init ０ should give ６
    let src = "取り込む 「関数」；整数列 元 ＝ 【１、２、３】；整数 結果 ＝ 畳み込み右（元、０、｜ｘ：整数、ａ：整数｜ ー＞ 整数 ｛ 返す ｘ ＋ ａ； ｝）；返す 結果；";
    assert_eq!(run(src), Some(Value::Int(6)));
}

// ── Phase 23b: string padding and base conversion ─────────────────────

#[test]
fn test_vm_pad_left() {
    let src = r#"取り込む 「文字列」；返す 左詰め（「hi」、５）；"#;
    assert_eq!(run(src), Some(Value::Str("hi   ".to_string())));
}

#[test]
fn test_vm_pad_right() {
    let src = r#"取り込む 「文字列」；返す 右詰め（「hi」、５）；"#;
    assert_eq!(run(src), Some(Value::Str("   hi".to_string())));
}

#[test]
fn test_vm_format_radix_hex() {
    let src = r#"取り込む 「文字列」；返す 基数変換（２５５、１６）；"#;
    assert_eq!(run(src), Some(Value::Str("ff".to_string())));
}

#[test]
fn test_vm_format_radix_binary() {
    let src = r#"取り込む 「文字列」；返す 基数変換（１０、２）；"#;
    assert_eq!(run(src), Some(Value::Str("1010".to_string())));
}

// ── Phase 23d: exit, stderr (smoke tests) ────────────────────────────

#[test]
fn test_vm_print_stderr_does_not_crash() {
    let src = r#"取り込む 「入出力」；エラー印刷（「エラーです」）；"#;
    assert_eq!(run(src), None);
}

// ── Phase 24b: assertions ─────────────────────────────────────────────

#[test]
fn test_vm_assert_true_succeeds() {
    let src = "確認（１ ＋ １ ＝＝ ２）；返す １；";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_assert_false_raises_assertion_failed() {
    let src = "確認（１ ＝＝ ２）；";
    assert_eq!(run_result(src), Err(RuntimeError::AssertionFailed));
}

#[test]
fn test_vm_assert_false_caught_by_try_catch() {
    let src = "整数 結果 ＝ ０；試す ｛ 確認（偽）； ｝ 失敗 内容 ｛ 結果 ＝ １； ｝返す 結果；";
    assert_eq!(run(src), Some(Value::Int(1)));
}

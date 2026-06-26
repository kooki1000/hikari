use super::*;

// ── 16a: generic function declarations ───────────────────────────────

#[test]
fn test_typecheck_generic_identity_int_is_ok() {
    // 関数＜Ｔ＞ 恒等（Ｔ ｘ）ー＞ Ｔ ｛ 返す ｘ； ｝
    // 整数 ｒ ＝ 恒等（４２）；
    let src = "関数＜Ｔ＞ 恒等（Ｔ ｘ）ー＞Ｔ｛ 返す ｘ； ｝整数 ｒ ＝ 恒等（４２）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_generic_identity_string_is_ok() {
    let src = "関数＜Ｔ＞ 恒等（Ｔ ｘ）ー＞Ｔ｛ 返す ｘ； ｝文字列 ｒ ＝ 恒等（「こんにちは」）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_generic_identity_wrong_result_type_is_error() {
    // Calling 恒等(42) gives 整数, but assigning to 文字列 should fail.
    let src = "関数＜Ｔ＞ 恒等（Ｔ ｘ）ー＞Ｔ｛ 返す ｘ； ｝文字列 ｒ ＝ 恒等（４２）；";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(err, TypeError::VarDeclMismatch { .. }));
}

#[test]
fn test_typecheck_generic_two_type_params_is_ok() {
    // 関数＜Ａ、Ｂ＞ 最初（Ａ ａ、Ｂ ｂ）ー＞Ａ｛ 返す ａ； ｝
    let src =
        "関数＜Ａ、Ｂ＞ 最初（Ａ ａ、Ｂ ｂ）ー＞Ａ｛ 返す ａ； ｝整数 ｒ ＝ 最初（１、「文字」）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_generic_second_type_param_is_ok() {
    // Same fn but use second type var for result.
    let src = "関数＜Ａ、Ｂ＞ 第二（Ａ ａ、Ｂ ｂ）ー＞Ｂ｛ 返す ｂ； ｝文字列 ｒ ＝ 第二（１、「文字」）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_generic_arg_mismatch_when_two_params_same_type_var() {
    // 関数＜Ｔ＞ ペア（Ｔ ａ、Ｔ ｂ）ー＞Ｔ｛ 返す ａ； ｝
    // ペア(1, "x") — first arg binds T=整数, second arg is 文字列: mismatch.
    let src = "関数＜Ｔ＞ ペア（Ｔ ａ、Ｔ ｂ）ー＞Ｔ｛ 返す ａ； ｝整数 ｒ ＝ ペア（１、「ｘ」）；";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_generic_array_param_infers_element_type() {
    // 関数＜Ｔ＞ 先頭（配列＜Ｔ＞ ｌ）ー＞Ｔ｛ 返す ｌ【０】； ｝
    let src = "取り込む 「配列」；関数＜Ｔ＞ 先頭（配列＜Ｔ＞ ｌ）ー＞Ｔ｛ 返す ｌ【０】； ｝\
               整数 ｒ ＝ 先頭（【１、２、３】）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_generic_array_param_result_type_mismatch_is_error() {
    // 先頭([1,2,3]) returns 整数 — assigning to 文字列 must fail.
    let src = "取り込む 「配列」；関数＜Ｔ＞ 先頭（配列＜Ｔ＞ ｌ）ー＞Ｔ｛ 返す ｌ【０】； ｝\
               文字列 ｒ ＝ 先頭（【１、２、３】）；";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(err, TypeError::VarDeclMismatch { .. }));
}

#[test]
fn test_typecheck_generic_body_return_type_matches_type_var() {
    // Body returns a value of type Ｔ — the return-type check inside the
    // function body must pass even though Ｔ is not a declared record.
    let src = "関数＜Ｔ＞ 変換（Ｔ ｖ）ー＞Ｔ｛ 返す ｖ； ｝";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_generic_function_missing_return_is_error() {
    // Body has no 返す — should raise MissingReturn.
    let src = "関数＜Ｔ＞ 空（Ｔ ｖ）ー＞Ｔ｛ 印刷（ｖ）； ｝";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(err, TypeError::MissingReturn { .. }));
}

#[test]
fn test_typecheck_generic_void_return_is_ok() {
    // Generic param, void return — no MissingReturn check.
    let src = "関数＜Ｔ＞ 表示（Ｔ ｖ）ー＞無｛ 印刷（ｖ）； ｝表示（４２）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_generic_called_multiple_times_with_different_types() {
    // Each call site gets its own substitution — independent instantiation.
    let src = "関数＜Ｔ＞ 恒等（Ｔ ｘ）ー＞Ｔ｛ 返す ｘ； ｝\
               整数 ａ ＝ 恒等（１）；\
               文字列 ｂ ＝ 恒等（「ｈｉ」）；\
               真偽 ｃ ＝ 恒等（真）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

use super::*;

// ── 8a: exhaustive-return analysis ───────────────────────────────────

#[test]
fn test_typecheck_if_else_both_return_is_ok() {
    let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ もし Ａ ＞ ０ ならば ｛ 返す １； ｝ 違えば ｛ 返す ０； ｝ ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_if_without_else_is_missing_return() {
    let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ もし Ａ ＞ ０ ならば ｛ 返す １； ｝ ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::MissingReturn { name, .. } if name == "計算"));
}

#[test]
fn test_typecheck_trailing_return_after_if_else_is_ok_even_if_branches_dont_return() {
    let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ もし Ａ ＞ ０ ならば ｛ 印刷（１）； ｝ 違えば ｛ 印刷（０）； ｝ 返す Ａ； ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_void_function_exempt_from_missing_return() {
    let src = "関数 表示（整数 Ａ）ー＞ 無 ｛ 印刷（Ａ）； ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_while_loop_with_return_is_still_missing_return() {
    let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 間 Ａ ＞ ０ ならば ｛ 返す Ａ； ｝ ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::MissingReturn { name, .. } if name == "計算"));
}

#[test]
fn test_typecheck_try_catch_both_branches_return_is_ok() {
    let src = "関数 計算（）ー＞ 整数 ｛ 試す ｛ 返す １； ｝ 失敗 失敗内容 ｛ 返す ０； ｝ ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_try_catch_only_one_branch_returns_is_missing_return() {
    let src =
        "関数 計算（）ー＞ 整数 ｛ 試す ｛ 返す １； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝ ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::MissingReturn { name, .. } if name == "計算"));
}

// ── 8b: break / continue ─────────────────────────────────────────────

#[test]
fn test_typecheck_break_continue_inside_loops_ok() {
    assert!(
        TypeChecker::new()
            .check(&parse("間 真 ならば ｛ 抜ける； ｝"))
            .is_ok()
    );
    assert!(
        TypeChecker::new()
            .check(&parse("間 真 ならば ｛ 続ける； ｝"))
            .is_ok()
    );
    assert!(
        TypeChecker::new()
            .check(&parse("繰り返す ｉ ＝ ０ から ５ ならば ｛ 抜ける； ｝"))
            .is_ok()
    );
    assert!(
        TypeChecker::new()
            .check(&parse(
                "整数列 数字 ＝ 【１】；各 要素 ： 数字 ならば ｛ 続ける； ｝"
            ))
            .is_ok()
    );
}

#[test]
fn test_typecheck_break_continue_outside_loop_is_error() {
    let err = TypeChecker::new().check(&parse("抜ける；")).unwrap_err();
    assert!(matches!(err, TypeError::ControlFlowOutsideLoop { .. }));

    let err = TypeChecker::new().check(&parse("続ける；")).unwrap_err();
    assert!(matches!(err, TypeError::ControlFlowOutsideLoop { .. }));
}

#[test]
fn test_typecheck_break_inside_if_inside_loop_is_ok() {
    let src = "間 真 ならば ｛ もし 真 ならば ｛ 抜ける； ｝ ｝";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_break_inside_function_body_not_itself_in_loop_is_error() {
    // The function is CALLED from inside a loop, but its own body has no
    // enclosing loop, proving loop_depth resets per function.
    let src = "関数 内部（）ー＞ 無 ｛ 抜ける； ｝間 真 ならば ｛ 内部（）； 抜ける； ｝";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(err, TypeError::ControlFlowOutsideLoop { .. }));
}

// ── 8c: bare return / void semantics ──────────────────────────────────

#[test]
fn test_typecheck_bare_return_in_void_function_is_ok() {
    let src = "関数 表示（）ー＞ 無 ｛ 返す； ｝";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_bare_return_in_non_void_function_is_error() {
    let src = "関数 計算（）ー＞ 整数 ｛ 返す； ｝";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ReturnTypeMismatch {
            expected: HikariType::Int,
            got: HikariType::Void,
            ..
        }
    ));
}

#[test]
fn test_typecheck_bare_return_at_top_level_is_ok() {
    assert!(TypeChecker::new().check(&parse("返す；")).is_ok());
}

// ── void value used in value position ─────────────────────────────────
// A 無-returning function call produces no value, so using its result where a
// value is expected must be a compile-time TypeError, not a runtime crash.

const VOID_FN: &str = "関数 何もしない（）ー＞ 無 ｛ 返す； ｝";

#[test]
fn test_typecheck_void_call_as_bare_statement_is_ok() {
    // Calling a 無 function purely for its side effects must stay legal.
    let src = format!("{VOID_FN}何もしない（）；");
    assert!(TypeChecker::new().check(&parse(&src)).is_ok());
}

#[test]
fn test_typecheck_void_call_as_print_argument_is_error() {
    let src = format!("{VOID_FN}印刷（何もしない（））；");
    let err = TypeChecker::new().check(&parse(&src)).unwrap_err();
    assert!(matches!(err, TypeError::VoidValueUsed { .. }));
}

#[test]
fn test_typecheck_void_call_as_var_initialiser_is_error() {
    let src = format!("{VOID_FN}整数 Ｘ ＝ 何もしない（）；");
    let err = TypeChecker::new().check(&parse(&src)).unwrap_err();
    assert!(matches!(err, TypeError::VoidValueUsed { .. }));
}

#[test]
fn test_typecheck_void_call_in_binary_op_is_error() {
    let src = format!("{VOID_FN}印刷（何もしない（） ＋ １）；");
    let err = TypeChecker::new().check(&parse(&src)).unwrap_err();
    assert!(matches!(err, TypeError::VoidValueUsed { .. }));
}

#[test]
fn test_typecheck_void_call_as_function_argument_is_error() {
    let src = format!(
        "{VOID_FN}関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝二倍（何もしない（））；"
    );
    let err = TypeChecker::new().check(&parse(&src)).unwrap_err();
    assert!(matches!(err, TypeError::VoidValueUsed { .. }));
}

#[test]
fn test_typecheck_void_call_as_array_element_is_error() {
    let src = format!(
        "取り込む 「配列」；{VOID_FN}整数列 数字 ＝ 【１】；追加（数字、何もしない（））；"
    );
    let err = TypeChecker::new().check(&parse(&src)).unwrap_err();
    assert!(matches!(err, TypeError::VoidValueUsed { .. }));
}

#[test]
fn test_typecheck_void_call_as_return_value_is_error() {
    let src = format!("{VOID_FN}関数 包む（）ー＞ 整数 ｛ 返す 何もしない（）； ｝");
    let err = TypeChecker::new().check(&parse(&src)).unwrap_err();
    assert!(matches!(err, TypeError::VoidValueUsed { .. }));
}

#[test]
fn test_typecheck_void_call_as_condition_is_error() {
    let src = format!("{VOID_FN}もし 何もしない（） ならば ｛ 印刷（１）； ｝");
    let err = TypeChecker::new().check(&parse(&src)).unwrap_err();
    assert!(matches!(err, TypeError::VoidValueUsed { .. }));
}

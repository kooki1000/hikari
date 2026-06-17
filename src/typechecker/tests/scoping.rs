use super::*;

#[test]
fn test_typecheck_for_range_valid() {
    let src = "繰り返す カウンタ ＝ ０ から ５ ならば ｛ 印刷（カウンタ）； ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_for_range_non_int_bound() {
    let src = "繰り返す カウンタ ＝ 「あ」 から ５ ならば ｛ 印刷（カウンタ）； ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_for_each_valid() {
    let src = "整数列 数字 ＝ 【１、２、３】；各 要素 ： 数字 ならば ｛ 印刷（要素）； ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_for_each_non_array() {
    let src = "整数 数字 ＝ ５；各 要素 ： 数字 ならば ｛ 印刷（要素）； ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::NotIndexable { .. }));
}

#[test]
fn test_typecheck_if_body_var_not_visible_after_block() {
    let src = "もし 真 ならば ｛ 整数 Ｎ ＝ ５； ｝ 返す Ｎ；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "Ｎ"));
}

#[test]
fn test_typecheck_while_body_var_not_visible_after_block() {
    let src = "間 真 ならば ｛ 整数 Ｎ ＝ ５； ｝ 返す Ｎ；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "Ｎ"));
}

#[test]
fn test_typecheck_for_range_var_not_visible_after_loop() {
    let src = "繰り返す カウンタ ＝ ０ から ５ ならば ｛ 印刷（カウンタ）； ｝返す カウンタ；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "カウンタ"));
}

#[test]
fn test_typecheck_for_each_var_not_visible_after_loop() {
    let src = "整数列 数字 ＝ 【１、２】；各 要素 ： 数字 ならば ｛ 印刷（要素）； ｝返す 要素；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "要素"));
}

#[test]
fn test_typecheck_outer_var_visible_inside_nested_block() {
    let src = "整数 外 ＝ １０；もし 真 ならば ｛ 間 真 ならば ｛ 印刷（外）； ｝ ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_shadowing_does_not_corrupt_outer_type() {
    // Outer 値 is Int; inner block shadows it with String, then exits.
    // After the block, 値 should still be Int, so adding it to an Int works.
    let src = "整数 値 ＝ １；もし 真 ならば ｛ 文字列 値 ＝ 「あ」； ｝ 整数 結果 ＝ 値 ＋ ２；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_function_body_isolated_from_outer_scope() {
    // 外 is declared in the script scope, not as a param of 関数.
    // The function body must NOT see it.
    let src = "整数 外 ＝ １；関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す 外； ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "外"));
}

#[test]
fn test_typecheck_if_then_and_else_have_separate_scopes() {
    let src = "もし 真 ならば ｛ 整数 Ａ ＝ １； ｝ 違えば ｛ 返す Ａ； ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "Ａ"));
}

#[test]
fn test_typecheck_try_catch_error_var_is_string() {
    let src = "試す ｛ 印刷（１）； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_try_body_var_does_not_leak() {
    let src = "試す ｛ 整数 Ａ ＝ １； ｝ 失敗 失敗内容 ｛ 返す Ａ； ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "Ａ"));

    let src2 = "試す ｛ 整数 Ａ ＝ １； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝返す Ａ；";
    let ast2 = parse(src2);
    let err2 = TypeChecker::new().check(&ast2).unwrap_err();
    assert!(matches!(err2, TypeError::UndeclaredVariable(n, _) if n == "Ａ"));
}

#[test]
fn test_typecheck_try_catch_error_var_not_visible_after_block() {
    let src = "試す ｛ 印刷（１）； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝印刷（失敗内容）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "失敗内容"));
}

#[test]
fn test_typecheck_try_body_type_error_still_rejected() {
    let src = "試す ｛ 整数 Ａ ＝ 「文字」； ｝ 失敗 失敗内容 ｛ ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::VarDeclMismatch { .. }));
}

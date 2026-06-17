use super::*;

#[test]
fn test_typecheck_valid_var_decl() {
    // 整数 年齢 ＝ ２０；  — declared Int, assigned Int literal: OK
    let ast = parse("整数 年齢 ＝ ２０；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_type_mismatch_var_decl() {
    // 整数 名前 ＝ 「太郎」；  — declared Int, assigned String: must fail
    let ast = parse("整数 名前 ＝ 「太郎」；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::VarDeclMismatch {
            declared: HikariType::Int,
            got: HikariType::String,
            ..
        }
    ));
}

#[test]
fn test_typecheck_binop_type_mismatch() {
    // 整数 結果 ＝ １ ＋ 「文字」；  — Int + String: must fail
    let ast = parse("整数 結果 ＝ １ ＋ 「文字」；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::BinOpMismatch {
            lhs: HikariType::Int,
            rhs: HikariType::String,
            ..
        }
    ));
}

#[test]
fn test_typecheck_undeclared_variable() {
    // 返す 年齢；  — 年齢 never declared
    let ast = parse("返す 年齢；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "年齢"));
}

#[test]
fn test_typecheck_valid_function() {
    // 関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
    let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_bool_literal_as_if_condition() {
    // 真偽 フラグ ＝ 真；もし フラグ ならば ｛ 印刷（１）； ｝
    let ast = parse("真偽 フラグ ＝ 真；もし フラグ ならば ｛ 印刷（１）； ｝");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_while_valid() {
    let src = "整数 Ｎ ＝ ０；間 Ｎ ＜ ３ ならば ｛ 整数 Ｎ ＝ Ｎ ＋ １； ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_while_non_bool_condition() {
    let src = "整数 Ｎ ＝ ０；間 Ｎ ならば ｛ 整数 Ｎ ＝ Ｎ ＋ １； ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ConditionNotBool(HikariType::Int, _)
    ));
}

#[test]
fn test_typecheck_if_non_bool_condition() {
    let src = "整数 Ｎ ＝ ０；もし Ｎ ならば ｛ 印刷（Ｎ）； ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ConditionNotBool(HikariType::Int, _)
    ));
}

#[test]
fn test_typecheck_reassignment_valid() {
    let ast = parse("整数 年齢 ＝ ２０；年齢 ＝ ３０；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_string_concat() {
    let ast = parse("文字列 結果 ＝ 「あ」 ＋ 「い」；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_reassignment_type_mismatch() {
    // 整数 年齢 ＝ ２０； 年齢 ＝ 「太郎」；
    let ast = parse("整数 年齢 ＝ ２０；年齢 ＝ 「太郎」；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::VarDeclMismatch {
            declared: HikariType::Int,
            got: HikariType::String,
            ..
        }
    ));
}

#[test]
fn test_typecheck_builtin_strlen() {
    let ast = parse("整数 結果 ＝ 文字数（「あ」）；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_builtin_strlen_arg_type_mismatch() {
    let ast = parse("整数 結果 ＝ 文字数（１）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ArgTypeMismatch {
            param: HikariType::String,
            got: HikariType::Int,
            ..
        }
    ));
}

#[test]
fn test_typecheck_builtin_input() {
    let ast = parse("文字列 結果 ＝ 入力（）；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_builtin_input_arg_count_mismatch() {
    let ast = parse("文字列 結果 ＝ 入力（「余分」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ArgCountMismatch {
            expected: 0,
            got: 1,
            ..
        }
    ));
}

#[test]
fn test_typecheck_builtin_parse_int() {
    let ast = parse("整数 結果 ＝ 整数化（「４２」）；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_builtin_parse_float() {
    let ast = parse("小数 結果 ＝ 小数化（「３．５」）；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_builtin_to_str_polymorphic() {
    let ast = parse("文字列 結果 ＝ 文字列化（１）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("文字列 結果 ＝ 文字列化（１．５）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("文字列 結果 ＝ 文字列化（真）；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_builtin_to_str_rejects_string_arg() {
    let ast = parse("文字列 結果 ＝ 文字列化（「だめ」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ArgTypeMismatch {
            got: HikariType::String,
            ..
        }
    ));
}

#[test]
fn test_typecheck_reassignment_undeclared_variable() {
    let ast = parse("年齢 ＝ ２０；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "年齢"));
}

#[test]
fn test_typecheck_unary_minus_int_ok() {
    let ast = parse("整数 結果 ＝ ー５；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_unary_minus_on_bool_fails() {
    let ast = parse("真偽 フラグ ＝ 真；整数 結果 ＝ ーフラグ；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::UnaryOpMismatch {
            got: HikariType::Bool,
            ..
        }
    ));
}

#[test]
fn test_typecheck_logical_and_or_require_bool() {
    let ast = parse("真偽 結果 ＝ 真 かつ 偽；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("真偽 結果 ＝ 真 または 偽；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("真偽 結果 ＝ １ かつ 真；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::BinOpMismatch { .. }));
}

#[test]
fn test_typecheck_unary_not_requires_bool() {
    let ast = parse("真偽 結果 ＝ 否定 真；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("真偽 結果 ＝ 否定 １；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::UnaryOpMismatch {
            got: HikariType::Int,
            ..
        }
    ));
}

#[test]
fn test_typecheck_additional_comparison_operators() {
    let ast = parse("真偽 結果 ＝ ３ ≦ ５；");
    assert!(TypeChecker::new().check(&ast).is_ok());
    let ast = parse("真偽 結果 ＝ ５ ≧ ３；");
    assert!(TypeChecker::new().check(&ast).is_ok());
    let ast = parse("真偽 結果 ＝ １ ≠ ２；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_return_type_mismatch() {
    // Function declared ー＞ 整数 but returns a 文字列 literal: must fail
    let src = "関数 誤り（）ー＞ 整数 ｛ 返す 「間違い」； ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ReturnTypeMismatch {
            expected: HikariType::Int,
            got: HikariType::String,
            ..
        }
    ));
}

#[test]
fn test_typecheck_array_literal_valid() {
    let ast = parse("整数列 数字 ＝ 【１、２、３】；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_array_element_type_mismatch() {
    let ast = parse("整数列 数字 ＝ 【１、「あ」】；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ArrayElementTypeMismatch {
            expected: HikariType::Int,
            got: HikariType::String,
            ..
        }
    ));
}

#[test]
fn test_typecheck_empty_array_literal() {
    let ast = parse("整数列 数字 ＝ 【】；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::EmptyArrayLiteral(_)));
}

#[test]
fn test_typecheck_index_non_array() {
    let ast = parse("整数 値 ＝ ５；返す 値【０】；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::NotIndexable {
            got: HikariType::Int,
            ..
        }
    ));
}

#[test]
fn test_typecheck_index_not_int() {
    let ast = parse("整数列 数字 ＝ 【１、２】；返す 数字【「あ」】；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::IndexNotInt {
            got: HikariType::String,
            ..
        }
    ));
}

#[test]
fn test_typecheck_index_assign_valid() {
    let ast = parse("整数列 数字 ＝ 【１、２】；数字【０】＝ ９；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_index_assign_type_mismatch() {
    let ast = parse("整数列 数字 ＝ 【１、２】；数字【０】＝ 「あ」；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ArrayElementTypeMismatch {
            expected: HikariType::Int,
            got: HikariType::String,
            ..
        }
    ));
}

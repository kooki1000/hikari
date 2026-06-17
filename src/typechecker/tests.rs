use super::error::TypeError;
use super::*;
use crate::lexer::Lexer;
use crate::parser::HikariType;
use crate::parser::{Parser, Stmt};

fn parse(src: &str) -> Vec<Stmt> {
    Parser::new(Lexer::new(src).tokenize()).parse().unwrap()
}

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

#[test]
fn test_typecheck_math_builtins_after_import() {
    let src = "取り込む 「数学」；整数 結果 ＝ 絶対値（ー５）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「数学」；小数 結果 ＝ 平方根（９）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「数学」；整数 結果 ＝ 乱数（１、１０）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「数学」；整数 結果 ＝ 最大（１、２）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「数学」；小数 結果 ＝ 最小（１．０、２．０）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_string_builtins_after_import() {
    let src = "取り込む 「文字列」；文字列列 結果 ＝ 分割（「あ、い」、「、」）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「文字列」；文字列 結果 ＝ 結合（【「あ」、「い」】、「、」）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「文字列」；真偽 結果 ＝ 含む（「あいう」、「い」）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「文字列」；文字列 結果 ＝ 置換（「あいう」、「い」、「え」）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_stdlib_builtin_without_import_fails() {
    let src = "整数 結果 ＝ 絶対値（ー５）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ModuleNotImported { module, .. } if module == "数学"
    ));

    let src = "真偽 結果 ＝ 含む（「あ」、「い」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ModuleNotImported { module, .. } if module == "文字列"
    ));
}

#[test]
fn test_typecheck_abs_sqrt_polymorphic_mismatch() {
    let src = "取り込む 「数学」；整数 結果 ＝ 絶対値（「文字」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));

    let src = "取り込む 「数学」；小数 結果 ＝ 平方根（「文字」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_max_min_polymorphic_mismatch() {
    let src = "取り込む 「数学」；整数 結果 ＝ 最大（１、「あ」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));

    let src = "取り込む 「数学」；文字列 結果 ＝ 最小（「あ」、「い」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_arithmetic_on_bool_is_rejected() {
    // 真 ＋ 偽 has matching operand types but ＋ is undefined for 真偽.
    let ast = parse("真偽 結果 ＝ 真 ＋ 偽；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::BinOpMismatch { .. }));
}

#[test]
fn test_typecheck_subtraction_on_strings_is_rejected() {
    // ＋ concatenates strings, but ー/＊/／ are numbers-only.
    let ast = parse("文字列 結果 ＝ 「あ」 ー 「い」；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::BinOpMismatch { .. }));
}

#[test]
fn test_typecheck_ordering_on_strings_is_rejected() {
    // ＜/＞/≦/≧ are only defined for numbers.
    let ast = parse("真偽 結果 ＝ 「あ」 ＜ 「い」；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::BinOpMismatch { .. }));
}

#[test]
fn test_typecheck_equality_on_strings_is_allowed() {
    // ＝＝/≠ remain valid for any two values of the same type.
    let ast = parse("真偽 結果 ＝ 「あ」 ＝＝ 「い」；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_string_concatenation_still_allowed() {
    let ast = parse("文字列 結果 ＝ 「あ」 ＋ 「い」；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

// ── 7a: modulo ───────────────────────────────────────────────────────

#[test]
fn test_typecheck_modulo_numeric_only() {
    let ast = parse("整数 結果 ＝ １０ ％ ３；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("文字列 結果 ＝ 「あ」 ％ 「い」；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::BinOpMismatch { .. }));
}

// ── 7b: array builtins ──────────────────────────────────────────────

#[test]
fn test_typecheck_array_builtins_require_import() {
    let ast = parse("整数列 数字 ＝ 【１】；整数 結果 ＝ 要素数（数字）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ModuleNotImported { module, .. } if module == "配列"
    ));
}

#[test]
fn test_typecheck_array_len_happy_and_mismatch() {
    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 要素数（数字）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；整数 結果 ＝ 要素数（５）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_push_happy_and_mismatch() {
    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；追加（数字、２）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；追加（数字、「あ」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_pop_happy_and_mismatch() {
    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 取り出す（数字）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；整数 結果 ＝ 取り出す（５）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_contains_array_and_index_of_happy_and_mismatch() {
    let ast =
        parse("取り込む 「配列」；整数列 数字 ＝ 【１】；真偽 結果 ＝ 含む配列（数字、１）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 位置（数字、１）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast =
        parse("取り込む 「配列」；整数列 数字 ＝ 【１】；真偽 結果 ＝ 含む配列（数字、「あ」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_reverse_happy_and_mismatch() {
    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数列 結果 ＝ 逆順（数字）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；整数 結果 ＝ 逆順（５）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_sort_happy_and_rejects_bool_array() {
    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整列（数字）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；真偽列 旗 ＝ 【真】；整列（旗）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_slice_happy_and_mismatch() {
    let ast = parse(
        "取り込む 「配列」；整数列 数字 ＝ 【１、２】；整数列 結果 ＝ 部分列（数字、０、１）；",
    );
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse(
        "取り込む 「配列」；整数列 数字 ＝ 【１】；整数列 結果 ＝ 部分列（数字、「あ」、１）；",
    );
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_new_array_expr() {
    let ast = parse("整数列 数字 ＝ 新配列＜整数＞；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("小数列 数字 ＝ 新配列＜小数＞；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("文字列列 文字 ＝ 新配列＜文字列＞；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("真偽列 旗 ＝ 新配列＜真偽＞；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

// ── 7c: more math builtins ─────────────────────────────────────────

#[test]
fn test_typecheck_pow_happy_and_mismatch() {
    let ast = parse("取り込む 「数学」；整数 結果 ＝ 累乗（２、３）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「数学」；整数 結果 ＝ 累乗（２、「あ」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_floor_ceil_round_happy_and_reject_int() {
    let ast = parse("取り込む 「数学」；整数 結果 ＝ 切り捨て（３．５）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「数学」；整数 結果 ＝ 切り上げ（３．５）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「数学」；整数 結果 ＝ 四捨五入（３．５）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「数学」；整数 結果 ＝ 切り捨て（３）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_remainder_function_form() {
    let ast = parse("取り込む 「数学」；整数 結果 ＝ 余り（１０、３）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「数学」；整数 結果 ＝ 余り（１０、「あ」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_math_builtins_without_import_fails() {
    let ast = parse("整数 結果 ＝ 累乗（２、３）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ModuleNotImported { module, .. } if module == "数学"
    ));
}

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

// ── 9a: records ───────────────────────────────────────────────────────

#[test]
fn test_typecheck_record_construction_and_field_read_valid() {
    let src =
        "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝点 ｐ ＝ 点 ｛ ｘ：１、ｙ：２ ｝；整数 結果 ＝ ｐ：：ｘ；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_field_read_unknown_field() {
    let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；整数 結果 ＝ ｐ：：ｚ；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UnknownField { field, .. } if field == "ｚ"));
}

#[test]
fn test_typecheck_field_read_on_non_record_value() {
    let ast = parse("整数 値 ＝ ５；整数 結果 ＝ 値：：ｘ；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::NotARecord {
            got: HikariType::Int,
            ..
        }
    ));
}

#[test]
fn test_typecheck_record_construction_missing_field() {
    let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::MissingField { field, .. } if field == "ｙ"));
}

#[test]
fn test_typecheck_record_construction_extra_field() {
    let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１、ｚ：２ ｝；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UnknownField { field, .. } if field == "ｚ"));
}

#[test]
fn test_typecheck_record_construction_field_type_mismatch() {
    let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：「あ」 ｝；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    match err {
        TypeError::FieldTypeMismatch { expected, got, .. } => {
            assert_eq!(*expected, HikariType::Int);
            assert_eq!(*got, HikariType::String);
        }
        other => panic!("expected FieldTypeMismatch, got {:?}", other),
    }
}

#[test]
fn test_typecheck_field_assign_happy_path() {
    let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；ｐ：：ｘ ＝ ９；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_field_assign_wrong_value_type() {
    let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；ｐ：：ｘ ＝ 「あ」；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    match err {
        TypeError::FieldTypeMismatch { expected, got, .. } => {
            assert_eq!(*expected, HikariType::Int);
            assert_eq!(*got, HikariType::String);
        }
        other => panic!("expected FieldTypeMismatch, got {:?}", other),
    }
}

#[test]
fn test_typecheck_undeclared_type_in_construction() {
    let src = "点 ｐ ＝ 点 ｛ ｘ：１ ｝；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredType(n, _) if n == "点"));
}

#[test]
fn test_typecheck_undeclared_type_in_var_decl() {
    let src = "型 点 ｛ 整数 ｘ； ｝存在しない ｐ ＝ 点 ｛ ｘ：１ ｝；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredType(n, _) if n == "存在しない"));
}

// ── 9b: enums and pattern matching ──────────────────────────────────

#[test]
fn test_typecheck_variant_construction_happy_path() {
    let src = "構造 結果 ｛ 成功（整数）、 異常（文字列） ｝結果 値 ＝ 成功（１）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_variant_construction_zero_payload() {
    let src = "構造 信号 ｛ 赤、 黄、 青 ｝信号 値 ＝ 赤（）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_variant_construction_wrong_arg_count() {
    let src = "構造 結果 ｛ 成功（整数） ｝結果 値 ＝ 成功（）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ArgCountMismatch {
            expected: 1,
            got: 0,
            ..
        }
    ));
}

#[test]
fn test_typecheck_variant_construction_wrong_arg_type() {
    let src = "構造 結果 ｛ 成功（整数） ｝結果 値 ＝ 成功（「あ」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ArgTypeMismatch {
            param: HikariType::Int,
            got: HikariType::String,
            ..
        }
    ));
}

#[test]
fn test_typecheck_duplicate_enum_variant_across_enums() {
    let src = "構造 結果 ｛ 成功 ｝構造 状態 ｛ 成功 ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::DuplicateEnumVariant { variant, .. } if variant == "成功"));
}

#[test]
fn test_typecheck_match_exhaustive_is_ok() {
    let src = "構造 信号 ｛ 赤、 青 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ 印刷（１）； ｝ 青（） ならば ｛ 印刷（２）； ｝ ｝";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_match_non_exhaustive_lists_missing_variant() {
    let src = "構造 信号 ｛ 赤、 黄、 青 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ 印刷（１）； ｝ ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    match &err {
        TypeError::NonExhaustiveMatch(info) => {
            assert_eq!(info.missing, vec!["黄".to_string(), "青".to_string()]);
        }
        other => panic!("expected NonExhaustiveMatch, got {:?}", other),
    }
    assert!(err.to_string().contains("黄"));
    assert!(err.to_string().contains("青"));
}

#[test]
fn test_typecheck_match_duplicate_arm() {
    let src = "構造 信号 ｛ 赤、 青 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ ｝ 赤（） ならば ｛ ｝ 青（） ならば ｛ ｝ ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::DuplicateMatchArm { variant, .. } if variant == "赤"));
}

#[test]
fn test_typecheck_match_arm_from_different_enum_is_undeclared_variant() {
    let src = "構造 信号 ｛ 赤、 青 ｝構造 状態 ｛ 開始 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ ｝ 開始（） ならば ｛ ｝ ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::UndeclaredEnumVariant { enum_name, variant, .. }
        if enum_name == "信号" && variant == "開始"
    ));
}

#[test]
fn test_typecheck_match_arm_wrong_binder_count() {
    let src = "構造 結果 ｛ 成功（整数） ｝結果 値 ＝ 成功（１）；照合 値 ｛ 成功（ａ、ｂ） ならば ｛ ｝ ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ArgCountMismatch {
            expected: 1,
            got: 2,
            ..
        }
    ));
}

#[test]
fn test_typecheck_match_on_non_enum_value() {
    let src = "整数 値 ＝ ５；照合 値 ｛ ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::NotAnEnum {
            got: HikariType::Int,
            ..
        }
    ));
}

#[test]
fn test_typecheck_match_arm_binder_scoped_to_its_own_arm() {
    let src = "構造 結果 ｛ 成功（整数）、 異常（文字列） ｝結果 値 ＝ 成功（１）；照合 値 ｛ 成功（ｎ） ならば ｛ 印刷（ｎ）； ｝ 異常（ｅ） ならば ｛ 印刷（ｎ）； ｝ ｝";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "ｎ"));
}

#[test]
fn test_typecheck_match_binder_not_visible_after_match() {
    let src = "構造 結果 ｛ 成功（整数） ｝結果 値 ＝ 成功（１）；照合 値 ｛ 成功（ｎ） ならば ｛ 印刷（ｎ）； ｝ ｝印刷（ｎ）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "ｎ"));
}

#[test]
fn test_typecheck_void_call_result_cannot_be_used_as_value() {
    let src = "関数 表示（整数 Ａ）ー＞ 無 ｛ 印刷（Ａ）； ｝整数 結果 ＝ 表示（５）；";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(
        err,
        TypeError::VarDeclMismatch {
            got: HikariType::Void,
            ..
        }
    ));
}

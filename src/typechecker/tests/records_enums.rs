use super::*;

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
    assert!(matches!(err, TypeError::VoidValueUsed { .. }));
}

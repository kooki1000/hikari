use super::*;

// ── 9a: records ───────────────────────────────────────────────────────

#[test]
fn test_parse_type_decl() {
    let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝";
    let ast = parse_helper(src);
    assert!(matches!(
        &ast[0],
        Stmt::TypeDecl { name, fields, .. }
        if name == "点" && fields == &[
            (HikariType::Int, "ｘ".to_string()),
            (HikariType::Int, "ｙ".to_string()),
        ]
    ));
}

#[test]
fn test_parse_record_construction() {
    let src = "点 ｐ ＝ 点 ｛ ｘ：１、ｙ：２ ｝；";
    let ast = parse_helper(src);
    let Stmt::VarDecl {
        ty: HikariType::Record(tyname),
        value: Expr::RecordLit { type_name, fields },
        ..
    } = &ast[0]
    else {
        panic!("expected VarDecl with RecordLit")
    };
    assert_eq!(tyname, "点");
    assert_eq!(type_name, "点");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].0, "ｘ");
    assert_eq!(fields[1].0, "ｙ");
}

#[test]
fn test_parse_field_access() {
    let ast = parse_helper("返す ｐ：：ｘ；");
    assert!(matches!(
        &ast[0],
        Stmt::Return(Some(Expr::FieldAccess { record, field }), _)
        if matches!(record.as_ref(), Expr::Ident(n) if n == "ｐ") && field == "ｘ"
    ));
}

#[test]
fn test_parse_chained_field_access() {
    let ast = parse_helper("返す ａ：：ｂ：：ｃ；");
    let Stmt::Return(Some(Expr::FieldAccess { record, field }), _) = &ast[0] else {
        panic!("expected FieldAccess")
    };
    assert_eq!(field, "ｃ");
    let Expr::FieldAccess {
        record: inner_record,
        field: inner_field,
    } = record.as_ref()
    else {
        panic!("expected nested FieldAccess")
    };
    assert_eq!(inner_field, "ｂ");
    assert!(matches!(inner_record.as_ref(), Expr::Ident(n) if n == "ａ"));
}

#[test]
fn test_parse_field_assignment() {
    let ast = parse_helper("ｐ：：ｘ ＝ ９９；");
    assert!(matches!(
        &ast[0],
        Stmt::FieldAssign { record: Expr::Ident(n), field, value: Expr::LitInt(99), .. }
        if n == "ｐ" && field == "ｘ"
    ));
}

#[test]
fn test_parse_index_then_field_access_chain() {
    let ast = parse_helper("返す 配列【０】：：ｘ；");
    let Stmt::Return(Some(Expr::FieldAccess { record, field }), _) = &ast[0] else {
        panic!("expected FieldAccess")
    };
    assert_eq!(field, "ｘ");
    assert!(matches!(record.as_ref(), Expr::Index { .. }));
}

#[test]
fn test_parse_field_then_index_access_chain() {
    let ast = parse_helper("返す 点：：配列フィールド【０】；");
    let Stmt::Return(Some(Expr::Index { array, .. }), _) = &ast[0] else {
        panic!("expected Index")
    };
    assert!(matches!(array.as_ref(), Expr::FieldAccess { .. }));
}

#[test]
fn test_parse_type_decl_missing_semi_returns_error() {
    let src = "型 点 ｛ 整数 ｘ ｝";
    let tokens = Lexer::new(src).tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(err, ParseError::UnexpectedToken { .. }));
}

#[test]
fn test_parse_record_construction_missing_colon_returns_error() {
    let src = "点 ｐ ＝ 点 ｛ ｘ １ ｝；";
    let tokens = Lexer::new(src).tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(err, ParseError::UnexpectedToken { .. }));
}

#[test]
fn test_parse_type_decl_missing_field_name_returns_error() {
    let src = "型 点 ｛ 整数； ｝";
    let tokens = Lexer::new(src).tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(err, ParseError::ExpectedIdentifier { .. }));
}

// ── 9b: enums and pattern matching ──────────────────────────────────

#[test]
fn test_parse_enum_decl_with_payload_and_payloadless_variants() {
    let src = "構造 結果 ｛ 成功（整数）、 異常（文字列）、 不明 ｝";
    let ast = parse_helper(src);
    assert!(matches!(
        &ast[0],
        Stmt::EnumDecl { name, variants, .. }
        if name == "結果" && variants == &[
            ("成功".to_string(), vec![HikariType::Int]),
            ("異常".to_string(), vec![HikariType::String]),
            ("不明".to_string(), vec![]),
        ]
    ));
}

#[test]
fn test_parse_match_stmt_with_multiple_arms_including_zero_payload() {
    let src = "構造 結果 ｛ 成功（整数）、 異常 ｝照合 値 ｛ 成功（ｎ） ならば ｛ 印刷（ｎ）； ｝ 異常（） ならば ｛ 印刷（０）； ｝ ｝";
    let ast = parse_helper(src);
    let Stmt::Match { subject, arms, .. } = &ast[1] else {
        panic!("expected Match")
    };
    assert!(matches!(subject, Expr::Ident(n) if n == "値"));
    assert_eq!(arms.len(), 2);
    assert_eq!(arms[0].variant, "成功");
    assert_eq!(arms[0].binders, vec!["ｎ".to_string()]);
    assert_eq!(arms[0].body.len(), 1);
    assert_eq!(arms[1].variant, "異常");
    assert!(arms[1].binders.is_empty());
}

#[test]
fn test_parse_variant_construction_is_ordinary_call() {
    let ast = parse_helper("返す 成功（１２３）；");
    assert!(matches!(
        &ast[0],
        Stmt::Return(Some(Expr::Call { name, args }), _)
        if name == "成功" && args.len() == 1
    ));
}

#[test]
fn test_parse_zero_payload_variant_construction_requires_parens() {
    let ast = parse_helper("返す 異常（）；");
    assert!(matches!(
        &ast[0],
        Stmt::Return(Some(Expr::Call { name, args }), _)
        if name == "異常" && args.is_empty()
    ));
}

#[test]
fn test_parse_enum_decl_missing_comma_between_variants_returns_error() {
    let src = "構造 結果 ｛ 成功 異常 ｝";
    let tokens = Lexer::new(src).tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(err, ParseError::UnexpectedToken { .. }));
}

#[test]
fn test_parse_match_arm_missing_then_keyword_returns_error() {
    let src = "照合 値 ｛ 成功（） ｛ 印刷（０）； ｝ ｝";
    let tokens = Lexer::new(src).tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedToken {
            expected: TokenKind::KwThen,
            ..
        }
    ));
}

#[test]
fn test_parse_match_arm_missing_parens_returns_error() {
    let src = "照合 値 ｛ 成功 ならば ｛ 印刷（０）； ｝ ｝";
    let tokens = Lexer::new(src).tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedToken {
            expected: TokenKind::LParen,
            ..
        }
    ));
}

#[test]
fn test_parse_new_array_expr() {
    let tokens = Lexer::new("整数列 数字 ＝ 新配列＜整数＞；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::VarDecl {
            ty: HikariType::Array(inner),
            value: Expr::NewArray(elem_ty),
            ..
        }
        if **inner == HikariType::Int && *elem_ty == HikariType::Int
    ));
}

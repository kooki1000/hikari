use super::error::ParseError;
use super::*;
use crate::lexer::{Lexer, TokenKind};

#[test]
fn test_parse_var_decl() {
    let tokens = Lexer::new("整数 年齢 ＝ ２０；").tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    assert_eq!(ast.len(), 1);
    assert!(matches!(
        &ast[0],
        Stmt::VarDecl { ty: HikariType::Int, name, value: Expr::LitInt(20), .. }
        if name == "年齢"
    ));
}

#[test]
fn test_parse_binary_expression() {
    // 整数 結果 ＝ １ ＋ ２；
    let tokens = Lexer::new("整数 結果 ＝ １ ＋ ２；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::VarDecl {
            ty: HikariType::Int,
            name,
            value: Expr::BinOp { op: BinOpKind::Add, lhs, rhs },
            ..
        }
        if name == "結果"
            && matches!(lhs.as_ref(), Expr::LitInt(1))
            && matches!(rhs.as_ref(), Expr::LitInt(2))
    ));
}

#[test]
fn test_parse_operator_precedence() {
    // 整数 結果 ＝ ２ ＋ ３ ＊ ４；
    // Should parse as 2 + (3 * 4), not (2 + 3) * 4
    let tokens = Lexer::new("整数 結果 ＝ ２ ＋ ３ ＊ ４；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    let Stmt::VarDecl { value, .. } = &ast[0] else {
        panic!()
    };
    // outer op must be Add
    let Expr::BinOp { op, lhs, rhs } = value else {
        panic!()
    };
    assert_eq!(op, &BinOpKind::Add);
    assert!(matches!(lhs.as_ref(), Expr::LitInt(2)));
    // rhs must be Mul(3, 4)
    let Expr::BinOp {
        op: inner_op,
        lhs: il,
        rhs: ir,
    } = rhs.as_ref()
    else {
        panic!()
    };
    assert_eq!(inner_op, &BinOpKind::Mul);
    assert!(matches!(il.as_ref(), Expr::LitInt(3)));
    assert!(matches!(ir.as_ref(), Expr::LitInt(4)));
}

#[test]
fn test_parse_return_stmt() {
    // 返す 年齢 ＋ １；
    let tokens = Lexer::new("返す 年齢 ＋ １；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::Return(Some(Expr::BinOp { op: BinOpKind::Add, lhs, rhs }), _)
        if matches!(lhs.as_ref(), Expr::Ident(n) if n == "年齢")
            && matches!(rhs.as_ref(), Expr::LitInt(1))
    ));
}

#[test]
fn test_parse_fn_decl() {
    // 関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
    let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝";
    let tokens = Lexer::new(src).tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert_eq!(ast.len(), 1);
    let Stmt::FnDecl {
        name,
        params,
        return_ty,
        body,
        ..
    } = &ast[0]
    else {
        panic!("expected FnDecl")
    };
    assert_eq!(name, "計算");
    assert_eq!(params, &[(HikariType::Int, "Ａ".to_string())]);
    assert_eq!(return_ty, &HikariType::Int);
    assert_eq!(body.len(), 1);
    assert!(matches!(
        &body[0],
        Stmt::Return(
            Some(Expr::BinOp {
                op: BinOpKind::Add,
                ..
            }),
            _
        )
    ));
}

#[test]
fn test_parse_if_stmt() {
    // もし １ ＝＝ １ ならば ｛ 印刷（１）； ｝
    let src = "もし １ ＝＝ １ ならば ｛ 印刷（１）； ｝";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    assert_eq!(ast.len(), 1);
    let Stmt::If {
        condition,
        then_body,
        else_body,
        ..
    } = &ast[0]
    else {
        panic!("expected If stmt")
    };
    assert!(matches!(
        condition,
        Expr::BinOp {
            op: BinOpKind::Eq,
            ..
        }
    ));
    assert_eq!(then_body.len(), 1);
    assert!(matches!(then_body[0], Stmt::Print(_, _)));
    assert!(else_body.is_none());
}

#[test]
fn test_parse_if_else_stmt() {
    // もし Ａ ＝＝ ０ ならば ｛ 印刷（１）； ｝ 違えば ｛ 印刷（２）； ｝
    let src = "もし Ａ ＝＝ ０ ならば ｛ 印刷（１）； ｝ 違えば ｛ 印刷（２）； ｝";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let Stmt::If { else_body, .. } = &ast[0] else {
        panic!()
    };
    assert!(else_body.is_some());
    assert_eq!(else_body.as_ref().unwrap().len(), 1);
}

#[test]
fn test_parse_while_stmt() {
    // 間 カウンタ ＜ ３ ならば ｛ 印刷（カウンタ）； ｝
    let src = "間 カウンタ ＜ ３ ならば ｛ 印刷（カウンタ）； ｝";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    assert_eq!(ast.len(), 1);
    let Stmt::While {
        condition, body, ..
    } = &ast[0]
    else {
        panic!("expected While stmt")
    };
    assert!(matches!(
        condition,
        Expr::BinOp {
            op: BinOpKind::Lt,
            ..
        }
    ));
    assert_eq!(body.len(), 1);
    assert!(matches!(body[0], Stmt::Print(_, _)));
}

#[test]
fn test_parse_bool_literals() {
    // 真偽 フラグ ＝ 真；
    let tokens = Lexer::new("真偽 フラグ ＝ 真；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::VarDecl {
            ty: HikariType::Bool,
            value: Expr::LitBool(true),
            ..
        }
    ));

    // 真偽 フラグ ＝ 偽；
    let tokens = Lexer::new("真偽 フラグ ＝ 偽；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::VarDecl {
            ty: HikariType::Bool,
            value: Expr::LitBool(false),
            ..
        }
    ));
}

#[test]
fn test_parse_print_stmt() {
    // 印刷（年齢）；
    let tokens = Lexer::new("印刷（年齢）；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert_eq!(ast.len(), 1);
    assert!(matches!(
        &ast[0],
        Stmt::Print(Expr::Ident(n), _) if n == "年齢"
    ));
}

#[test]
fn test_parse_missing_semicolon_returns_error() {
    // 整数 年齢 ＝ ２０  （no trailing ；）
    let tokens = Lexer::new("整数 年齢 ＝ ２０").tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedToken {
            expected: TokenKind::Semi,
            got: TokenKind::Eof,
            ..
        }
    ));
}

#[test]
fn test_parse_missing_identifier_returns_error() {
    // 整数 ＝ ２０；  （missing variable name）
    let tokens = Lexer::new("整数 ＝ ２０；").tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(
        err,
        ParseError::ExpectedIdentifier {
            got: TokenKind::Assign,
            ..
        }
    ));
}

#[test]
fn test_parse_reassignment() {
    // 整数 年齢 ＝ ２０； 年齢 ＝ ３０；
    let tokens = Lexer::new("整数 年齢 ＝ ２０；年齢 ＝ ３０；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert_eq!(ast.len(), 2);
    assert!(matches!(
        &ast[1],
        Stmt::Assign { name, value: Expr::LitInt(30), .. } if name == "年齢"
    ));
}

#[test]
fn test_parse_multi_param_fn_decl() {
    // 関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝
    let src = "関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let Stmt::FnDecl { params, .. } = &ast[0] else {
        panic!("expected FnDecl")
    };
    assert_eq!(
        params,
        &[
            (HikariType::Int, "Ａ".to_string()),
            (HikariType::Int, "Ｂ".to_string())
        ]
    );
}

#[test]
fn test_parse_multi_arg_call() {
    // 関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝
    // 返す 加算（１、２）；
    let src = "関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝返す 加算（１、２）；";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let Stmt::Return(Some(Expr::Call { name, args }), _) = &ast[1] else {
        panic!("expected Return(Call)")
    };
    assert_eq!(name, "加算");
    assert_eq!(args.len(), 2);
}

#[test]
fn test_parse_unary_minus() {
    // 整数 結果 ＝ ー５；
    let tokens = Lexer::new("整数 結果 ＝ ー５；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::VarDecl { value: Expr::UnaryMinus(inner), .. }
        if matches!(inner.as_ref(), Expr::LitInt(5))
    ));
}

#[test]
fn test_parse_unary_minus_in_addition() {
    // 整数 結果 ＝ １０ ＋ ー３；
    let tokens = Lexer::new("整数 結果 ＝ １０ ＋ ー３；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    let Stmt::VarDecl { value, .. } = &ast[0] else {
        panic!()
    };
    let Expr::BinOp { op, rhs, .. } = value else {
        panic!()
    };
    assert_eq!(op, &BinOpKind::Add);
    assert!(matches!(rhs.as_ref(), Expr::UnaryMinus(_)));
}

#[test]
fn test_parse_logical_and_or_precedence() {
    // １ ＝＝ １ かつ ２ ＝＝ ２
    let tokens = Lexer::new("返す １ ＝＝ １ かつ ２ ＝＝ ２；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    let Stmt::Return(Some(expr), _) = &ast[0] else {
        panic!()
    };
    let Expr::BinOp { op, lhs, rhs } = expr else {
        panic!()
    };
    assert_eq!(op, &BinOpKind::And);
    assert!(matches!(
        lhs.as_ref(),
        Expr::BinOp {
            op: BinOpKind::Eq,
            ..
        }
    ));
    assert!(matches!(
        rhs.as_ref(),
        Expr::BinOp {
            op: BinOpKind::Eq,
            ..
        }
    ));
}

#[test]
fn test_parse_unary_not() {
    // 返す 否定 真；
    let tokens = Lexer::new("返す 否定 真；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::Return(Some(Expr::UnaryNot(inner)), _) if matches!(inner.as_ref(), Expr::LitBool(true))
    ));
}

#[test]
fn test_parse_additional_comparison_operators() {
    let tokens = Lexer::new("返す ３ ≦ ３；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::Return(
            Some(Expr::BinOp {
                op: BinOpKind::LtEq,
                ..
            }),
            _
        )
    ));
}

#[test]
fn test_parse_unexpected_expr_token_returns_error() {
    // 整数 結果 ＝ ；  （missing expression before semicolon）
    let tokens = Lexer::new("整数 結果 ＝ ；").tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedExprToken {
            got: TokenKind::Semi,
            ..
        }
    ));
}

#[test]
fn test_parse_array_literal() {
    let tokens = Lexer::new("整数列 数字 ＝ 【１、２、３】；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::VarDecl {
            ty: HikariType::Array(inner),
            value: Expr::Array(elems),
            ..
        }
        if **inner == HikariType::Int && elems.len() == 3
    ));
}

#[test]
fn test_parse_index_expr() {
    let tokens = Lexer::new("返す 数字【１】；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::Return(Some(Expr::Index { array, index }), _)
        if matches!(array.as_ref(), Expr::Ident(n) if n == "数字")
            && matches!(index.as_ref(), Expr::LitInt(1))
    ));
}

#[test]
fn test_parse_index_assign() {
    let tokens = Lexer::new("数字【０】＝ ９９；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::IndexAssign { name, index: Expr::LitInt(0), value: Expr::LitInt(99), .. }
        if name == "数字"
    ));
}

#[test]
fn test_parse_for_range_stmt() {
    let src = "繰り返す カウンタ ＝ ０ から ５ ならば ｛ 印刷（カウンタ）； ｝";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::ForRange { var, from: Expr::LitInt(0), to: Expr::LitInt(5), body, .. }
        if var == "カウンタ" && body.len() == 1
    ));
}

#[test]
fn test_parse_for_each_stmt() {
    let src = "各 要素 ： 数字 ならば ｛ 印刷（要素）； ｝";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::ForEach { var, array: Expr::Ident(arr_name), body, .. }
        if var == "要素" && arr_name == "数字" && body.len() == 1
    ));
}

#[test]
fn test_parse_try_catch_stmt() {
    let src = "試す ｛ 印刷（１）； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::TryCatch { try_body, error_var, catch_body, .. }
        if try_body.len() == 1 && error_var == "失敗内容" && catch_body.len() == 1
    ));
}

#[test]
fn test_parse_try_catch_missing_error_var_returns_error() {
    let src = "試す ｛ 印刷（１）； ｝ 失敗 ｛ 印刷（１）； ｝";
    let tokens = Lexer::new(src).tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(err, ParseError::ExpectedIdentifier { .. }));
}

#[test]
fn test_parse_import_stmt() {
    let src = "取り込む 「数学」；";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::Import { name, .. } if name == "数学"
    ));
}

#[test]
fn test_parse_import_missing_string_literal_returns_error() {
    let src = "取り込む 数学；";
    let tokens = Lexer::new(src).tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(err, ParseError::UnexpectedToken { .. }));
}

#[test]
fn test_parse_import_missing_semi_returns_error() {
    let src = "取り込む 「数学」";
    let tokens = Lexer::new(src).tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedToken {
            expected: TokenKind::Semi,
            ..
        }
    ));
}

#[test]
fn test_parse_malformed_number_returns_invalid_number_error() {
    let src = "整数 Ｘ ＝ １．２．３；";
    let tokens = Lexer::new(src).tokenize();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(err, ParseError::InvalidNumber { .. }));
}

#[test]
fn test_parse_modulo_precedence() {
    // １０ ％ ３ ＋ １ should parse as (10 % 3) + 1, Mod binding tighter than Add.
    let tokens = Lexer::new("返す １０ ％ ３ ＋ １；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    let Stmt::Return(Some(expr), _) = &ast[0] else {
        panic!()
    };
    let Expr::BinOp { op, lhs, rhs } = expr else {
        panic!()
    };
    assert_eq!(op, &BinOpKind::Add);
    assert!(matches!(rhs.as_ref(), Expr::LitInt(1)));
    let Expr::BinOp {
        op: inner_op,
        lhs: il,
        rhs: ir,
    } = lhs.as_ref()
    else {
        panic!()
    };
    assert_eq!(inner_op, &BinOpKind::Mod);
    assert!(matches!(il.as_ref(), Expr::LitInt(10)));
    assert!(matches!(ir.as_ref(), Expr::LitInt(3)));
}

#[test]
fn test_parse_break_stmt() {
    let ast = parse_helper("間 真 ならば ｛ 抜ける； ｝");
    let Stmt::While { body, .. } = &ast[0] else {
        panic!()
    };
    assert!(matches!(body[0], Stmt::Break(_)));
}

#[test]
fn test_parse_continue_stmt() {
    let ast = parse_helper("間 真 ならば ｛ 続ける； ｝");
    let Stmt::While { body, .. } = &ast[0] else {
        panic!()
    };
    assert!(matches!(body[0], Stmt::Continue(_)));
}

#[test]
fn test_parse_bare_return() {
    let ast = parse_helper("返す；");
    assert!(matches!(ast[0], Stmt::Return(None, _)));
}

#[test]
fn test_parse_return_with_expr_still_works() {
    let ast = parse_helper("返す ５；");
    assert!(matches!(ast[0], Stmt::Return(Some(Expr::LitInt(5)), _)));
}

fn parse_helper(src: &str) -> Vec<Stmt> {
    Parser::new(Lexer::new(src).tokenize()).parse().unwrap()
}

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

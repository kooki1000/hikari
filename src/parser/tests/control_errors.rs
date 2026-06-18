use super::*;

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

// ── recursion-depth limit (Phase 12 fuzz hardening) ──────────────────

#[test]
fn test_parse_deeply_nested_parens_errors_not_overflows() {
    // Thousands of nested parens used to overflow the parser's stack; now it
    // is a clean error.
    let src = format!("整数 ａ ＝ {}１{}；", "（".repeat(5000), "）".repeat(5000));
    let err = Parser::new(Lexer::new(&src).tokenize())
        .parse()
        .unwrap_err();
    assert!(matches!(err, ParseError::TooDeeplyNested { .. }));
}

#[test]
fn test_parse_deeply_nested_blocks_errors_not_overflows() {
    let src = "もし 真 ならば ｛".repeat(5000);
    let err = Parser::new(Lexer::new(&src).tokenize())
        .parse()
        .unwrap_err();
    assert!(matches!(err, ParseError::TooDeeplyNested { .. }));
}

#[test]
fn test_parse_modestly_nested_input_is_accepted() {
    // Ordinary nesting depth must still parse fine.
    let src = format!("整数 ａ ＝ {}１{}；", "（".repeat(20), "）".repeat(20));
    assert!(Parser::new(Lexer::new(&src).tokenize()).parse().is_ok());
}

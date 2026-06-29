use super::*;

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
        Stmt::Print(exprs, _) if matches!(exprs.as_slice(), [Expr::Ident(n)] if n == "年齢")
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
    let Stmt::Return(Some(Expr::Call { name, args, .. }), _) = &ast[1] else {
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
fn test_parse_print_multiple_values() {
    let tokens = Lexer::new("印刷（年齢、「歳」）；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::Print(exprs, _) if exprs.len() == 2
    ));
}

#[test]
fn test_parse_print_no_values() {
    let tokens = Lexer::new("印刷（）；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(&ast[0], Stmt::Print(exprs, _) if exprs.is_empty()));
}

// ── 21b: i64::MIN literal ─────────────────────────────────────────────────────

#[test]
fn test_parse_i64_min_literal() {
    // ー followed by the magnitude of i64::MIN should produce i64::MIN directly.
    let tokens = Lexer::new("整数 ｘ ＝ ー９２２３３７２０３６８５４７７５８０８；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::VarDecl { value: Expr::LitInt(n), .. } if *n == i64::MIN
    ));
}

#[test]
fn test_parse_i64_min_as_expression() {
    // As a standalone expression statement.
    let tokens = Lexer::new("ー９２２３３７２０３６８５４７７５８０８；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::Expr(Expr::LitInt(n), _) if *n == i64::MIN
    ));
}

// ── 21c: empty array literal inference ───────────────────────────────────────

#[test]
fn test_parse_empty_array_literal() {
    // 【】 should parse successfully (no ParseError).
    let tokens = Lexer::new("整数列 ａ ＝ 【】；").tokenize();
    let ast = Parser::new(tokens).parse().unwrap();
    assert!(matches!(
        &ast[0],
        Stmt::VarDecl { value: Expr::Array(elems), .. } if elems.is_empty()
    ));
}

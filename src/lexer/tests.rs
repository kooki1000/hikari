use super::*;

#[test]
fn test_lex_integer_keyword() {
    let tokens = Lexer::new("整数").tokenize();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::TyInt);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_lex_all_keywords() {
    let src = "整数 小数 文字列 真偽 無 関数 返す";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            &TokenKind::TyInt,
            &TokenKind::TyFloat,
            &TokenKind::TyString,
            &TokenKind::TyBool,
            &TokenKind::TyVoid,
            &TokenKind::KwFn,
            &TokenKind::KwReturn,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_full_width_integer_literal() {
    let tokens = Lexer::new("２０").tokenize();
    assert_eq!(tokens[0].kind, TokenKind::LitInt(20));
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_lex_full_width_operators() {
    let src = "＋ ー ＊ ／ ＝ ；";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            &TokenKind::Plus,
            &TokenKind::Minus,
            &TokenKind::Star,
            &TokenKind::Slash,
            &TokenKind::Assign,
            &TokenKind::Semi,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_return_arrow() {
    // ー＞ must be a single Arrow token, not Minus followed by something.
    let tokens = Lexer::new("ー＞").tokenize();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Arrow);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_lex_minus_not_arrow() {
    // A lone ー (not followed by ＞) must remain Minus.
    let tokens = Lexer::new("ー").tokenize();
    assert_eq!(tokens[0].kind, TokenKind::Minus);
}

#[test]
fn test_lex_identifier_containing_long_vowel_mark() {
    // ー is a common katakana long vowel mark in loanwords (エラー =
    // "error"); it must stay part of the identifier when it's not the
    // first character of a token.
    let tokens = Lexer::new("エラー").tokenize();
    assert_eq!(tokens[0].kind, TokenKind::Ident("エラー".to_string()));
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_lex_string_literal() {
    let tokens = Lexer::new("「こんにちは」").tokenize();
    assert_eq!(
        tokens[0].kind,
        TokenKind::LitString("こんにちは".to_string())
    );
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_lex_variable_declaration() {
    // 整数 年齢 ＝ ２０；
    let src = "整数 年齢 ＝ ２０；";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::TyInt,
            TokenKind::Ident("年齢".to_string()),
            TokenKind::Assign,
            TokenKind::LitInt(20),
            TokenKind::Semi,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_function_declaration() {
    // 関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
    let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::KwFn,
            TokenKind::Ident("計算".to_string()),
            TokenKind::LParen,
            TokenKind::TyInt,
            TokenKind::Ident("Ａ".to_string()),
            TokenKind::RParen,
            TokenKind::Arrow,
            TokenKind::TyInt,
            TokenKind::LBrace,
            TokenKind::KwReturn,
            TokenKind::Ident("Ａ".to_string()),
            TokenKind::Plus,
            TokenKind::LitInt(1),
            TokenKind::Semi,
            TokenKind::RBrace,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_print_keyword() {
    let tokens = Lexer::new("印刷").tokenize();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::KwPrint);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_lex_if_keywords() {
    let src = "もし ならば 違えば";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            &TokenKind::KwIf,
            &TokenKind::KwThen,
            &TokenKind::KwElse,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_while_keyword() {
    let tokens = Lexer::new("間").tokenize();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::KwWhile);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_lex_comparison_operators() {
    let src = "＝＝ ＜ ＞";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            &TokenKind::EqEq,
            &TokenKind::Lt,
            &TokenKind::Gt,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_comment_skipped() {
    let src = "＃ これはコメントです\n整数 年齢 ＝ ２０；";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::TyInt,
            TokenKind::Ident("年齢".to_string()),
            TokenKind::Assign,
            TokenKind::LitInt(20),
            TokenKind::Semi,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_comment_at_eof_with_no_trailing_newline() {
    let tokens = Lexer::new("＃ comment only").tokenize();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

#[test]
fn test_lex_comma_token() {
    let tokens = Lexer::new("、").tokenize();
    assert_eq!(tokens[0].kind, TokenKind::Comma);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_lex_extended_comparison_operators() {
    let src = "≦ ≧ ≠";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            &TokenKind::LtEq,
            &TokenKind::GtEq,
            &TokenKind::NotEq,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_logical_keywords() {
    let src = "かつ または 否定";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            &TokenKind::KwAnd,
            &TokenKind::KwOr,
            &TokenKind::KwNot,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_array_type_keywords() {
    let src = "整数列 小数列 文字列列 真偽列";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            &TokenKind::TyIntArray,
            &TokenKind::TyFloatArray,
            &TokenKind::TyStringArray,
            &TokenKind::TyBoolArray,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_bracket_tokens() {
    let tokens = Lexer::new("【１、２】").tokenize();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::LBracket,
            TokenKind::LitInt(1),
            TokenKind::Comma,
            TokenKind::LitInt(2),
            TokenKind::RBracket,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_try_catch_keywords() {
    let src = "試す 失敗";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![&TokenKind::KwTry, &TokenKind::KwCatch, &TokenKind::Eof,]
    );
}

#[test]
fn test_lex_for_range_and_each_keywords() {
    let src = "繰り返す から 各 ：";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            &TokenKind::KwForRange,
            &TokenKind::KwFrom,
            &TokenKind::KwEach,
            &TokenKind::Colon,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_import_keyword() {
    let src = "取り込む 「数学」；";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            &TokenKind::KwImport,
            &TokenKind::LitString("数学".to_string()),
            &TokenKind::Semi,
            &TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_overflowing_integer_is_invalid_token() {
    // Too large for i64: a clean Invalid token rather than a panic.
    let tokens = Lexer::new("９９９９９９９９９９９９９９９９９９９９").tokenize();
    assert_eq!(
        tokens[0].kind,
        TokenKind::Invalid("99999999999999999999".to_string())
    );
}

#[test]
fn test_lex_malformed_number_is_invalid_token() {
    // Two decimal points: an Invalid token rather than a panic.
    let tokens = Lexer::new("１．２．３").tokenize();
    assert_eq!(tokens[0].kind, TokenKind::Invalid("1.2.3".to_string()));
}

#[test]
fn test_lex_percent_token() {
    let tokens = Lexer::new("１０ ％ ３").tokenize();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::LitInt(10),
            TokenKind::Percent,
            TokenKind::LitInt(3),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_lex_break_continue_keywords() {
    let src = "抜ける 続ける";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![&TokenKind::KwBreak, &TokenKind::KwContinue, &TokenKind::Eof,]
    );
}

#[test]
fn test_lex_type_keyword() {
    let tokens = Lexer::new("型").tokenize();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::KwType);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_lex_enum_and_match_keywords() {
    let src = "構造 照合";
    let tokens = Lexer::new(src).tokenize();
    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        vec![&TokenKind::KwEnum, &TokenKind::KwMatch, &TokenKind::Eof,]
    );
}

#[test]
fn test_lex_new_array_keyword() {
    let tokens = Lexer::new("新配列＜整数＞").tokenize();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::KwNewArray,
            TokenKind::Lt,
            TokenKind::TyInt,
            TokenKind::Gt,
            TokenKind::Eof,
        ]
    );
}

use super::*;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn warnings(src: &str) -> Vec<Warning> {
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    check(&ast)
}

fn messages(src: &str) -> Vec<String> {
    warnings(src).into_iter().map(|w| w.message).collect()
}

// ── unused variables ─────────────────────────────────────────────────

#[test]
fn test_lint_unused_local_is_warned() {
    let msgs = messages("整数 ａ ＝ ５；");
    assert_eq!(msgs, vec!["未使用の変数「ａ」です。".to_string()]);
}

#[test]
fn test_lint_used_local_is_not_warned() {
    assert!(messages("整数 ａ ＝ ５；印刷（ａ）；").is_empty());
}

#[test]
fn test_lint_used_only_in_expression_is_not_warned() {
    assert!(messages("整数 ａ ＝ ５；整数 ｂ ＝ ａ ＋ １；印刷（ｂ）；").is_empty());
}

#[test]
fn test_lint_variable_only_assigned_never_read_is_warned() {
    // Declared and reassigned but never read → still unused.
    let msgs = messages("整数 ａ ＝ ５；ａ ＝ ６；");
    assert_eq!(msgs, vec!["未使用の変数「ａ」です。".to_string()]);
}

#[test]
fn test_lint_function_parameters_are_exempt() {
    // An unused parameter is not warned about.
    assert!(messages("関数 ｆ（整数 ｎ）ー＞ 整数 ｛ 返す ０； ｝").is_empty());
}

#[test]
fn test_lint_loop_variable_is_exempt() {
    assert!(messages("繰り返す ｉ ＝ ０ から ３ ならば ｛ 印刷（０）； ｝").is_empty());
}

#[test]
fn test_lint_unused_inside_function_body_is_warned() {
    let msgs = messages("関数 ｆ（）ー＞ 整数 ｛ 整数 ｘ ＝ １； 返す ０； ｝");
    assert!(msgs.iter().any(|m| m == "未使用の変数「ｘ」です。"));
}

#[test]
fn test_lint_shadowed_inner_use_does_not_mark_outer_used() {
    // Inner ａ is used; outer ａ is not — only the outer should warn.
    let src = "整数 ａ ＝ １；もし 真 ならば ｛ 整数 ａ ＝ ２； 印刷（ａ）； ｝";
    assert_eq!(messages(src), vec!["未使用の変数「ａ」です。".to_string()]);
}

#[test]
fn test_lint_closure_capture_counts_as_use() {
    // ｂ is read only inside the lambda body (a capture); still "used".
    let src = "整数 ｂ ＝ １０；関数＜（整数） ー＞ 整数＞ ｆ ＝ ｜ｎ：整数｜ ー＞ 整数 ｛ 返す ｎ ＋ ｂ； ｝；印刷（ｆ（１））；";
    assert!(messages(src).is_empty());
}

// ── unreachable code ─────────────────────────────────────────────────

#[test]
fn test_lint_unreachable_after_return_is_warned() {
    let msgs = messages("関数 ｆ（）ー＞ 整数 ｛ 返す １； 印刷（２）； ｝");
    assert!(msgs.iter().any(|m| m == "この後の文には到達しません。"));
}

#[test]
fn test_lint_unreachable_after_break_is_warned() {
    let src = "間 真 ならば ｛ 抜ける； 印刷（１）； ｝";
    let msgs = messages(src);
    assert!(msgs.iter().any(|m| m == "この後の文には到達しません。"));
}

#[test]
fn test_lint_return_at_end_of_block_is_not_unreachable() {
    assert!(messages("関数 ｆ（）ー＞ 整数 ｛ 整数 ｘ ＝ １； 返す ｘ； ｝").is_empty());
}

#[test]
fn test_lint_clean_program_has_no_warnings() {
    let src = "整数 合計 ＝ ０；繰り返す ｉ ＝ ０ から ５ ならば ｛ 合計 ＝ 合計 ＋ ｉ； ｝印刷（合計）；";
    assert!(messages(src).is_empty());
}

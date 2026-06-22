// Phase 18 — module & namespace integration tests.
// These tests write real .hkr library files to the temp dir, import them with
// `取り込む 「…」 として エイリアス;`, then run the resulting program.

use std::collections::HashSet;
use std::path::Path;

use crate::compiler::{Compiler, Value};
use crate::lexer::Lexer;
use crate::modules::resolve_imports;
use crate::parser::Parser;
use crate::vm::Vm;

fn run_with_imports(src: &str, base: &Path) -> Option<Value> {
    let raw = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let mut visited = HashSet::new();
    let resolved = resolve_imports(raw, base, &mut visited).unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&resolved).unwrap();
    Vm::with_chunks(compiler.constants, compiler.chunks, script)
        .run()
        .unwrap()
}

// ── 18a: basic aliased import ─────────────────────────────────────────────────

#[test]
fn test_vm_aliased_import_simple_call() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("vm18_simple_{}.hkr", std::process::id()));
    std::fs::write(
        &path,
        "公開 関数 二倍（整数 ｎ）ー＞整数｛ 返す ｎ ＊ ２； ｝",
    )
    .unwrap();

    let src = format!(
        "取り込む 「{}」 として 算；返す 算。二倍（２１）；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let result = run_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(result, Some(Value::Int(42)));
}

#[test]
fn test_vm_aliased_import_multiple_calls() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("vm18_multi_{}.hkr", std::process::id()));
    std::fs::write(
        &path,
        "公開 関数 加算（整数 ａ、整数 ｂ）ー＞整数｛ 返す ａ ＋ ｂ； ｝\
         公開 関数 乗算（整数 ａ、整数 ｂ）ー＞整数｛ 返す ａ ＊ ｂ； ｝",
    )
    .unwrap();

    let src = format!(
        "取り込む 「{}」 として 算；\
         整数 和 ＝ 算。加算（３、４）；\
         整数 積 ＝ 算。乗算（３、４）；\
         返す 和 ＋ 積；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let result = run_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    // 3+4=7, 3*4=12, total=19
    assert_eq!(result, Some(Value::Int(19)));
}

// ── 18c: export control ───────────────────────────────────────────────────────

#[test]
fn test_vm_public_fn_is_callable() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("vm18_pub_{}.hkr", std::process::id()));
    std::fs::write(
        &path,
        "公開 関数 こんにちは（）ー＞整数｛ 返す ９９； ｝",
    )
    .unwrap();

    let src = format!(
        "取り込む 「{}」 として 模；返す 模。こんにちは（）；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let result = run_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(result, Some(Value::Int(99)));
}

#[test]
fn test_vm_internal_private_call_executes_correctly() {
    // 公開 function delegates to a private helper — both run, result is correct.
    let dir = std::env::temp_dir();
    let path = dir.join(format!("vm18_priv_{}.hkr", std::process::id()));
    std::fs::write(
        &path,
        "公開 関数 結果（整数 ｎ）ー＞整数｛ 返す 内部（ｎ）； ｝\
         関数 内部（整数 ｎ）ー＞整数｛ 返す ｎ ＊ ｎ； ｝",
    )
    .unwrap();

    let src = format!(
        "取り込む 「{}」 として 数；返す 数。結果（７）；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let result = run_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(result, Some(Value::Int(49)));
}

// ── 18b: library uses stdlib internally ──────────────────────────────────────

#[test]
fn test_vm_aliased_import_with_stdlib_usage() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("vm18_stdlib_{}.hkr", std::process::id()));
    // Library uses 「数学」 internally — its stdlib import must propagate.
    std::fs::write(
        &path,
        "取り込む 「数学」；公開 関数 累乗二（整数 ｎ）ー＞整数｛ 返す 累乗（ｎ、２）； ｝",
    )
    .unwrap();

    let src = format!(
        "取り込む 「{}」 として 算；返す 算。累乗二（６）；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let result = run_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(result, Some(Value::Int(36)));
}

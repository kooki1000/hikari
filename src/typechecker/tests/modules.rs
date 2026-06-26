use super::*;
use crate::modules::resolve_imports;
use std::collections::HashSet;

fn typecheck_with_imports(src: &str, base: &std::path::Path) -> Result<(), TypeError> {
    let raw = parse(src);
    let mut visited = HashSet::new();
    let resolved = resolve_imports(raw, base, &mut visited)
        .map_err(|msg| panic!("resolve_imports failed: {}", msg))
        .unwrap();
    TypeChecker::new().check(&resolved)
}

// ── 18a: aliased imports ──────────────────────────────────────────────────────

#[test]
fn test_aliased_import_call_type_checks() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("tc_alias_{}.hkr", std::process::id()));
    std::fs::write(
        &path,
        "公開 関数 加算（整数 ａ、整数 ｂ）ー＞整数｛ 返す ａ ＋ ｂ； ｝",
    )
    .unwrap();

    let src = format!(
        "取り込む 「{}」 として 算；整数 結果 ＝ 算。加算（１、２）；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let res = typecheck_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    assert!(
        res.is_ok(),
        "aliased qualified call should type-check: {:?}",
        res
    );
}

#[test]
fn test_aliased_import_arg_type_mismatch_is_caught() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("tc_alias_mismatch_{}.hkr", std::process::id()));
    std::fs::write(
        &path,
        "公開 関数 加算（整数 ａ、整数 ｂ）ー＞整数｛ 返す ａ ＋ ｂ； ｝",
    )
    .unwrap();

    // Pass a String where Int is expected.
    let src = format!(
        "取り込む 「{}」 として 算；整数 結果 ＝ 算。加算（「こんにちは」、２）；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let res = typecheck_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    assert!(
        matches!(res, Err(TypeError::ArgTypeMismatch { .. })),
        "type mismatch should be caught for aliased call: {:?}",
        res
    );
}

#[test]
fn test_unaliased_import_still_works() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("tc_unaliased_{}.hkr", std::process::id()));
    std::fs::write(&path, "関数 旧来（整数 ｎ）ー＞整数｛ 返す ｎ ＊ ２； ｝").unwrap();

    let src = format!(
        "取り込む 「{}」；整数 結果 ＝ 旧来（５）；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let res = typecheck_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    assert!(
        res.is_ok(),
        "unaliased (flat) import must still work: {:?}",
        res
    );
}

// ── 18c: export control ───────────────────────────────────────────────────────

#[test]
fn test_private_fn_call_from_outside_is_rejected() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("tc_private_{}.hkr", std::process::id()));
    std::fs::write(
        &path,
        "公開 関数 公開関数（）ー＞整数｛ 返す 非公開（）； ｝\
         関数 非公開（）ー＞整数｛ 返す ４２； ｝",
    )
    .unwrap();

    let src = format!(
        "取り込む 「{}」 として 模；整数 ｘ ＝ 模。非公開（）；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let res = typecheck_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    assert!(
        matches!(res, Err(TypeError::PrivateFunctionAccess { .. })),
        "calling private fn from outside should fail: {:?}",
        res
    );
}

#[test]
fn test_public_fn_call_from_outside_is_allowed() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("tc_public_{}.hkr", std::process::id()));
    std::fs::write(&path, "公開 関数 こんにちは（）ー＞整数｛ 返す １； ｝").unwrap();

    let src = format!(
        "取り込む 「{}」 として 模；整数 ｘ ＝ 模。こんにちは（）；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let res = typecheck_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    assert!(
        res.is_ok(),
        "public fn should be callable from outside: {:?}",
        res
    );
}

#[test]
fn test_private_fn_call_from_within_module_is_allowed() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("tc_internal_{}.hkr", std::process::id()));
    // 公開関数 calls 非公開 internally — this must not be rejected.
    std::fs::write(
        &path,
        "公開 関数 公開関数（）ー＞整数｛ 返す 非公開（）； ｝\
         関数 非公開（）ー＞整数｛ 返す ９９； ｝",
    )
    .unwrap();

    let src = format!(
        "取り込む 「{}」 として 模；整数 ｘ ＝ 模。公開関数（）；",
        path.file_name().unwrap().to_str().unwrap()
    );
    let res = typecheck_with_imports(&src, &dir);
    std::fs::remove_file(&path).unwrap();
    assert!(
        res.is_ok(),
        "internal private call should be allowed: {:?}",
        res
    );
}
